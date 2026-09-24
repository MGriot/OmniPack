# OmniPack Dashboard

## Active focus
**Rebuild: one Rust engine + Tauri 2 app for Windows and Android** (branch `rebuild`).

The previous code (Python/Numba engine, C++/Qt, plain-C and Android JNI ports,
Toga/WebView app) is frozen in tag `v0-legacy` / branch `legacy/toga`. Why it
was replaced:
- 5 diverging engine copies; only the Python one fed the real UI.
- Physics used Z as "up" while scoring and the viewer used Y, so items could
  float; Level 1 had no support check; stacking loads were checked only one
  level down; `stability_factor`, `max_weight`, `CORNER_FIRST`,
  `allow_mixing`, `group_id` did nothing; `stability_score` was a constant.
- MCTS / Genetic / Rust / R-Tree / AUTO-mode tasks (007, 012, 015, 016, 038)
  were marked done, but that code had been deleted; the UI silently ran Level 2.
- Items that did not fit were dropped without being reported.

## Milestones
| # | Deliverable | Status |
|---|---|---|
| M1 | Rust core: boxes + cylinders, gravity-drop placer, exact static physics, validator, CLI, benchmarks | **done** |
| M2 | Tauri 2 app (Windows MSI + Android APK) running M1 with the 3D viewer | next |
| M3 | Rapier3D transport simulation (EN 12195-1 braking/cornering) + replay | todo |
| M4 | BRKGA / NSGA-II Pareto optimizer, beam + local search, diverse solutions | todo |
| M5 | Any-shape items: mesh import, convex decomposition, stable poses | todo |
| M6 | ONNX learned candidate ranker (on only if benchmarks improve) | todo |

## M1 baseline (release build, `omnipack bench 2`)
Greedy constructive placer with stability margin 0.1 and 50% minimum support:
BR1–BR7-style average volume utilization about 69–79% in under 0.2 s per
instance, with every plan passing independent validation. This is the
baseline M4 has to beat.

## How to verify
```
cargo test --workspace
PROPTEST_CASES=1000 cargo test -p omnipack-core --test properties
cargo run --release -p omnipack-cli -- bench 5
```
