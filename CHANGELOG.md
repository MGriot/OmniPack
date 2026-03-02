# CHANGELOG 📋

All notable changes to the OmniPack-Hybrid project will be documented in this file.

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

---
*For granular task history, see individual files in the `changelog/` directory.*
