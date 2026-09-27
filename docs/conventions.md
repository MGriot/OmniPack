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
| Shape | Parameters | Orientations used |
|---|---|---|
| `box` | `w, h, d` | all 6 |
| `cylinder` | `radius, length` (axis = local y) | upright, lying along X, lying along Z |
| `sphere` | `radius` | 1 |
| `cone` | `radius, height` (base down) | upright only |
| `pyramid` | `w, d, height` (base down) | upright, 90° yaw |
| `prism` | `sides` (3–64), `radius` (circumradius), `length` | upright ×2, lying on a flat side ×2 |
| `l_profile` | legs `a`, `b`, `thickness`, `length` | all 6 |

Volumes, centres of mass and render meshes come from the real geometry (parry3d),
so a cone's centre of gravity sits at a quarter of its height.

`com_offset` moves the centre of mass away from that uniform-density point, in the
item's local (unrotated) frame, mm. The app shows and edits it as coordinates from
the bottom-left-back corner of the unrotated item (`X` along W, `Y` along H, `Z` along
D); an empty field means the geometric centre. It must stay inside the bounding box.

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
- `stop_order: lifo` (default, rear-door vehicles): later stops are loaded first
  and filling runs from the back wall; the first stop ends up at the door.
- `stop_order: fifo` (side loading / drive-through): stop 1 is loaded first and
  filling runs from the door towards the back.
- `stop: 0` = stays aboard, loaded first either way. Then back-zone units, then
  floor-only units, then `priority`: `volume` (default), `mass`, `base_area`,
  `height` or `as_listed`.

`accessibility` is the share of stop-tagged units not blocked (in front or on
top) by a unit of a later stop, when unloading through the door.

## Tipping in transport
`physics.avoid_tipping` (default on, needs `check_tipping` and a transport case) makes
tipping a placement rule: a position is rejected if the unit, or anything below it,
would tip on a side that no wall or already loaded neighbour (reaching above its centre
of gravity, within `max_fill_gap`) holds. Units loaded later are not counted on. When
no position passes, the unit is placed anyway and flagged for lashing. The ★ Best search
prefers plans with fewer such units over denser ones.

## JSON
Request: `PackRequest { container, items, options }` (see
`crates/omnipack-core/src/model.rs`). Response: `PackResult` with schema tag
`omnipack.plan/1` (see `plan.rs`). Generate an example with
`omnipack gen mixed 1`.
