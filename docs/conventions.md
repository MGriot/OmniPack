# Conventions

## Axes and units
- **Y is up.** X is the container width (left → right), Z is the depth.
  The back wall is at `z = 0`, the **door at `z = depth`**.
- Lengths are **mm**, masses **kg**. Loads are reported in kg-force.
- A placement's `position` is the **minimum corner of its world-space AABB**;
  `size` is the AABB size. The 3D viewer maps these directly to Babylon's Y-up frame.
- Tolerances live in one place: `crates/omnipack-geom/src/tol.rs`
  (contact 0.05 mm, penetration 0.01 mm).

## Shapes and orientations
- `box {w,h,d}`: local extents along local x, y, z.
- `cylinder {radius,length}`: axis along local y.
- An orientation says which local extent lies along world X, Y, Z,
  e.g. `WHD` = unrotated, `DHW` = 90° yaw, `HWD` = local height lying along X.
  `upright_only` keeps local Y vertical (`WHD`, `DHW`).

## Fill biases (`options.bias`)
Each bias is a strict priority of the three axes (normalised positions):

| Bias | Priority | Result |
|---|---|---|
| `wall_building` (default) | z ≫ y ≫ x | vertical walls across the width, back to door |
| `floor_first` | y ≫ z ≫ x | full floor layers, rows across the width from the back |
| `longitudinal` | x ≫ y ≫ z | vertical walls along the length, left to right |
| `lateral` | y ≫ x ≫ z | floor layers, rows along the length from the left |
| `corner_first` | x + y + z | grows out of the back-left-bottom corner |

`balance_weight` (0..1) adds a penalty for moving the load's centre of gravity
off the centreline. Items with `zone: front` prefer the door, `back` the far wall.

## Loading order
Units are loaded later-stop first (`stop: 0` = stays aboard, loaded first),
then back-zone, then floor-only, then larger and heavier first. `accessibility`
is the share of stop-tagged units not blocked (in front or on top) by a unit of
a later stop.

## JSON
Request: `PackRequest { container, items, options }` (see
`crates/omnipack-core/src/model.rs`). Response: `PackResult` with schema tag
`omnipack.plan/1` (see `plan.rs`). Generate an example with
`omnipack gen mixed 1`.
