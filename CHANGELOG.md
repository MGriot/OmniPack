# Changelog

All notable changes to OmniPack are documented here. Version 0.2 restarts the
numbering for the Rust/Tauri rewrite; the 1.x entries further down belong to the
earlier Python prototype (tag `v0-legacy`).

## [0.8.0] - 2026-10-02

### Added
- **Italian.** The whole app (Windows and Android) is available in English and Italian.
  - On first start it follows the system language. The **EN / IT** selector at the
    right end of the toolbar switches it at any time, without losing the setup, the
    plan or the manual session, and the choice is remembered.
  - Everything is translated: menus, setup fields and hints, results, legends,
    messages, dialogs, and the text the engine sends (transport legs, vehicles,
    violations, reasons for units left out, ship-motion warnings).
  - In Italian, numbers and dates use the Italian format (0,45 and 1.200).
  - Saved files do not depend on the language. The load-list CSV keeps its English
    column names, and the API stays in English.
  - The Windows setup.exe installer is in Italian on Italian systems.
- **Italian user guide:** [docs/it/user-guide.md](docs/it/user-guide.md).

### Changed
- Small English wording fixes that came with the translation:
  - Counts are singular when there is one: "1 type", "1 warning", "1 unit".
  - Search plans are labelled with pattern names rather than internal ids.
  - The sample message names the sample ("Mixed truck load").

### Documentation
- The user guide describes the language switch and the **Avoid tipping** check.

## [0.7.1] - 2026-10-02

### Fixed
- **Manual mode:** the green or red ghost kept showing the old plan after a unit was
  placed, moved, removed or undone. On phones, which have no hover to redraw it, it sat
  on top of the unit just placed. It is now cleared on every change.
- The timeline's last line said "All items loaded" while units were still waiting
  (manual mode) or did not fit (automatic). It now says how many.

### Documentation
- Building guide: the API server, the Linux binary built in Docker, and the release
  steps.
- Screenshots of the API dialog and of the phone layout.
- Conventions: how the API formats relate to the plan JSON.

## [0.7.0] - 2026-10-02

### Added
- **Integration API** (`crates/omnipack-api`, [docs/api.md](docs/api.md)), so other
  systems (SAP and other ERP or warehouse systems) can send cargo and get placements
  back.
  - REST endpoints under `/api/v1`, with an OpenAPI 3.1 description.
  - A simple ERP format: SAP field names, SAP/ISO unit codes (`CMT`, `KGM`, `LBR`, …),
    and a flat placement list in the request's units, with loading sequence, rotation
    and securing.
  - The full OmniPack JSON for `/pack` and `/optimize`.
  - `/validate` checks plans made elsewhere.
  - CSV load lists.
  - Background jobs with progress, cancel, and callback URLs with custom headers.
  - File drop folders (JSON or CSV in, result and load list out, written atomically).
    CSV item lists may use SAP column names (`MATNR`, `LFIMG`, `LAENG`, `BRGEW`, …).
  - API keys; optional HTTPS.
- **`omnipack-server`**: the API as a standalone service for Windows and Linux, with
  flags or a JSON config, a Dockerfile, and setup notes for a Windows service (NSSM)
  and systemd.
- **Local API in the Windows app:** the **API…** dialog serves the same API while the
  app runs. It offers the port, a generated key, "allow other computers" and drop
  folders, and uses the app's learned model.

### Fixed
- **Drag & drop in manual mode** never dropped the unit with a real mouse or touch (the
  release was swallowed). Units now move. The view still does not turn while dragging,
  a slight jitter on a tap is not taken as a drag, and a unit let go outside the
  container lands at the last spot it was dragged over.
- **Phones:**
  - Auto | Manual and Solutions come first in the toolbar.
  - Messages show on their own line above the tabs.
  - The 3D view is framed correctly when it was prepared while hidden (manual mode).
  - The help texts talk about tapping, not keys, also on phones whose browser reports
    a mouse-like pointer.
  - Saving and opening files says "Saved." / "Opened." instead of showing an Android
    `content://` address.
- An empty plan showed "-0" for its mass and centre of gravity.

## [0.6.0] - 2026-10-02

### Added
- **Manual placement mode** (Auto | Manual in the toolbar).
  - Pick a unit and an allowed orientation, then click in the 3D view: the unit drops
    under gravity. Drag placed units to move them.
  - Keys: R rotates, the arrows nudge, Delete removes, Ctrl+Z undoes. The selected
    unit has exact X/Y/Z fields.
  - A live ghost shows the landing position, green or red with the reason.
  - Snapping to walls and faces; gravity can be switched off.
  - Nothing is refused: the plan is checked like an automatic one and every problem
    is flagged.
  - **Auto-fill the rest** packs the remaining units around yours
    (`pack_with_fixed`). Nothing is stacked on a unit that fails the checks.
