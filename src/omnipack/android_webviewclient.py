"""Chaquopy static_proxy (a Python class extending a Java class - see
build_gradle_extra_content in pyproject.toml for the required staticProxy
declaration). Routes viewer.html/vendor/ through WebViewAssetLoader instead
of a direct file:// load, which WebView's sandbox blocks for paths under
the app's private data directory (net::ERR_ACCESS_DENIED)."""
from java import static_proxy, method
from android.webkit import WebResourceRequest, WebResourceResponse, WebView, WebViewClient

# Set by app.py's _setup_android_bridge before the WebView navigates.
asset_loader = None


class OmniPackWebViewClient(static_proxy(WebViewClient)):
    @method(WebResourceResponse, [WebView, WebResourceRequest])
    def shouldInterceptRequest(self, view, request):
        return asset_loader.shouldInterceptRequest(request.getUrl())
