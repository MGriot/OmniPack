# API Reference 🛠️

The OmniPack API is a RESTful service built with FastAPI. It handles 3D spatial optimization requests and returns structured coordinate data.

## Base URL
`http://127.0.0.1:8000`

## Endpoints

### `POST /pack`
The primary optimization endpoint.

#### Request Body
```json
{
  "container": {
    "id": "Standard_Bin",
    "width": 100.0,
    "height": 50.0,
    "depth": 50.0
  },
  "items": [
    {
      "id": "Box_A",
      "width": 20.0,
      "height": 20.0,
      "depth": 20.0,
      "weight": 10.0,
      "max_stack_weight": 50.0,
      "allow_mixing": true
    }
  ],
  "mode": "level2",
  "strategy": "minimize_out",
  "iterations": 20
}
```

- **`mode`**: 
  - `level1`: Standard Python (Single-threaded).
  - `level2`: Numba JIT (Parallel Accelerated).
  - `mcts`: Monte Carlo Tree Search (Intelligent thinking).
  - `genetic`: Evolutionary sequence optimizer.
- **`strategy`**:
  - `minimize_out`: Standard overflow.
  - `optimal_balance`: Suggest custom secondary space size.
- **`iterations`**: Number of random permutations to test for the best local minimum.

#### Response
Returns a list of `suggestions`, each containing one or more containers with placed items.

---

## Coordinate System Mapping
OmniPack-Hybrid uses a right-handed coordinate system for calculation:
- **X**: Width
- **Y**: Depth (Horizontal)
- **Z**: Height (Vertical / Stacking)

**Babylon.js Mapping (Viewer)**:
- Backend **X** -> Babylon **X**
- Backend **Y** -> Babylon **Z**
- Backend **Z** -> Babylon **Y (Up)**