- **Saved solutions:** "Save solution…" in both modes keeps the plan with its setup in
  the app. The **Solutions…** dialog lists, opens, deletes and marks them for
  training. Only valid plans can be used for training.
- **Learned placement** (`learn.rs`, [docs/learning.md](docs/learning.md)).
  - Marked plans are replayed decision by decision against the placer's own
    candidate positions.
  - A linear score over 12 placement features is trained with a pairwise loss, in
    the app, in seconds. It starts from the built-in pattern that explains the
    decisions best and is never worse than it.
  - It adds the fill pattern "Learned", which ★ Best also tries.
  - The decisions export as JSON lines for other models.
  - CLI: `omnipack train`, `omnipack export-training`, and `--ranker` for `pack` and
    `optimize`.
- **Setup panel:**
  - tabs (Container, Cargo, Strategy, Physics, and Place in manual mode);
  - foldable sections with one-line summaries, with their open state remembered;
  - a compact cargo list (click a row for the full card, ⧉ to duplicate, a filter for
    long lists);
  - a resizable, wider panel.

### Changed
- `ContainerState::place_at` records "may tip in transport" exactly like the search.
  Rebuilt plans (lengthwise centring, training replay) are therefore judged like the
  original.
- The optimizer's fill-pattern gene covers the learned pattern when the request has a
  ranker. Default plans are unchanged: BR1–7 utilization is identical to 0.5.0.

## [0.5.0] - 2026-10-01

Physics and load balance from the CTU Code, SOLAS, ISO 1496-1, EN 12195-1 and the EU
weights directive.

### Added
- **Load balance report** (`ContainerPlan.balance`). These are warnings: a plan stays
  valid.
  - **CTU Code checks:** the cargo's centre of gravity within ±5% of the length and
    the width from the middle, at least 60% of the mass in the middle half of the
    length (25–75%), and the centre of gravity below half the height. All three
    limits are configurable.
  - Also reported: the mass in each half, and the free length to the front wall and
    the door.
- **Lengthwise centring** (option, off by default): slides a partial load by the
  shortest distance that meets the centre-of-gravity window and the axle limits. The
  moved load is rebuilt through the placer's checks, so every loading step stays
  verified.
- **VGM** (SOLAS VI/2): container tare + cargo.
- **Road vehicle axle loads:** a tractor + semi-trailer model, split by moments into
  steer axle, drive axle, trailer axle group and gross mass. It is checked against
  the limits of Directive 96/53/EC as amended by 2015/719 (10 t, 11.5 t, 24 t,
  44 t). Two presets.
- **Floor pressure** per unit (ISO 1496-1). Above the floor rating, the area its
  load must be spread over with beams is reported. The app has a "Floor pressure"
  colour mode.
- **Door opening:** units must pass the door as loaded. This is a hard rule:
  too-tall units are laid down, or left out as `door_too_small`.
- **Container presets:** 20 ft, 40 ft and 40 ft high cube (inside size, door, tare,
  payload, floor rating), and a 13.6 m semi-trailer.
- **Direct lashing count** (EN 12195-1). Each sliding or tipping issue also gives
  the number of lashings, from the friction factor 0.75, the angles, and the weaker
  of strap (2000 daN) and lashing point (1000 daN).
- **Sea case from ship motion:**
  - the roll period from beam and GM (`T = 2cB/√GM`);
  - accelerations from roll, pitch, surge and heave at the stowage height and
    position;
  - warnings for stiff or tender ships.
- **App:**
  - a balance section with ✓ / ⚠ per check, and the CoG window and quarter lines on
    the 3D floor;
  - EPAL 1 / EPAL 2 pallet items, and a "Metal on wooden floor" friction preset;
  - lashings and floor pressure in the CSV load list;
  - gaps of 150 mm or more marked "airbag".
- **★ Best search:** balance excess and units over the floor rating now cost value
  (`Objective.balance`). The score reports `balance_issues` and `floor_overloads`.

### Changed
- Road preset: the vertical factor for dynamic stacking is 1.3 (±0.3 g road
  vibration). Friction still uses 1.0, as in EN 12195-1.
- The validator and the transport report share one analysis per plan, which makes
  the search faster.

### Fixed
- A rare crash in parry3d's contact solver (seen with octagonal prisms) is now
  caught. The query then takes the conservative answer.
- Phone layout: two-column settings no longer overflow the screen width.
- The search summary no longer shows "dunnage -0.00 m".

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
