# CHANGELOG 📋

All notable changes to the OmniPack-Hybrid project will be documented in this file.

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
*For granular task history, see individual files in the `changelog/` directory.*
