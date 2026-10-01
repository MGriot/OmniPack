# Research notes: stability and fill patterns

These notes cover the literature and standards behind OmniPack's placer,
securing model and search. Each entry says what it contributes, what OmniPack
takes from it, and where. Measured effects come from
`omnipack bench <n> [--optimize <s>]` on the BR1–BR7-style generator.

## Stability at rest

**Ramos, Oliveira, Gonçalves & Lopes (2016), "A container loading algorithm
with static mechanical equilibrium stability constraints", *Transportation
Research Part B* 91.**
- Replaces the traditional "full base support" rule with Newton–Euler
  equilibrium of rigid bodies. This packs denser and is still physically sound.
- It is also what OmniPack already does:
  - support polygons come from exact contacts;
  - the centre of gravity or load resultant must lie inside the polygon with a
    margin;
  - loads are propagated down every stack (minimum-norm contact forces);
  - an independent validator re-checks every plan and every loading prefix.
- *Kept as is.* The only choice left is `min_support_ratio` (default 50%), a
  practical rule for soft or deformable bases.

**Online 3D bin packing with fast stability validation (arXiv 2507.09123,
2025).**
- Shows that incremental, per-item stability checks are enough when loads
  propagate downward.
- OmniPack's placer already works this way: it re-checks only the items below
  the new one (`placer.rs`, `check`).

## Transport securing (EN 12195-1, CTU Code)

**EN 12195-1:2010 and the IMO/ILO/UNECE CTU Code (2014), annex 7 and the
Quick Lashing Guide.**
- *Acceleration factors.* Road: 0.8 g forward, 0.5 g backward and sideways.
  Sea and rail values are in the `TransportCase::presets`.
- *Friction.* Annex B gives typical dry values, for example:
  - sawn wood on plywood 0.45;
  - plastic pallet on plywood 0.2;
  - rubber anti-slip mats 0.6.

  With the common 0.4, friction alone never holds cargo on the road, so almost
  all road cargo must be **blocked** (form-locked) or **lashed**.
- *Blocking.* The standard accepts blocking by walls, headboards, other cargo
  and *filled* void spaces. Gaps are filled with dunnage, timber or airbags.
  Leaving them open lets cargo build up speed before it hits the blocker.

**What OmniPack takes (`validate.rs`, `transport_report`):**
- `max_fill_gap` (default 50 mm, up to 200 mm in the app): gaps up to this
  size count as filled, and every gap the blocking relies on is listed.
  Before this, blocking needed the faces to be within 0.05 mm. In real loads
  that almost never happens, so practically every unit was reported as
  "needs securing".
- A separate floor friction and an anti-slip-mat option. The app has a
  friction preset list with the Annex B values.
- Securing classes per unit: secured, held once gaps are filled, chocks,
  lashing, stack overloaded.
- **Impact.** The force each unit must pass on to its blocker: its own excess
  over friction plus what the units behind push into it along the blocking
  chain. The units at the end of a chain (the front wall under braking) carry
  the sum. This is the quasi-static EN 12195-1 force, not a crash simulation.

- **Direct lashing count** (since 0.5). Each sliding or tipping force is also
  turned into a number of direct lashings with the EN 12195-1 formula: the
  friction factor f_μ = 0.75, the angles α and β, and the weaker of strap and
  lashing point. ISO 1496-1 requires at least 1000 daN for floor lashing points
  and 500 daN for those on the posts. A 2000 daN strap on a floor ring therefore
  counts as 1000 daN.

Not modelled yet:
- Top-over and loop lashing.
- Blocking through the layer below (friction plus lip).
- Dynamic effects. Rapier3D is planned for M3.

## Load distribution and vehicle (since 0.5)

