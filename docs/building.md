# Building OmniPack

## Prerequisites

| For | You need |
|---|---|
| Engine and CLI | Rust 1.82+ (`rustup`) |
| Desktop app | Node.js 20+ and npm, WebView2 runtime (preinstalled on Windows 11) |
| Android APK | Android SDK with platform 34+, NDK 26+, JDK 17, and the Rust targets `aarch64-linux-android` and `x86_64-linux-android` (add `armv7-linux-androideabi` and `i686-linux-android` for older devices) |
| Linux API server from Windows | Docker (Docker Desktop with WSL 2) |

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

## API server

The API lives in `crates/omnipack-api`; `crates/omnipack-server` is the standalone
program around it. The Windows app links the same crate for its Local API.

```
cargo test -p omnipack-api                  # endpoints, ERP format, jobs + callbacks, drop folders
cargo build --release -p omnipack-server    # target/release/omnipack-server(.exe)
```

The Linux binary is built in Docker with the `Dockerfile` in the repository root
(Rust on Debian 12, so the binary needs glibc 2.35 or newer), then copied out of the
image:

```
docker build --load -t omnipack-server:<version> .
docker create --name omnipack-export omnipack-server:<version>
docker cp omnipack-export:/usr/local/bin/omnipack-server release/omnipack-server_<version>_linux-x64
docker rm omnipack-export
```

`--load` puts the image in the local image store. A buildx `docker-container` builder
needs it, and the default builder accepts it. The same image runs the server directly;
see [api.md](api.md#running-it).

## Testing the built apps

Both apps can be driven from scripts through the Chrome DevTools Protocol:

- **Windows:** start the exe with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and connect to
  `http://127.0.0.1:9222/json`.
- **Android:** use a debug APK (or any build on an emulator, whose system images are
  debuggable), find the `webview_devtools_remote_<pid>` socket with
  `adb shell cat /proc/net/unix`, then run
  `adb forward tcp:9223 localabstract:webview_devtools_remote_<pid>`.
- **Mouse and touch in the 3D view:** send trusted input (`Input.dispatchMouseEvent`
  over CDP on Windows, `adb shell input tap` / `swipe` on Android). Synthetic
  `PointerEvent`s do not get Babylon.js pointer capture, so a scripted drag turns the
  camera instead of moving the unit.
- **Local API:** enable it in **API…**, then
  `curl http://127.0.0.1:8765/api/v1/health`. The `omnipack-api` tests cover the rest
  without a running app.

## Releasing

1. Set the version in `Cargo.toml` (`[workspace.package]`), `app/package.json` and
   `app/src-tauri/tauri.conf.json`, and add the section to `CHANGELOG.md`.
2. Build everything, one build at a time (they share the `target` folder):
   `npx tauri build`, the APK, `cargo build --release -p omnipack-server`, and the Linux
   server in Docker. The APK build packs the web UI when it starts, so finish UI
   changes first.
3. Copy the files into `release/`:
   - `OmniPack_<version>_x64_en-US.msi`
   - `OmniPack_<version>_x64-setup.exe`
   - `OmniPack_<version>_android-universal.apk` (from `app-universal-release.apk`)
   - `omnipack-server_<version>_windows-x64.exe`
   - `omnipack-server_<version>_linux-x64`
4. In `release/`, run `sha256sum -b <the five files> > SHA256SUMS.txt`.
5. Commit `chore: release <version>`, tag it with `git tag -a v<version>`, then push with
   `git push origin main --follow-tags`.
6. Run `gh release create v<version> --notes-file <notes> <the five files> SHA256SUMS.txt`.
   The notes are the CHANGELOG section plus a download list.

The binaries are not committed (`.gitignore`); only `release/README.md` and
`SHA256SUMS.txt` are.
