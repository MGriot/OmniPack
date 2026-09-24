# OmniPack Dashboard

## 🚀 Active Focus
**Standalone app: Toga/Briefcase Pure-Native (Windows + Android done, macOS/Linux/iOS not started)**

Decision (supersedes the native_c/native plain-C/C++ direction for the
*standalone app* goal, though neither is deleted): rather than re-porting the
whole engine + a custom renderer to C/Kotlin/Qt from scratch, package the
**existing, unmodified** Python engine (`src/omnipack/logic.py` and friends)
and the **existing, unmodified** Babylon.js UI (`viewer.html`) via
BeeWare/Briefcase, with a small platform-specific in-process bridge replacing
FastAPI/uvicorn (no server, no port, verified with zero network calls). This
gets full feature parity and full 3D rendering essentially for free, since
nothing is reimplemented - the cost is a heavier per-platform build (embeds a
Python runtime) versus native_c's leaner-but-far-more-effort path. See tasks
057 (Windows) and 056 (Android) for the bridge mechanism on each platform.
`native_c/` stays as the low-footprint alternative if that tradeoff is ever
revisited (e.g. for a constrained embedded target).

## 📋 Backlog (Toga/Briefcase Pure-Native app, current direction for "standalone")
- [ ] macOS build (toga-cocoa) - not started
- [ ] Linux build (toga-gtk) - not started
- [ ] iOS build (toga-iOS) - not started, would need the same kind of in-process bridge investigation as Windows/Android (WKWebView's URL scheme handler, most likely)

## 📋 Backlog (native_c/, low-footprint alternative, paused)
- [ ] 055: ML Placement Model (data-gen + training pipeline + C inference, its own scoping pass)

## 📋 Backlog (native/, C++/Qt - paused, not abandoned)
- [ ] 048: Native C++/Qt Engine - Qt Quick 3D Viewer
- [ ] 049: Native C++/Qt Engine - Catalog UI
- [ ] 050: Native C++/Qt Engine - Android Target
- [ ] 051: Native C++/Qt Engine - Linux Target

## 📋 Backlog (Python app, paused, not abandoned)
- [ ] 043: Visualizer Fix - Scenario Switching Bug
- [ ] 044: Visualizer Improvement - Modular Iterations Input
- [ ] 021: PDF/CAD Report Generation
- [ ] 022: Visual 'Thinking' Progress Bar
- [ ] 023: Cloud Sync & User Profiles
- [ ] 024: Multi-Language Support (Localization)
- [ ] 042: Catalog UI - JSON Import/Export (was in progress, paused for the native rewrite)

## 🕒 Recent History
- [x] 058: viewer.html Mobile-Responsive CSS (Verified: #ui panel width now capped to viewport, full-width + 70vh media query under 480px; confirmed on Android emulator at 360px/390px - stats panel values and scenario switcher no longer clipped off-screen; desktop 420px width unchanged; both MSI and APK repackaged and redelivered)
- [x] 057: Toga/Briefcase Pure-Native App - Windows In-Process Bridge (Verified: WebView2 SetVirtualHostNameToFolderMapping + WebResourceRequested, no server, full engine + rendering, zero network calls after vendoring Babylon.js)
- [x] 056: Toga/Briefcase Pure-Native App - Android In-Process Bridge (Verified: WebViewAssetLoader for static assets + addJavascriptInterface for API calls, no server, full engine + rendering on AVD)
- [x] 054: Plain-C Engine - Android NDK + JNI + Kotlin Shell (Verified: JDK+SDK+NDK installed, android_app/ Gradle project builds APK linking native_c/core via JNI for arm64-v8a+x86_64, ran on AVD - Pack Sample button produces correct output, no crashes)
- [x] 053: Plain-C Engine - Win32 Smoke-Test App (Verified: builds, launches, packs sample via ported C engine)
- [x] 052: Plain-C Engine - Headless Core + Tests (Verified: 27 tests, 0 failed)
- [x] 047: Native C++/Qt Engine - Qt6 Toolchain + Smoke-Test App (Verified: builds and launches, packs sample via ported C++ engine)
- [x] 046: Native C++/Qt Engine - Headless Core + Tests (Verified: 27 tests, 76 assertions)
- [x] 045: Python Baseline - Fix Migration Regressions (Verified: pytest 27 passed)
- [x] 041: Catalog Integration - Preserve calculations (Verified)
- [x] 040: Catalog UI - Save/Load library (Verified)
- [x] 039: Catalog API - local JSON CRUD (Verified)
- [x] 038: Engine Best-Fit Mode (Verified)
- [x] 037: GUI Stats Integration (Verified)
- [x] 034: Cleanup - Remove Compound Parts (Verified)
