# User guide

This guide walks through the OmniPack app on Windows and Android. Both use the same
screens; on a phone they are split into **Setup**, **3D view** and **Results** tabs, with
a **Pack** button at the bottom.

## 1. Describe the container

In **Container**, enter the inside width (X), height (Y) and depth (Z) in millimetres.
The **door is at the far end of the depth axis** and is drawn in orange. Depth 0 is the
front wall, where braking pushes the cargo.

Optional limits:
- **Max payload** (kg).
- **Max CoG offset**: how far the cargo's centre of gravity may move off the
  centreline, in mm.

Leave a field empty for "no limit".

## 2. Add the cargo

Use **+ Add…** to add an item type, choosing its shape:

| Shape | Size fields | Typical cargo |
|---|---|---|
| Box | W × H × D | cartons, crates, pallets |
| Cylinder / drum | radius, length | drums, rolls, pipes |
| Sphere | radius | balls, tanks |
| Cone | base radius, height | traffic cones, hoppers |
| Pyramid | base W × D, height | packaged pyramids, hoods |
| Prism | sides (3 = triangle, 6 = hexagon…), radius, length | beams, bars, extrusions |
| L-profile | leg A, leg B, thickness, length | steel angles |

For each item type you can set:
- **Mass** (kg per unit) and **Quantity**.
- **Max load on top** (kg): everything stacked above counts, not just the item directly
  on top. **Fragile** means nothing may go on top.
- **Stop:** the delivery stop, where 1 is unloaded first. Leave 0 for cargo that stays
  aboard.
- **Zone:** anywhere, near the back wall, or near the door.
- **Friction μ:** friction against whatever the item stands on. Leave it empty to use
  the default.
- **This side up / Upright only:** keeps the item's height axis vertical.
- **Floor only:** the item must stand on the container floor.

Samples (the **Sample…** menu) show complete setups: a mixed truck load, all shapes in
a 20 ft container, and benchmark sets.

## 3. Choose the loading strategy

- **Unloading order:**
  - **LIFO:** for vehicles unloaded through a rear door. The last stop is loaded first,
    deep inside, and the first stop ends up at the door.
  - **FIFO:** for side-loading or drive-through. The first stop is loaded first and
    filling starts at the door.
- **Within a stop, load:** largest, heaviest, largest base or tallest first, or as
  listed.
- **Fill pattern:** the order in which space is used. Choose walls across the width, full
  floor layers, walls along the length, rows along the length, or growing from a corner.
- **★ Best: search all patterns & orders:** instead of one pattern, OmniPack searches.
  It tries every fill pattern and load priority, then evolves loading orders and
  orientations (a genetic search followed by local search). Every candidate gets the full
  physics check, and stops, zones and floor-only rules are always kept.
  - **Search time** (5–120 s): the search keeps the best plans found in that time.
    **Pack** turns into **Stop ■** while it runs; stopping keeps what was found so far.
  - **Prefer:** slide towards *max. density* to fill the most volume, or towards
    *least securing* to favour plans that need less lashing and dunnage.
  - You get up to three clearly different plans to choose from in **Results**, next to
    what your own settings give.
- **Stability margin:** how far each centre of gravity must stay inside its support area.
  0 means "just doesn't tip"; higher values are safer.
- **Minimum support area:** the share of a flat bottom that must rest on something.
- **Balance:** how strongly to keep the load centred left and right.
- **Allow rotation:** lets items turn into any of their resting orientations.

## 4. Choose the physics

Gravity support, tipping at rest, stacking limits and payload are always checked.
Under **Physics & transport**:

- **Transport legs:** tick every leg of the journey, for example Road, then Rail
  (shunting), then Sea area B. Each preset lists its accelerations (forward / backward /
  sideways, in g).
- **Sliding:** friction must hold the item unless a wall or a chain of touching items
  blocks it.
- **Tipping:** the item and everything on it must not tip, unless it is blocked higher up
  than its centre of gravity.
- **Dynamic stacking:** stack limits are checked with the vertical acceleration added,
  which matters at sea.
- **Chocks for round items:** lying drums and balls are held with wedges. Switch this off
  to require them to be wedged in by walls or neighbours instead.
- **Load end secured:** a locking bar, gate or dunnage closes the open end of the load.
- **Anti-slip mats:** rubber mats under every item and between layers (μ ≥ 0.6). They
  hold 0.5 g sideways on the road, but not 0.8 g braking.
- **Cargo on floor:** typical friction values from EN 12195-1 (sawn-wood pallet on
  plywood 0.45, plastic pallet 0.2, …). Picking one sets **Default friction μ**.
- **Dunnage fills gaps up to** (mm): gaps up to this size between items, or between an
  item and a wall, count as filled with dunnage or airbags and block like direct contact.
  0 means only touching faces block. With the common μ 0.4, friction alone never holds
  cargo on the road, so blocking decides almost everything. If this is too small for your
  practice, nearly every unit will show as needing lashing.

## 5. Pack and read the results

Press **Pack** (or Ctrl+Enter). The badges at the top of **Results** summarise the plan:

- **✓ Stable at rest**, or **✗ Violations found** with the list of problems.
- **Securing:** either **Secured for transport**, or **N units need lashing**. Each
  selected transport leg then lists what would slide or tip, in which direction, and the
  force (kN) the lashing or blocking must provide. The largest forces are listed first.
  **Fill N gaps with dunnage** lists the gaps the blocking relies on, with their width.
- **Chocks:** how many round items need wedges.

Below the badges you'll find fill %, cargo mass, centre of gravity, axle loads,
unloading accessibility (the share of items reachable at their stop without moving
another stop's cargo), and any units that could not be packed, with the reason.

## 6. Explore the 3D view

- **Rotate** with a left drag or one finger, **zoom** with the wheel or a pinch, **pan**
  with a right drag. ⟲ resets the camera.
- **Colour by** item, delivery stop, load against limit, stability margin, securing
  needed, or the impact heatmap.
  - *Securing needed:* red = needs lashing, purple = stack overloaded, orange = chocks,
    blue = held once the listed gaps are filled, green = secured.
  - *Impact heatmap:* the transport force each unit must pass on to whatever blocks it,
    in the worst transport leg and direction. This is its own push plus everything
    behind it in the blocking chain. Green is low, red is the highest in this container.
    The four bands can be isolated like any legend entry.
- **Legend:** click entries to show only those groups (the others fade out); click again
  to remove one; **Show all** resets.
- **Timeline:** use ⏮ ◀ ▶ ▶| ⏭ or the keys ← → Home End Space to go through the load
  one item at a time. The next item is shown as an orange ghost at its target position,
  and its name, mass and coordinates appear under the slider.
- **Click an item** to see its details: load order, orientation, load on top, stability
  margin, and any securing it needs.

## 7. Save and export

- **Save… / Open…:** the whole setup as a JSON file.
- **Catalog:** named setups kept inside the app.
- **Export plan (JSON):** every placement with its coordinates, for other software.
- **Export load list (CSV):** a loading list for the warehouse, with position, size,
  orientation, mass, stop, chocks and securing notes.
