# Physics model (static, M1)

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
6. **Rolling.** A lying cylinder on a single contact line must be chocked by a
   wall or a touching neighbour on both sides, or nested between two others.
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

Not modelled yet (M3, Rapier3D): friction and sliding under braking,
acceleration and cornering, or dynamic tipping in transport.
