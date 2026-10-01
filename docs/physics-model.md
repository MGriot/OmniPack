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
9. **Door opening.** With `door` set (ISO containers: 2340 × 2280 mm, high cube
   2340 × 2585 mm), a unit must pass the opening in the orientation it is loaded:
   its X extent within the door width and its Y extent within the door height.
   Orientations that do not pass are never tried; a unit with none is reported as
   `door_too_small`.

Steps 1–9 are hard rules: a plan that breaks one is invalid. Everything below is
reported, and the ★ Best search takes it into account, but it does not make a plan
invalid.

The standards use x for the length, y for the width and z for the height. OmniPack
uses Z for the length (front wall at `z = 0`, door at `z = depth`), X for the width
and Y for the height.

## Transport (quasi-static, EN 12195-1 method)

For every selected case in `options.physics.transport` (presets: road
EN 12195-1, rail combined transport, rail wagon shunting, sea areas A/B/C of the
CTU Code) the finished plan is checked in four directions. Forward (braking)
means towards the front wall at `z = 0`.

- **Blocking.** An item is blocked in a direction by a wall, by the secured load
  end (`secure_load_end`: a locking bar or gate at the open face of the load), or
  by a same-layer neighbour that is itself blocked (chains). Gaps up to
  `max_fill_gap` (default 50 mm) count as filled with dunnage or airbags; every
  gap a blocking chain relies on is listed in the report. For tipping, the
  blocker must reach above the item's centre of gravity.
- **Sliding.** Unblocked items slide if `a > μ · c_z,min`. μ is the lowest
  friction among the item, the items it rests on and, if set, `floor_friction`.
  `anti_slip_mats` raises every contact to at least 0.6.
  Required securing force = `m_column · g · (a − μ · c_z,min)`.
- **Tipping.** The item plus everything resting on it tips if
  `a · h > c_z,min · e`. Here `h` is the height of the combined centre of gravity
  above the support plane, and `e` is the distance from the load resultant to the
  support-polygon edge in that direction.
- **Dynamic stacking** (optional): load on top × `c_z,max` must stay within
  `max_load_on_top`. On the road, `c_z` is 1.0 for friction (EN 12195-1) and up to
  1.3 for stacks (±0.3 g of road vibration).

Every unit gets a **securing class**: secured, held once gaps are filled
(dunnage), chocks, lashing (a sliding or tipping force remains), or stack
overloaded. It also gets an **impact**: the force it must pass on to its
blocker in the worst case and direction. That is its own excess over friction,
`m·g·(a − μ·c_z,min)`, plus everything the units behind it push into it along
the blocking chain. The units against the front wall under braking carry the sum
of their row. The app shows it as the impact heatmap.

The results are listed per case with the securing force needed (kN). They do not
make the plan invalid: they tell you what to lash, block or fill with dunnage.

### Direct lashing (EN 12195-1)

Every sliding or tipping issue also gives the number of direct lashings that would
hold it (`physics.lashing`). One lashing counts with the weaker of its lashing
capacity LC (default 2000 daN) and the lashing point it is fixed to (default
1000 daN, the ISO 1496-1 floor points; points on the posts are 500 daN). The
lashing makes the angle α with the floor (default 45°) and β with the direction it
holds (default 30°).

- Sliding, with the dynamic friction factor `f_μ = 0.75`:

  `n · LC ≥ m · g · (c − μ · f_μ · c_z) / (μ · f_μ · sin α + cos α · cos β)`

- Tipping: the excess tipping moment over the lashing's horizontal component,
  taken at the top of the unit (`H`). This is conservative, because the vertical
  component also helps:

  `n · LC · cos α · cos β · H ≥ F_tip · h`

The blocking force (`required`, kN) is unchanged: `m · g · (c − μ · c_z)`.
When transport checks are enabled, the placer prefers poses that resist tipping
on their own (for example, slender items lying down) and positions that touch
walls or neighbours.

### Sea cases from ship motion

Besides the CTU Code sea areas, a case can be derived from the ship and the
stowage position (`TransportCase::from_ship`, "Sea case from ship motion" in the
app):

