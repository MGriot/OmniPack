# Physics model

Every candidate position is checked in this order. The independent validator
(`crates/omnipack-core/src/validate.rs`) repeats the same checks from the
finished placement list alone, and the property tests require every plan, and
every loading prefix of it, to pass.

1. **Gravity drop.** The item is lowered from above until it touches the
   floor or another item (exact shape-cast in parry3d). Floating placements
   cannot be produced.
2. **No interpenetration.** Exact test between the real shapes (boxes and
   cylinders), not bounding boxes.
3. **Supports.** Contact points with the floor and with items loaded earlier.
   Flat-on-flat contacts use the exact overlap polygon; curved contacts
   (lying cylinders, nesting in grooves) come from parry contact manifolds.
   Walls never carry vertical load.
4. **Support area.** Flat bottoms need at least `min_support_ratio` of their
   area in contact (default 50%).
5. **Tipping.** The centre of gravity must lie inside the convex support
   polygon, at least `stability_margin × (smaller half-footprint)` from its edge.
   Off-centre masses (`com_offset`) are honoured.
6. **Rolling.** Round items (lying cylinders, spheres) resting on a line or a
   point must be wedged in by walls or neighbours on every side they could roll
   to, or (with `use_chocks`, the default) are marked `needs_chocks` in the plan.
7. **Load flow.** Each item's weight plus everything it carries is split over
   its contact points as equal, tension-free springs: the minimum-norm solution
   of force and moment balance, re-solved with any "pulling" contact released.
   This gives the lever rule for bridges and a linear pressure distribution for
   eccentric loads. Loads propagate down to the floor, and for every item below:
   - the carried mass must stay within `max_load_on_top` (fragile = 0);
   - the resultant of its own weight plus the carried load must stay inside its
     support polygon with the same margin.
8. **Container limits.** Max payload, axle loads by the lever rule between the
   two axles, and centre-of-gravity limits (lateral offset, longitudinal window,
   height).

## Transport (quasi-static, EN 12195-1 method)

For every selected case in `options.physics.transport` (presets: road
EN 12195-1, rail combined transport, rail wagon shunting, sea areas A/B/C of the
CTU Code) the finished plan is checked in four directions. Forward (braking)
means towards the front wall at `z = 0`.

- **Blocking.** An item is blocked in a direction by a wall, by the secured load
  end (`secure_load_end`: a locking bar or gate at the open face of the load), or
  by a touching neighbour that is itself blocked (chains). For tipping, the
  blocker must reach above the item's centre of gravity.
- **Sliding.** Unblocked items slide if `a > μ · c_z,min`. μ is the lowest
  friction among the item and its supports (per item, or `default_friction`).
  Required securing force = `m_column · g · (a − μ · c_z,min)`.
- **Tipping.** The item plus everything resting on it tips if
  `a · h > c_z,min · e`. Here `h` is the height of the combined centre of gravity
  above the support plane, and `e` is the distance from the load resultant to the
  support-polygon edge in that direction.
- **Dynamic stacking** (optional): load on top × `c_z,max` must stay within
  `max_load_on_top`.

The results are listed per case with the securing force needed (kN). They do not
make the plan invalid: they tell you what to lash, block or fill with dunnage.
When transport checks are enabled, the placer prefers poses that resist tipping
on their own (for example, slender items lying down) and positions that touch
walls or neighbours.

Not modelled yet: full rigid-body dynamics (Rapier3D) with friction cones and
vibration.
