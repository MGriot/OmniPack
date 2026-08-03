# OmniPack Dashboard

## 🚀 Active Focus
**Task 055: ML Placement Model** (data-gen + training pipeline + C inference - needs its own scoping pass)

Decision (superseding the C++/Qt direction below): reproduce the app in
**plain C** instead, with raw per-platform native UI (Win32 on Windows,
Android NDK + a thin Kotlin/Java Activity shell on Android - pure C can't
draw Android UI without going through the Java framework) plus a **real
trained ML model** for placement decisions. Work lives in `native_c/`,
side by side with both `native/` (C++/Qt) and `src/omnipack/` (Python) -
nothing has been deleted from either earlier path.

## 📋 Backlog (native_c/, current direction)
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
