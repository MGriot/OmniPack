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
| M2 | Tauri 2 app (Windows MSI + Android APK) running M1 with the 3D viewer | **done** |
| M2b | Shapes (sphere, cone, pyramid, prism, L-profile), transport physics profiles (road/rail/sea), FIFO/LIFO + load priorities, timeline stepping, legend isolation, phone layout | **done** |
| M3 | Rapier3D dynamic simulation + replay (quasi-static EN 12195-1 checks already in M2b) | todo |
| M4 | BRKGA / NSGA-II Pareto optimizer, beam + local search, diverse solutions | **partly done**: sweep + BRKGA + local search, 3 distinct plans (`omnipack-opt`); NSGA-II and block building open |
| M5 | Any-shape items: mesh import, convex decomposition, stable poses | todo |
| M6 | ONNX learned candidate ranker (on only if benchmarks improve) | **partly done** (0.6): linear learned placement score trained in-app from marked plans, plus a learning-to-rank data export; the ONNX ranker is open |

## M1 baseline (release build, `omnipack bench 2`)
Greedy constructive placer with stability margin 0.1 and 50% minimum support:
BR1–BR7-style average volume utilization about 69–79% in under 0.2 s per
instance, with every plan passing independent validation. This is the
baseline M4 has to beat.

## M4 status (2026-09-25, `omnipack bench 2 --optimize 5`)
Placer with extreme-point projection and contact/blocking tie-breaks: BR1–7
average 79.0% (was 75.9%). The search (5 s per instance) reaches 83.4%, and
every plan still validates. See `docs/research/packing-stability.md`.

## Load balance (0.5.0, 2026-10-01)
CTU Code checks (CoG ±5% L/W, at least 60% of the mass in the middle half, CoG in
the lower half), VGM, tractor + semi-trailer axle loads (EU 96/53/EC as amended by
2015/719), floor pressure against the floor rating, and the door opening. Also a
direct-lashing count (EN 12195-1) and sea cases from ship motion. The balance
checks are warnings (`balance.rs`); the door opening is a hard rule. Optional
lengthwise centring rebuilds the moved load through the placer's checks, so every
loading step stays valid.

## Integration API (0.7.0, 2026-10-02)
`crates/omnipack-api` serves REST under `/api/v1`: the ERP format with unit codes, the
full JSON, validation, jobs with callbacks, drop folders and an OpenAPI spec. It runs in
`omnipack-server` (Windows/Linux/Docker) and in the desktop app ("Local API"). See
docs/api.md.

## How to verify
```
cargo test --workspace
PROPTEST_CASES=1000 cargo test -p omnipack-core --test properties
cargo run --release -p omnipack-cli -- bench 5
cargo run --release -p omnipack-cli -- bench 2 --optimize 5
```
