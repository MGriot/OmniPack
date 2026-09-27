# Changelog

All notable changes to OmniPack are documented here. Version 0.2 restarts the
numbering for the Rust/Tauri rewrite; the 1.x entries further down belong to the
earlier Python prototype (tag `v0-legacy`).

## [0.4.0] - 2026-09-27

### Fixed
- **Units that tip in transport:** tipping was only a score penalty, so the placer
  (and above all the ★ Best search) happily left slender or top-heavy units standing
  where they would tip. With `physics.avoid_tipping` (on by default) a position is
  rejected if the unit, or the column under it, would tip in a selected transport case
  on a side no wall or neighbour holds. A unit that fits no other way is still loaded
  and reported as needing lashing. The search ranks plans by units that may tip right
  after the container count. Sample loads: 24 → 2 (mixed), 26 → 4 (all shapes) with the
  plain placer, 0 after a 10 s search.

### Added
- **3D preview in every item card:** drag to rotate; shows the bounding box, the X/Y/Z
  axes and the centre of mass.
- **Centre of mass input:** X/Y/Z in mm from the item's base corner (stored as
  `com_offset`). A centre of mass outside the item's bounding box is rejected.

## [0.3.0] - 2026-09-25

### Fixed
- **"Everything needs securing":** blocking chains required faces within 0.05 mm, so
  any real gap broke them and practically every unit was reported as unsecured. Gaps up
  to a configurable size (default 50 mm) now count as filled with dunnage, and the
  gaps the plan relies on are listed.

### Added
- **Securing classes per unit:** secured, held once gaps are filled, chocks, needs
  lashing, stack overloaded. The legend lets you isolate each class.
- **Impact heatmap:** the transport force each unit must pass on to its blocker,
  including what the units behind push into it.
- **Friction options:** EN 12195-1 friction presets, a floor friction value, and
  anti-slip mats.
- **★ Best fill mode (`omnipack-opt`):** a time-boxed search.
  - It sweeps every pattern × priority, runs a BRKGA over loading orders and
    orientations, then a local search.
  - It returns up to three distinct plans. You can stop it early and keep the best
    found so far.
  - Also available on the command line as `omnipack optimize`.
- **Placer improvements:** extreme-point projection onto neighbouring items, plus
  contact-area, blocking, dead-gap and flat-top tie-breaks. BR1–7 average utilization
  rose from 75.9% to 79.0%.
- **Docs:** research notes in `docs/research/packing-stability.md`.
- **Bench:** `omnipack bench` reports the units needing lashing and the dunnage.
  `--optimize <s>` compares against the search.

### Removed
- Legacy leftovers from the repository root. The history is kept in tag `v0-legacy`.

## [0.2.0] - 2026-09-25

The complete rewrite: one Rust engine for Windows and Android, with exact geometry and
physics.

### Added
- **Engine (Rust):**
  - Exact collision and contact geometry (parry3d).
  - Placement by lowering items from above, so nothing can float.
  - Stability at rest: centre of gravity over the support polygon with a margin, and
    elastic load distribution followed down every stack.
  - Load limits and fragile items, rolling and chocks, payload, axle and
    centre-of-gravity limits.
  - An independent validator that re-checks every plan, and every step of its loading
    sequence.
- **Shapes:** box, cylinder, sphere, cone, pyramid, n-sided prism and L-profile, with
  volumes and centres of gravity from the true geometry.
- **Transport physics (EN 12195-1 / CTU Code):**
  - Road, rail (combined transport and shunting) and sea areas A/B/C.
  - Sliding, tipping and dynamic stacking checks, with blocking chains and a secured
    load end.
  - A report of the securing force each item needs.
  - When transport checks are on, placement prefers poses that won't tip and positions
    that touch walls or neighbours.
- **Loading strategies:**
  - LIFO and FIFO unloading order.
  - Load priority: volume, mass, base area, height, or as listed.
  - Five fill patterns and a balance weight.
- **App (Tauri 2) for Windows and Android:**
  - Setup editors, samples, a catalog, JSON open/save, and plan and CSV export.
  - Babylon.js 3D view drawing exactly the engine's geometry.
  - Colour modes, including "securing needed".
  - Legend isolation.
  - Timeline with first / previous / play / next / last buttons, keyboard shortcuts,
    and a preview of the next item.
  - Phone layout with bottom tabs.
- **CLI:** `omnipack pack | gen | thpack | bench`.
- **Documentation:** user guide, physics model, conventions, building.

### Changed
- License: Apache-2.0.
- Windows installer: 4.8 MB, down from 32 MB. Android APK: 27 MB, with no embedded
  Python.

