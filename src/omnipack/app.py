import threading
import uvicorn
import toga
from toga.style import Pack
from toga.style.pack import COLUMN, ROW
import time
import socket

def is_port_in_use(port):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(('127.0.0.1', port)) == 0

class OmniPackApp(toga.App):
    def startup(self):
        # 1. Create Main Window
        self.main_window = toga.MainWindow(title=self.formal_name)
        
        # 2. Start Backend in background thread
        self.backend_thread = threading.Thread(target=self.run_backend, daemon=True)
        self.backend_thread.start()
        
        # 3. Create WebView
        # We wait a bit for the backend to start, or we can just point it
        self.webview = toga.WebView(
            url="http://127.0.0.1:8000/",
            style=Pack(flex=1)
        )
        
        self.main_window.content = self.webview
        self.main_window.show()

    def run_backend(self):
        from main import app
        # Standard Uvicorn run
        uvicorn.run(app, host="127.0.0.1", port=8000, log_level="info")

def main():
    return OmniPackApp("OmniPack", "com.omnipack.app")

if __name__ == "__main__":
    app = main()
    app.main_loop()
