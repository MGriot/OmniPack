# OmniPack

A 3D loading planner for containers and trucks. You give it a container and a
list of items (boxes and cylinders, with mass, stacking limits, fragility,
"this side up" and delivery stops). It returns a loading plan that is physically
valid: nothing floats, overlaps, tips, rolls or gets crushed, and payload, axle
and centre-of-gravity limits are respected. Every plan is re-checked by an
independent validator.

> **Status: rebuild in progress** (branch `rebuild`). Milestone M1, the Rust
> engine and CLI, is done. The Tauri app for Windows and Android (M2), the
> transport simulation (M3), the optimizer (M4), arbitrary meshes (M5) and the
> learned ranker (M6) come next; see `conductor/stage.md`. The previous Python,
> C, C++ and Toga code is kept in tag `v0-legacy`.

## Layout
```
crates/omnipack-geom   shapes, orientations, exact drop / overlap / contact queries (parry3d)
crates/omnipack-core   model, placer, static physics, validator, metrics, generators
crates/omnipack-cli    `omnipack` command-line tool
docs/                  conventions.md (axes, units, biases, JSON), physics-model.md
```

## Quick start
Requires Rust 1.82+.
```
cargo test --workspace
cargo run --release -p omnipack-cli -- gen mixed 1 -o out/mixed.json
cargo run --release -p omnipack-cli -- pack out/mixed.json -o out/plan.json
cargo run --release -p omnipack-cli -- bench 5
```
To use the official Bischoff & Ratcliff instances, download `thpack1.txt` …
`thpack7.txt` from the OR-Library and run
`omnipack thpack thpack1.txt 1 -o br1-1.json`.

## License
Proprietary / internal development. All rights reserved.