### Removed
- The Python/Numba engine, and the C++/Qt, plain-C and JNI ports. These are kept in
  tag `v0-legacy`.

### Fixed (compared with the prototype)
- The axis mix-up (Z-up physics shown in a Y-up viewer) that let items float.
- Stack loads were checked only one level down.
- The stability and weight options did nothing.
- The UI offered MCTS and genetic modes whose code no longer existed.
- Units that did not fit disappeared without being reported.

---

## Legacy prototype (Python)

## [1.2.0] - 2026-03-07

### Added
- **Advanced Filling Biases**: Implemented **Wall Building** (vertical depth layers) and **Corner First** (4-corner perimeter) strategies for specialized logistics requirements.
- **Balance-Aware Scoring**: Enhanced all calculation cores to prioritize low and centered Centers of Mass, significantly improving container stability for transport.
- **Comprehensive Validation**: Created `comprehensive_test.py` to benchmark all engines, verify all 5 filling biases, and confirm collision-free packing.
- **Symmetrical Shape Handling**: Added automatic dimension synchronization in the UI catalog for symmetrical shapes (Spheres and Tetrahedrons).
- **Favicon Fix**: Added a 204 No Content response for `/favicon.ico` in `main.py` to reduce log noise.

### Fixed
- **GUI Dimension Mapping**: Fixed a critical bug in `viewer.html` where Width, Height, and Depth were incorrectly swapped during API requests.
- **Optimizer Integration**: Fixed a critical bug in `MultiContainerEngine` where results from Genetic and MCTS optimizers were not correctly assigned to containers, leading to 0% utilization.
- **Accessibility Metric**: Corrected the accessibility penalty multiplier in `core/models.py` to properly reflect the 0-100% scale.
- **Refactor Stability**: Restored the entire test suite (`tests/`) to 100% pass rate following the removal of compound parts.
- **Floor-First Bias**: Refined the scoring logic to strictly prioritize surface area saturation before stacking.
- **UI Visibility**: Enhanced 3D container rendering in `viewer.html` by enabling edge rendering for better spatial context.

### Changed
- **Rust Hello Message**: Updated core verification to include R-Tree support status.

## [1.1.0] - 2026-03-04

### Added
- **Vision Update**: Redefined the product strategy to include Rust-accelerated core, 3D R-Tree spatial indexing, and advanced logistics constraints (FIFO/LIFO).
- **Logistics Constraints**: Added theoretical support for directional Z-packing (back-to-front/front-to-back) and accessibility scoring.
- **Portability Roadmap**: Identified Capacitor and BeeWare as primary targets for Mobile/Desktop packaging.
- **Task Roadmap**: Updated `tasks.json` with the new Ultra-Performance development sequence (Tasks 015-020).

## [1.0.0] - 2026-03-01

### Added
- **Core Models**: Item, Container, and Rotation classes with 3D math logic.
- **Hybrid Calculation Engine**: Support for Level 1 (Legacy) and Level 2 (Accelerated).
- **Intelligent Search**: Monte Carlo Tree Search (MCTS) and Genetic sequence optimization.
- **Physics Engine**: Static stability (Center of Gravity) and recursive force transfer logic.
- **Multi-Container Logic**: Automatic overflow handling and custom space suggestions.
- **Web API**: FastAPI endpoints for remote calculation and multi-minima search.
- **3D Visualizer**: Babylon.js GUI with live previews, color pickers, and scenario selectors.

### Fixed
- **API Connectivity**: Resolved "API unreachable" error by standardizing on IPv4 (127.0.0.1) and improving host detection in the visualizer.
- **3D Coordinate Alignment**: Fixed WxHxD mapping to use standard Y-up orientation, ensuring height (H) correctly represents the vertical axis.
- **Preview Scaling**: Implemented dynamic normalization for item previews, fixing the issue where objects appeared too small.
- **Engine Integrity**: Restored visibility and verified all 4 optimization engines (Level 1, Level 2, Genetic, MCTS).
- Improved floor-first priority logic to prevent unnecessary vertical stacking.

### Changed
- **Optimization Priority**: Replaced binary Stability toggle with a proportional **Stability/Density Slider** (`stability_factor`). This allows granular control over height penalties and space utilization.
- **Interactive Previews**: Item previews in the sidebar now support full 3D rotation and zoom controls.
- **Single Container Optimization**: Refined `MultiContainerEngine` to prioritize filling the primary container and suppress empty overflow suggestions.

---
*Pre-rebuild history lives in tag `v0-legacy`.*
