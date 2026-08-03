# OmniPack-Hybrid 📦 ⚛️

**Cross-Platform Ultra-Performance 3D Spatial Optimization Engine**

OmniPack-Hybrid is an advanced 3D packing and space optimization system designed to scale across hardware. It features a high-performance Rust-inspired logic implemented in Python with Numba JIT acceleration, a physics-accurate stability engine, and an intelligent Monte Carlo "thinking" search.

---

## 🚀 Core Features

- **Hybrid Calculation Architecture**: 
  - **Level 1 (Deterministic)**: Optimized for mobile/web (Legacy mode).
  - **Level 2 (Accelerated)**: PC/Workstation grade using Multithreading & Numba JIT.
- **Intelligent Search**: 
  - **MCTS Engine**: Monte Carlo Tree Search mimics human foresight to find global minima for wasted space.
  - **Genetic Optimization**: Evolutionary sequence discovery for complex loadouts.
- **Real-World Physics**: 
  - **Static Stability**: Center of Gravity (CoG) enforcement prevents unrealistic tipping.
  - **Recursive Load Transfer**: 100% accurate weight distribution based on contact surface area.
  - **Stacking Constraints**: Per-item weight limits and fragile item protection.
- **Interactive 3D GUI**: Built with Babylon.js, featuring real-time previews, color pickers, and multi-scenario result toggling. Standard Y-up coordinate system for intuitive WxHxD alignment.

---

## 🛠️ Tech Stack

- **Engine**: Python 3.11+, Numba (JIT Compiler), NumPy.
- **API**: FastAPI, Uvicorn, Pydantic.
- **Environment**: Managed via `uv`.
- **Frontend**: Babylon.js (WebGL/WebGPU), HTML5/CSS3.

---

## 🏁 Quick Start

### 1. Prerequisites
Ensure you have `uv` installed:
```powershell
pip install uv
```

### 2. Initialize Environment
```powershell
uv sync
```

### 3. Launch the API Server
```powershell
uv run start.py
```
*Note: This automatically cleans up any previous instance on port 8000.*

### 4. Open the Visualizer (GUI)
To avoid CORS issues and enable full 3D performance, serve the frontend via a local web server:
```powershell
uv run python -m http.server 8080
```
Then navigate to **`http://localhost:8080/viewer.html`** in your browser.

---

## 📚 Documentation Structure (Diátaxis)

- [Tutorials](./docs/tutorials/): Step-by-step lessons for beginners.
- [How-to Guides](./docs/how-to/): Practical solutions for specific packing challenges.
- [Reference](./docs/reference/): Technical API specifications and coordinate system details.
- [Explanations](./docs/explanation/): Deep dives into the Physics Engine and MCTS logic.

---

## ⚖️ License
Proprietary / Internal Development. All rights reserved.
