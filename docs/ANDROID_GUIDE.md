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
This creates the `build/omnipack/android/gradle` Gradle project structure.
```powershell
uv run briefcase create android
```
*Note: The first time you run this, it may download the Android SDK and tools. Accept licenses if prompted.*

**Required manual step** (`create`/`update` only manage Python sources, not
custom Java - see Architecture Overview below): copy the hand-written JS
bridge class into the generated project before building:
```powershell
Copy-Item android\java\com\omnipack\omnipack\OmniPackBridge.java `
  build\omnipack\android\gradle\app\src\main\java\com\omnipack\omnipack\OmniPackBridge.java
```

### 3. Build the APK
This compiles the Python code and bundles it into an APK. If you change any
Python source after `create`, run `briefcase update android` first (`build`
alone does not re-sync `src/omnipack`).
```powershell
uv run briefcase update android
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

The Android app runs with no embedded HTTP server at all - verified end-to-end
(engine options, catalog, and Babylon.js 3D rendering) on an emulator. Two
separate mechanisms are needed, because Android's WebView has no single
equivalent of Windows' `WebResourceRequested` + virtual-host-folder-mapping:

1.  **Static assets** (`viewer.html`, `vendor/babylon.js`): Toga-android's
    `toga.WebView` doesn't support loading local files directly - WebView's
    sandbox blocks `file://` URLs into Chaquopy's app-private extracted
    files with `net::ERR_ACCESS_DENIED`. Instead, `app.py`'s
    `_setup_android_bridge` installs a custom `WebViewClient`
    (`src/omnipack/android_webviewclient.py` - a **Chaquopy `static_proxy`**,
    i.e. a Python class that actually subclasses
    `android.webkit.WebViewClient`, declared via `staticProxy` in
    `pyproject.toml`'s `build_gradle_extra_content`) that serves them through
    `androidx.webkit.WebViewAssetLoader`, reachable at
    `https://appassets.androidplatform.net/assets/...`.
2.  **API calls** (`/pack`, `/catalog`): Android's `WebResourceRequest` has no
    way to read a POST body, so intercepting `fetch()` (the Windows approach)
    doesn't work here. Instead `viewer.html`'s `api()` function detects
    `window.OmniPackAndroid` and calls it directly:
    `window.OmniPackAndroid.call(path, method, bodyJson)` - a synchronous
    JS→Java call via `addJavascriptInterface`, reaching a **hand-written Java
    class** (`android/java/com/omnipack/omnipack/OmniPackBridge.java`) that
    calls straight into `omnipack.bridge.dispatch_json` (shared with the
    Windows bridge's `dispatch_sync`) via Chaquopy's `Python.getInstance()`.

    **Important**: `briefcase create android`/`update android` only manage
    Python sources - there is no supported Briefcase mechanism for custom
    Java files. If the `build/omnipack/android/gradle` project is ever
    regenerated (`briefcase create android`), you must manually copy
    `android/java/com/omnipack/omnipack/OmniPackBridge.java` back to
    `build/omnipack/android/gradle/app/src/main/java/com/omnipack/omnipack/`
    before building.
3.  **Data Persistence**: `catalog.json` is stored in the App's private data storage (mapped via `OMNIPACK_DATA_DIR`, set from `self.paths.data`).

## ⚠️ Known Limitations on Android

-   **Numba/JIT**: Confirmed on-device - `core.accelerated` can't import `numba` on the emulator's ABI, so the engine falls back to **pure Python mode** (logged as `WARNING:root:Numba not found. Falling back to pure Python (slower).`). Stable, but slower than the desktop build for high iteration counts.
-   **Layout is not mobile-responsive**: `viewer.html`'s CSS was written for desktop widths. On a phone-width viewport the stats panel and 3D canvas are pushed off the right edge of the screen (values do come back correctly - e.g. Center of Mass renders - they're just visually clipped). Not yet fixed; would need responsive CSS work in `viewer.html`, unrelated to the native bridge itself.

## 🛠️ Troubleshooting

-   **Crash on startup**: Connect your device (or emulator) via `adb` and run `adb logcat -s "python.stdout:*" "python.stderr:*"` to see the Python stack trace.
-   **`net::ERR_ACCESS_DENIED` loading viewer.html**: means `_setup_android_bridge`'s `WebViewAssetLoader` setup isn't in effect - check the `OmniPackWebViewClient` static_proxy actually built (`staticProxy "omnipack.android_webviewclient"` must be present in the generated `app/build.gradle`'s `python {}` block).
-   **"API unreachable" / calculate packing does nothing**: check `window.OmniPackAndroid` is defined in the WebView console (`addJavascriptInterface` didn't register) - usually means `OmniPackBridge.java` wasn't present when the APK was built (see the required manual copy step above).
