# Physics Engine Explanation ⚛️

OmniPack-Hybrid uses a custom static stability solver designed to enforce realistic loading patterns. Unlike traditional packing algorithms that only check for geometry overlaps, this engine simulates the physical consequences of placement.

## 1. Center of Gravity (CoG) & Tipping Rule
For an item to be considered stable, its geometric center (CoG) must be physically supported by the layer below.
- **Rule**: The vertical projection of the CoG must fall within the boundary of at least one supporting item (or the floor).
- **Implementation**: We use a `is_point_in_rect` check against all contact surfaces directly beneath the item. If the CoG hangs over an empty gap, the position is rejected.

## 2. Recursive Load Transfer (Force Propagation)
The engine ensures that weight is never "lost" and that no intermediate items are crushed.
- **Mechanism**: Weight is propagated from the highest items down to the floor.
- **Surface Area Proportioning**: If a box is supported by multiple items, the weight is distributed proportionally to the **contact surface area**.
- **100% Transfer**: Even if a box partially overhangs a gap, 100% of its weight is transferred to its physical supports.

## 3. Stacking Constraints (Max Top Support)
Every item can define a `max_stack_weight` (kg).
- **Calculation**: For every potential placement, the engine calculates the **Total Cumulative Load** already resting on the candidate supports.
- **Validation**: `Current Load + New Item Weight <= Max Top Support`.
- **Vertical Chain Integrity**: This check is applied recursively down the entire stack. Adding a heavy item to the top of a tower will be rejected if *any* box in that tower cannot support the new total weight.

## 4. Floor-First Priority
To minimize risk and maximize contiguous space, the engine applies an extreme penalty to any placement above `Z=0`.
- **Heuristic**: `Score = (Z * 1000, Y, X)`.
- **Effect**: This forces the algorithm to exhaustively search for a floor-level gap before ever considering stacking, mimicking human organization logic.
