# 🤖 Building OmniPack for Android

This guide explains how to package OmniPack-Hybrid as a standalone Android APK using **BeeWare (Briefcase)**.

## 📋 Prerequisites

1.  **JDK (Java Development Kit)**: Version 17 or higher.
2.  **Android SDK**: (Briefcase handles this automatically, but having Android Studio installed helps).
3.  **Python 3.11+**: Ensure it's in your PATH.
4.  **Briefcase**: Installed in your environment.

## 🚀 Quick Build Steps

### 1. Install Dependencies
Ensure you have the development dependencies installed:
```powershell
uv sync --group dev
```

### 2. Initialize Android Project
This creates the `android` directory with the Gradle project structure.
```powershell
uv run briefcase create android
```
*Note: The first time you run this, it may download the Android SDK and tools. Accept licenses if prompted.*

### 3. Build the APK
This compiles the Python code and bundles it into an APK.
```powershell
uv run briefcase build android
```

### 4. Run on Emulator/Device
To test the app:
```powershell
uv run briefcase run android
```
*   Select a connected device or an emulator from the list.
*   **Important**: The first launch might take a moment as it extracts the Python runtime.

### 5. Locate the APK
After a successful build, the APK file will be located in:
`build/omnipack/android/gradle/app/build/outputs/apk/debug/app-debug.apk`

---

## 🔧 Architecture Overview

The Android app runs in a **Hybrid Mode** with no embedded HTTP server at all:
1.  **Native Shell (Toga)**: `src/omnipack/app.py` starts the Android app and creates a native `toga.WebView`.
2.  **Bundled UI**: `viewer.html` is read from the app bundle and loaded directly into the WebView via `set_content()` (no `127.0.0.1`, no port, no network traffic).
3.  **Native Bridge**: The WebView's JS calls `window.webkit`/`on_message` to send `{id, path, method, body}` messages; `app.py`'s `handle_webview_message`/`process_message` dispatch them straight to `omnipack.logic` (`run_pack_logic`, `get_catalog_data`, `save_to_catalog_data`, `delete_from_catalog_data`) in-process, then push the response back into the page by evaluating JS (`window.onBridgeResponse(...)`).
4.  **Data Persistence**: `catalog.json` is stored in the App's private data storage (mapped via `OMNIPACK_DATA_DIR`, set from `self.paths.data`).

## ⚠️ Known Limitations on Android

-   **Numba/JIT**: The high-performance `core.accelerated` module attempts to import `numba`. If Numba is not supported on the target architecture (likely on standard Android wheels), the engine automatically falls back to **pure Python mode**. This ensures stability but reduces packing speed.
-   **Performance**: Complex optimizations (Genetic/MCTS) with high iteration counts may be slower on mobile devices compared to Desktop.

## 🛠️ Troubleshooting

-   **"Address already in use"**: The app automatically finds a free port (`0`), so this shouldn't happen.
-   **Crash on startup**: Connect your device via USB and run `adb logcat | findstr python` to see the Python stack trace.
