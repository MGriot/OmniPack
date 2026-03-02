# How-to: Using the 3D GUI 🎮

The OmniPack Pro visualizer is designed for rapid iteration and visual debugging of packing strategies.

## 0. Running the GUI
To avoid browser security restrictions (CORS) when calling the API, run a local web server:
```powershell
uv run python -m http.server 8080
```
Then open **`http://localhost:8080/viewer.html`**.

## 1. Setting Up the Space
- Enter the **Width, Height, and Depth** of your primary container at the top of the UI.
- All units should be consistent (e.g., all cm or all inches).

## 2. Managing Items
- Click **"+ Add Item Type"** to define a new box category.
- **Dimensions**: Set the box size.
- **Weight**: Enter the weight in kg.
- **Max Top**: Crucial for fragile items. Set how much weight this box can support on its top surface.
- **Allow Mixing**:
  - **Checked**: The AI can shuffle these items individually to find better spots.
  - **Unchecked**: Keeps all items of this type together in one block.
- **Color**: Click the color square to pick a custom render color.

## 3. Finding the Best Result
- Set the **Iterations** (Minima Search). Higher numbers (e.g., 50) find better utilization but take longer.
- Select an **Engine**:
  - **Level 2 Accelerated** is recommended for most PC tasks.
  - **Intelligent (MCTS)** is best for complex, diverse item lists.
- Click **"FIND BEST MINIMA"**.

## 4. Inspecting Solutions
- Once complete, a list of **Top Unique Solutions** will appear.
- Click **Option 1, 2, or 3** to switch the 3D view.
- If multiple containers were required, use the **"< Prev"** and **"Next >"** buttons at the bottom of the screen to cycle through them.
- **3D Navigation**: Left-click to rotate, Right-click to pan, Scroll to zoom.
