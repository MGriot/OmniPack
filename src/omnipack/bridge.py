"""Shared in-process request dispatch for the native WebView bridges
(Windows: app.py's WebResourceRequested handler; Android: android_bridge.py's
JavascriptInterface). Routes the same paths backend.py exposes over HTTP,
but calls straight into logic.py - no server, no port."""
import json
import traceback
from urllib.parse import unquote


def dispatch_sync(path, method, body):
    """Runs the (nominally async, but never actually suspending) logic
    coroutines synchronously - avoids nesting an event loop inside a
    platform WebView callback."""
    from .logic import run_pack_logic, get_catalog_data, save_to_catalog_data, delete_from_catalog_data

    if path == "/pack" and method == "POST":
        coro = run_pack_logic(body or {})
    elif path == "/catalog" and method == "GET":
        coro = get_catalog_data()
    elif path == "/catalog" and method == "POST":
        coro = save_to_catalog_data(body or {})
    elif path.startswith("/catalog/") and method == "DELETE":
        coro = delete_from_catalog_data(unquote(path[len("/catalog/"):]))
    else:
        raise ValueError(f"No route for {method} {path}")

    try:
        coro.send(None)
    except StopIteration as e:
        return e.value
    raise RuntimeError(f"{path} handler awaited on something - can't run synchronously")


def dispatch_json(path, method, body_str):
    """String in/out wrapper for bridges that can only pass strings across
    the JS<->native boundary (e.g. Android's addJavascriptInterface, which
    has no concept of an HTTP request/response)."""
    try:
        body = json.loads(body_str) if body_str else None
        result = dispatch_sync(path, method, body)
        return json.dumps({"ok": True, "data": result})
    except Exception as e:
        traceback.print_exc()
        return json.dumps({"ok": False, "error": str(e)})
