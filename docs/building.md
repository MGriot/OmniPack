# Building OmniPack

## Prerequisites

| For | You need |
|---|---|
| Engine and CLI | Rust 1.82+ (`rustup`) |
| Desktop app | Node.js 20+ and npm, WebView2 runtime (preinstalled on Windows 11) |
| Android APK | Android SDK with platform 34+, NDK 26+, JDK 17, and the Rust targets `aarch64-linux-android` and `x86_64-linux-android` (add `armv7-linux-androideabi` and `i686-linux-android` for older devices) |

## Engine, tests and CLI

```
cargo test --workspace
PROPTEST_CASES=1000 cargo test -p omnipack-core --test properties   # stress the physics invariants
cargo run --release -p omnipack-cli -- bench 5                      # fill-rate benchmark
cargo run --release -p omnipack-cli -- gen shapes 1 -o out/shapes.json
cargo run --release -p omnipack-cli -- pack out/shapes.json -o out/plan.json
```

`omnipack thpack thpackN.txt <problem>` imports the Bischoff & Ratcliff instances from
the OR-Library.

## Windows installers

```
cd app
npm install
npx tauri build
```

The output is written to `target/release/bundle/`:
- `msi/OmniPack_<version>_x64_en-US.msi`
- `nsis/OmniPack_<version>_x64-setup.exe`

The first run downloads the WiX and NSIS toolsets. For development, use `npx tauri dev`.

## Android APK

Set the environment first. In PowerShell, adjust the paths to your installation:

```
$env:ANDROID_HOME = "C:\Android\sdk"
$env:NDK_HOME     = "C:\Android\sdk\ndk\26.3.11579264"
$env:JAVA_HOME    = "C:\Program Files\Microsoft\jdk-17.0.20.101-hotspot"
cd app
npx tauri android build --apk --target aarch64 --target x86_64
```

The APK is written to
`app/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`.
For an emulator or device with USB debugging, use `npx tauri android dev`.

R8 minification is deliberately switched off in the release build type, because it
stripped classes the WebView bridge needs and the app showed a blank screen.

### Android signing

The release build is signed through `app/src-tauri/gen/android/app/build.gradle.kts`:

- **With your own key.** Create `app/src-tauri/gen/android/keystore.properties`. It is
  git-ignored, so never commit it:
  ```
  storeFile=C:/path/to/omnipack-upload.jks
  storePassword=…
  keyAlias=omnipack
  keyPassword=…
  ```
  You can create a key with
  `keytool -genkeypair -v -keystore omnipack-upload.jks -keyalg RSA -keysize 2048 -validity 10000 -alias omnipack`.
  You need your own key for the Play Store and for updates signed by you.
- **Without it,** the standard Android debug key is used. This is fine for sideloading
  and testing, but users cannot later update to a build signed with a different key
  without uninstalling first.

## Testing the built apps

Both apps can be driven from scripts through the Chrome DevTools Protocol:

- **Windows:** start the exe with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and connect to
  `http://127.0.0.1:9222/json`.
- **Android:** use a debug APK, find the `webview_devtools_remote_<pid>` socket with
  `adb shell cat /proc/net/unix`, then run
  `adb forward tcp:9223 localabstract:webview_devtools_remote_<pid>`.