**IMO/ILO/UNECE CTU Code (2014), annex 7 ("Packing and securing of cargo into
CTUs").**
- The cargo's centre of gravity should stay close to the middle of the CTU.
  OmniPack's defaults are 5% of the length and of the width: a container lifted by
  a spreader then hangs level, and the load splits evenly over the axles.
- The centre of gravity should be in the lower half of the height.
- The 60/50 rule is quoted in two ways. "No more than 60% of the mass in one half
  of the length" is one. "At least 60% of the mass in the middle half (25–75% of
  the length)" is the other.
  - OmniPack checks the second, by the project owner's choice; the threshold is
    configurable.
  - Note that an evenly filled container has 50% in the middle half, so the rule
    flags uniform loads and favours heavy units in the middle.
  - The share in each half is reported too.
- **What OmniPack takes (`balance.rs`):**
  - The checks are warnings in `ContainerPlan.balance`, not violations. A
    partial load often cannot meet them, and the property tests require every
    plan and loading step to stay valid.
  - The ★ Best search subtracts `Objective::balance × excess` from its value.
  - The optional lengthwise centring slides a partial load by the shortest
    distance that best meets the window, the middle share and the axle limits.
    The moved plan is rebuilt through the placer's checks.

**SOLAS chapter VI, regulation 2 (verified gross mass, in force since 2016).**
Method 2 adds up the packed cargo, the packing and securing material, and the
container tare from the CSC plate. OmniPack reports tare + cargo; the dunnage and
lashings come on top.

**ISO 1496-1 / ISO 668 (series 1 freight containers).**
- They give the inside sizes, the door openings (2340 × 2280 mm; 2340 × 2585 mm
  for high cubes) and the floor test: a forklift axle of 5460 kg on wheels of
  142 cm² each.
- OmniPack's presets use typical inside sizes and tares for a 30 480 kg maximum
  gross mass.
- The door opening is a hard placement rule.
- The floor check compares each unit's contact pressure, including everything it
  carries, with a floor rating. The default 2500 kg/m² is the upper end of the
  2.0–2.5 t/m² usually quoted. Above it the unit is reported with the area its
  load must be spread over, with timber or steel beams across the floor
  cross-members.

**Directive 96/53/EC as amended by (EU) 2015/719 (weights and dimensions).**
- The limits are 10 t for a non-driven axle, 11.5 t for a drive axle and 24 t for
  a tri-axle group (spacing over 1.3 m). A 5-axle combination may weigh 40 t, or
  44 t carrying a 40 ft ISO container in combined transport.
- OmniPack splits the container (cargo + tare) and the trailer tare between the
  kingpin and the trailer axles by moments, and then the kingpin load and the
  tractor tare between the steer and drive axles.
- With a 40 ft chassis the drive axle is usually the first to overload, because
  the kingpin sits just ahead of it.

**Ship motion (roll and pitch).**
- The natural roll period is `T = 2·c·B/√GM` (c ≈ 0.4).
- The transverse acceleration at height z above the roll axis is
  `g·sin θ + (2π/T)²·z·sin θ`.
- Pitch adds `(2π/T_p)²·θ_p` times the distance from the pitch axis: fore and aft
  at height, vertically towards bow and stern.
- These are the textbook rigid-body terms behind the CTU Code tables for sea
  areas A/B/C. `TransportCase::from_ship` uses them to make a case for a specific
  ship and stowage place.
- A GM below the 0.15 m IS Code minimum means a tender ship. A short roll period
  means a stiff one, and violent accelerations high up on deck.

## Candidate positions

**Crainic, Perboli & Tadei (2008), "Extreme point-based heuristics for
three-dimensional bin packing", *INFORMS Journal on Computing* 20(3).**
- Extreme points (EPs) are the corners of placed items, projected along each
  axis until they meet the container or another item.
- They beat corner points because they also expose pockets *between* items.

**What OmniPack takes (`placer.rs`, `commit`):**
- Anchors were the item corners projected onto the walls. They now also
  include projections onto the nearest item towards the back and left walls.
- Height still comes from the gravity drop, so the anchors stay in the floor
  plane.
- Effect: part of the +3.1 points in the table below. With no tie-break
  weights, utilization went from 75.4% to 75.9%.

## Placement score (tie-breaks)

**Bortfeldt & Wäscher (2013), "Constraints in container loading – a
state-of-the-art review", *EJOR* 229(1).** It notes two things:
- Contact-maximising and "flat surface" rules improve both density and
  stability, because they leave fewer unusable voids and give better surfaces
  for the next layer.
- Real-world constraints (stability, load bearing, securing) are rarely
  optimized together.

OmniPack adds these secondary terms to the fill-pattern score (`ScoreWeights`
on `PackOptions`). Each value is a fraction of one container length along the
main fill direction:

| Term | Default | Rewards / penalises |
|---|---|---|
| `contact_area` | 0.004 | share of the four side faces touching walls or items |
| `blocking` | 0.004 | sides blocked against sliding, directly or across a fillable gap (transport checks on) |
| `dead_gap` | 0.004 | penalty per side leaving a gap wider than the dunnage limit but narrower than the smallest unit |
| `flat_top` | 0.002 | top flush with a touching neighbour |

Weight sweep (BR1–7, 4 instances per class, road transport):

| Weights | Avg. utilization | Units needing lashing |
|---|---|---|
| all 0 (extreme points only) | 75.9% | 3.7% |
| **defaults** | **79.0%** | **2.5%** |
| contact 0.01 | 79.3% | 3.5% |
| flat top 0 | 79.0% | 3.5% |
| blocking 0.01 | 78.5% | 3.2% |

- `dead_gap` made no measurable difference on these instances. It is kept,
  small, for loads with a wider size spread.
- Override the weights with
  `OMNIPACK_WEIGHTS=contact,blocking,dead_gap,flat_top omnipack bench`.

## Search: "Best (search all)"

**Gonçalves & Resende (2013), "A biased random key genetic algorithm for 2D
and 3D bin packing problems", *IJPE* 145(2).** Also Gonçalves & Resende (2012),
parallel multi-population BRKGA for container loading.
- Random keys decode through a constructive placer. Children inherit from an
  elite parent with probability ρ ≈ 0.7. A share of each generation is fresh
  random mutants.
- Among the best published results on the BR benchmarks.

**What OmniPack takes (`crates/omnipack-opt`):**
1. **Sweep.** Every fill pattern × load priority: exhaustive over the placer
   settings, and the seeds for step 2.
2. **BRKGA.**
   - A chromosome is one order key per unit, one preferred-orientation key per
     unit and one fill-pattern key.
   - Units are only reordered within their loading group (stop, zone,
     floor-only), so LIFO/FIFO and zones always hold.
   - The orientation key steers softly (`ORIENT_PREF_WEIGHT`), because a hard
     choice would make many chromosomes infeasible.
3. **Local search.** Swaps within a group, and changes to one orientation or
   the fill pattern.
4. **Ranking.** All units packed first, then fewest containers, then
   `density·util% − securing·lashing% − dunnage·m + stability·margin`. The
   app's "Prefer" slider moves weight between density and securing.
5. **Result.** The three best plans that differ in at least 15% of the unit
   positions.

**Effect** (2 instances per class, 5 s each):
- utilization: 78.3% (greedy) → **83.4%**;
- every plan still valid;
- on the mixed truck sample (15 s), units needing lashing fell from 48 to 6.

**Not taken:**
- NSGA-II Pareto fronts. The weighted objective plus three distinct plans
  covers the UI need for now.
- Block building (Fanslau & Bortfeldt 2010). With the contact score, identical
  units already end up in rows. A true block builder would place composite
  blocks, and that needs a placer change: see `conductor/stage.md` M4.

## References

- Bortfeldt, A., Wäscher, G. (2013). Constraints in container loading – a state-of-the-art review. *EJOR* 229(1), 1–20.
- Crainic, T. G., Perboli, G., Tadei, R. (2008). Extreme point-based heuristics for three-dimensional bin packing. *INFORMS J. Computing* 20(3), 368–384.
- Fanslau, T., Bortfeldt, A. (2010). A tree search algorithm for solving the container loading problem. *INFORMS J. Computing* 22(2), 222–235.
- Gonçalves, J. F., Resende, M. G. C. (2012). A parallel multi-population biased random-key genetic algorithm for a container loading problem. *Computers & OR* 39(2), 179–190.
- Gonçalves, J. F., Resende, M. G. C. (2013). A biased random key genetic algorithm for 2D and 3D bin packing problems. *IJPE* 145(2), 500–510.
- Ramos, A. G., Oliveira, J. F., Gonçalves, J. F., Lopes, M. P. (2016). A container loading algorithm with static mechanical equilibrium stability constraints. *Transportation Research Part B* 91, 565–581.
- EN 12195-1:2010 Load restraining on road vehicles – Safety – Part 1: Calculation of securing forces.
- ISO 1496-1:2013 Series 1 freight containers – Specification and testing – Part 1: General cargo containers; ISO 668:2020 Classification, dimensions and ratings.
- SOLAS chapter VI, regulation 2, as amended by resolution MSC.380(94): verified gross mass of containers (MSC.1/Circ.1475 guidelines).
- Council Directive 96/53/EC, as amended by Directive (EU) 2015/719: maximum weights and dimensions of road vehicles in the EU.
- IMO 2008 Intact Stability Code (resolution MSC.267(85)).
- IMO/ILO/UNECE Code of Practice for Packing of Cargo Transport Units (CTU Code), 2014.
- Online 3D Bin Packing with Fast Stability Validation and Stable Rearrangement Planning, arXiv:2507.09123 (2025).