- Natural roll period: `T_r = 2 · c · B / √GM` (B beam, GM metacentric height,
  c ≈ 0.38–0.42).
- Sideways: `c_y = sin θ_r + (2π / T_r)² · z · sin θ_r / g`. The first term is the
  heeled weight; the second is the tangential acceleration at height z above the
  roll axis.
- Fore and aft: `c_x = a_surge / g + sin θ_p + (2π / T_p)² · θ_p · z / g` (pitch
  angle θ_p, pitch period T_p).
- Vertical: `c_z = 1 ± (a_heave / g + (2π / T_p)² · θ_p · |x| / g)`, where x is the
  distance from midship. Containers far forward or aft see the most.

A large GM gives a **stiff** ship: a short roll period and violent accelerations
high up on deck, which load the lashing rods. A GM below 0.15 m (the IMO Intact
Stability Code minimum) is a **tender** ship. The app flags both.

Not modelled yet: full rigid-body dynamics (Rapier3D) with friction cones and
vibration.

## Load distribution (warnings)

`ContainerPlan.balance` reports where the weight sits. Its issues are warnings: they
do not make the plan invalid. `options.balance.ctu_checks` (on by default) switches
the CTU Code checks:

- **Centre of gravity lengthwise and sideways** within `max_eccentricity` (5%) of
  the length and the width from the middle (CTU Code annex 7). This keeps the
  container level when a spreader lifts it and splits the load evenly over the
  axles and the side walls.
- **Middle half:** at least `min_central_share` (60%) of the cargo mass between 25%
  and 75% of the length. Each unit's mass is split by how much of its length lies
  in that band. An evenly filled container has exactly 50% there, so this rule asks
  for the heavy units towards the middle.
- **Centre of gravity height** at most `max_cog_height` (50%) of the inside height.
- Also shown: the share in each half of the length, and the free length to the
  front wall and to the door. When a gap is larger than `max_fill_gap`, it must be
  braced.

Always reported when the data is set:

- **VGM** (SOLAS chapter VI, regulation 2, method 2) = container tare + cargo.
  Dunnage and lashing material come on top.
- **Road vehicle axle loads** (`container.vehicle`, a tractor + semi-trailer) by
  moments. The container mass M is the cargo plus the tare, which counts at
  mid-length. Its centre of gravity lies x behind the kingpin. The trailer axle
  group sits at L_semi; the tractor's front axle, kingpin and drive axle are at 0,
  d_ant and L_tr; the tares T_t and T_tr have their centres of gravity at x_t and
  x_tr.

  ```
  R_trailer = (M · x + T_t · x_t) / L_semi
  P_kingpin = M + T_t − R_trailer
  R_drive   = (P_kingpin · d_ant + T_tr · x_tr) / L_tr
  R_steer   = P_kingpin + T_tr − R_drive
  ```

  The defaults are the EU limits (Directive 96/53/EC as amended by 2015/719): steer
  axle 10 t, drive axle 11.5 t, tri-axle group 24 t, 44 t gross for a 40 ft ISO
  container in combined transport.
- **Floor pressure** (ISO 1496-1). Each unit's share of the load reaching the floor
  is divided by its floor contact area. Round units on a line or point are taken to
  sit in cradles spread over their footprint. Above `container.floor_rating`
  (2500 kg/m² for the ISO presets), the unit needs load-spreading beams over at
  least `load / rating` m². The app has a "Floor pressure" colour mode.

### Lengthwise centring

With `options.balance.centre_lengthwise` (off by default), a partial load is slid
along the length after packing. It moves by the shortest distance that best meets
the centre-of-gravity window, the middle-half share, the axle limits of the
container and of the vehicle, and any hard longitudinal window. It stays within the
free length at both ends.

Exact contact queries are translation-invariant only up to rounding. So the moved
load is rebuilt unit by unit through the placer's own checks: every loading step
stays verified. It is kept only if it then validates cleanly. The gaps it leaves at
the front wall and the door must be braced; "Load end secured" models that bracing
in the transport check.
