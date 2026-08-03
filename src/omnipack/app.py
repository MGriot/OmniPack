import toga
from toga.style import Pack
import json
import os
import sys
import traceback
from urllib.parse import urlparse

VIRTUAL_HOST = "omnipack.local"
API_HOST = "omnipack.api"  # deliberately NOT folder-mapped, so WebResourceRequested
                            # actually sees these requests (SetVirtualHostNameToFolderMapping
                            # shadows same-host requests before the event fires)


class OmniPackApp(toga.App):
    def startup(self):
        print("OmniPack: Starting Pure-Native Mode (No Ports)")

        try:
            # 1. Setup Data Directory
            if not os.path.exists(self.paths.data):
                try: os.makedirs(self.paths.data, exist_ok=True)
                except: pass
            os.environ["OMNIPACK_DATA_DIR"] = str(self.paths.data)

            # 2. Locate the app bundle root (viewer.html lives here)
            app_root = self.paths.app
            viewer_path = app_root / "viewer.html"
            if not viewer_path.exists():
                # Emergency recursive search in bundle
                for root, dirs, files in os.walk(self.paths.app):
                    if "viewer.html" in files:
                        app_root = type(self.paths.app)(root)
                        viewer_path = app_root / "viewer.html"
                        break

            # 3. Create Main Window + WebView
            self.main_window = toga.MainWindow(title=self.formal_name)
            self.webview = toga.WebView(style=Pack(flex=1))
            self.main_window.content = self.webview

            if not viewer_path.exists():
                self.webview.set_content(
                    "http://omnipack.local/",
                    "<html><body><h1>Fatal Error</h1><p>UI resources missing from APK.</p></body></html>",
                )
                self.main_window.show()
                return

            # 4. Wire up the platform-native in-process bridge (no server, no
            # port) so viewer.html's existing fetch("/pack") etc. calls are
            # answered locally instead of hitting the network.
            if sys.platform == "win32":
                self._setup_winforms_bridge(app_root, viewer_path)
            else:
                # No in-process bridge implemented for this backend yet -
                # fall back to loading the file directly (relative fetch()
                # calls will fail with "API unreachable" until a bridge is
                # added for this platform).
                self.webview.set_url(viewer_path.as_uri())

            self.main_window.show()

        except Exception as e:
            print(f"OmniPack: Startup Error: {e}")
            traceback.print_exc()

    def _setup_winforms_bridge(self, app_root, viewer_path):
        import clr  # noqa: F401  (side effect: loads pythonnet's CLR bootstrapper)
        from Microsoft.Web.WebView2.Core import (
            CoreWebView2HostResourceAccessKind,
            CoreWebView2WebResourceContext,
        )
        from System import Uri
        from System.IO import MemoryStream

        impl = self.webview._impl
        native = impl.native

        def when_ready(task=None):
            core = native.CoreWebView2
            core.SetVirtualHostNameToFolderMapping(
                VIRTUAL_HOST, str(app_root), CoreWebView2HostResourceAccessKind.Deny
            )

            def on_web_resource_requested(sender, args):
                try:
                    self._handle_web_resource_requested(args, MemoryStream, core)
                except Exception:
                    print(f"OmniPack: bridge request failed for {args.Request.Uri}")
                    traceback.print_exc()

            core.AddWebResourceRequestedFilter(
                f"https://{API_HOST}/*", CoreWebView2WebResourceContext.All
            )
            core.WebResourceRequested += on_web_resource_requested
            self._resource_handler_ref = on_web_resource_requested  # keep alive (avoid GC of the closure)

            native.Source = Uri(f"https://{VIRTUAL_HOST}/{viewer_path.name}")

        impl.run_after_initialization(when_ready)

    def _handle_web_resource_requested(self, args, MemoryStream, core):
        request = args.Request
        uri = str(request.Uri)
        path = urlparse(uri).path
        method = str(request.Method)

        if not (path == "/pack" or path == "/catalog" or path.startswith("/catalog/")):
            return  # not one of ours - let WebView2 handle it normally

        cors_headers = (
            "Access-Control-Allow-Origin: *\r\n"
            "Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\n"
            "Access-Control-Allow-Headers: Content-Type"
        )

        deferral = args.GetDeferral()

        if method == "OPTIONS":
            # CORS preflight - omnipack.api is a different origin than the
            # omnipack.local page, so POST/DELETE need this answered first.
            response = core.Environment.CreateWebResourceResponse(
                None, 200, "OK", cors_headers
            )
            args.Response = response
            deferral.Complete()
            return

        try:
            body = None
            if request.Content is not None:
                ms = MemoryStream()
                request.Content.CopyTo(ms)
                raw = bytes(ms.ToArray())
                if raw:
                    body = json.loads(raw.decode("utf-8"))

            result = self._dispatch_sync(path, method, body)
            status, payload = 200, result
        except Exception as e:
            traceback.print_exc()
            status, payload = 500, {"detail": str(e)}

        content_bytes = json.dumps(payload).encode("utf-8")
        response_stream = MemoryStream(content_bytes)
        response = core.Environment.CreateWebResourceResponse(
            response_stream, status, "OK" if status == 200 else "Internal Server Error",
            f"Content-Type: application/json\r\n{cors_headers}",
        )
        args.Response = response
        deferral.Complete()

    def _dispatch_sync(self, path, method, body):
        """Runs the (nominally async, but never actually suspending) logic
        coroutines synchronously - avoids nesting an event loop inside the
        WinForms/WebView2 callback, which already runs on Toga's own loop."""
        from .logic import run_pack_logic, get_catalog_data, save_to_catalog_data, delete_from_catalog_data

        if path == "/pack" and method == "POST":
            coro = run_pack_logic(body or {})
        elif path == "/catalog" and method == "GET":
            coro = get_catalog_data()
        elif path == "/catalog" and method == "POST":
            coro = save_to_catalog_data(body or {})
        elif path.startswith("/catalog/") and method == "DELETE":
            name = path[len("/catalog/"):]
            from urllib.parse import unquote
            coro = delete_from_catalog_data(unquote(name))
        else:
            raise ValueError(f"No route for {method} {path}")

        try:
            coro.send(None)
        except StopIteration as e:
            return e.value
        raise RuntimeError(f"{path} handler awaited on something - can't run synchronously")


def main():
    return OmniPackApp("OmniPack", "com.omnipack.app")
