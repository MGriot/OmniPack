//! Hand-made plans: units the user places anywhere in one container. They
//! are checked like any plan but never refused; the violations say what is
//! wrong. [`crate::pack_with_fixed`] packs the remaining units around them.

use crate::model::{PackRequest, StopOrder};
use crate::plan::{ContainerPlan, Placement, SecuringClass, Violation};
use crate::scene;
use crate::statics::Support;
use crate::validate;
use omnipack_geom::{drop_height, overlaps, tol, Body, Orientation, OrientedShape, Pt2};
use serde::{Deserialize, Serialize};

/// Where a unit would end up, and what would be wrong there.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub placement: Placement,
    pub problems: Vec<Violation>,
}

/// A unit of `item_id` at floor position (`x`, `z`) in `orientation`. With
/// `y = None` it is lowered onto whatever is below (gravity drop); with a `y`
/// it is put exactly there, floating or not. `replace` is the instance id of
/// a unit being moved (it keeps its id and is not an obstacle to itself).
#[allow(clippy::too_many_arguments)]
pub fn place(
    req: &PackRequest,
    placements: &[Placement],
    item_id: &str,
    orientation: Orientation,
    x: f64,
    z: f64,
    y: Option<f64>,
    replace: Option<&str>,
) -> Result<Placement, String> {
    let spec = req.items.iter().find(|i| i.id == item_id).ok_or_else(|| format!("no item `{item_id}`"))?;
    if !spec.shape.is_valid() {
        return Err(format!("item `{item_id}` has an invalid shape"));
    }
    let shape = OrientedShape::new(&spec.shape, orientation, spec.com_offset);
    let others: Vec<&Placement> = placements.iter().filter(|p| Some(p.instance_id.as_str()) != replace).collect();
    let y = match y {
        Some(y) => y,
        None => {
            let shapes: Vec<OrientedShape> = others.iter().map(|p| validate::rebuild_shape(p)).collect();
            drop_height(&shape, x, z, others.iter().zip(&shapes).map(|(p, s)| Body::new(s, p.position)))
        }
    };
    let instance_id = match replace {
        Some(id) => id.to_string(),
        None => (1..).map(|k| format!("{item_id}#{k}")).find(|id| !placements.iter().any(|p| &p.instance_id == id)).unwrap_or_default(),
    };
    // A moved unit keeps its place in the loading order; a new one comes last.
    let seq = placements.iter().find(|p| Some(p.instance_id.as_str()) == replace).map_or(placements.iter().map(|p| p.seq + 1).max().unwrap_or(0), |p| p.seq);
    let min = [x, y, z];
    Ok(Placement {
        instance_id,
        item_id: item_id.to_string(),
        seq,
        shape: spec.shape.clone(),
        orientation,
        position: min,
        size: shape.extents,
        center_of_mass: std::array::from_fn(|k| min[k] + shape.com_from_min[k]),
        mass: spec.mass,
        load_on_top: 0.0,
        support_margin: 0.0,
        stop: spec.stop,
        needs_chocks: false,
        securing: SecuringClass::Secured,
        impact: None,
        color: spec.color.clone(),
        floor_pressure: 0.0,
    })
}

/// [`place`] plus a quick check of that one unit against the others: bounds,
/// overlaps, door, orientation, floor-only, support, support area, rolling,
/// stability at rest, fragile units below and the payload. Loads on the
/// units below are left to [`evaluate`].
#[allow(clippy::too_many_arguments)]
pub fn probe(
    req: &PackRequest,
    placements: &[Placement],
    item_id: &str,
    orientation: Orientation,
    x: f64,
    z: f64,
    y: Option<f64>,
    replace: Option<&str>,
) -> Result<Probe, String> {
    let p = place(req, placements, item_id, orientation, x, z, y, replace)?;
    let (c, opts) = (&req.container, &req.options);
    let spec = req.items.iter().find(|i| i.id == item_id).ok_or_else(|| format!("no item `{item_id}`"))?;
    let shape = validate::rebuild_shape(&p);
    let body = Body::new(&shape, p.position);
    let others: Vec<&Placement> = placements.iter().filter(|q| Some(q.instance_id.as_str()) != replace).collect();
    let shapes: Vec<OrientedShape> = others.iter().map(|q| validate::rebuild_shape(q)).collect();
    let bodies: Vec<Body> = others.iter().zip(&shapes).map(|(q, s)| Body::new(s, q.position)).collect();
    let id = p.instance_id.clone();
    let mut problems = Vec::new();

    let size = [c.width, c.height, c.depth];
    let mx = body.max();
    if (0..3).any(|k| body.min[k] < -tol::BOUNDS || mx[k] > size[k] + tol::BOUNDS) {
        problems.push(Violation::OutOfBounds { item: id.clone() });
    }
    for (q, b) in others.iter().zip(&bodies) {
        if overlaps(&body, b) {
            problems.push(Violation::Overlap { a: id.clone(), b: q.instance_id.clone() });
        }
    }
    if !spec.orientations(opts.allow_rotation).contains(&orientation) {
        problems.push(Violation::OrientationNotAllowed { item: id.clone() });
    }
    if spec.floor_only && body.min[1] > tol::CONTACT {
        problems.push(Violation::NotOnFloor { item: id.clone() });
    }
    if !validate::passes_door(c, &shape) {
        problems.push(Violation::DoorTooSmall { item: id.clone() });
    }
    let info = scene::compute_supports(&body, bodies.iter().enumerate().map(|(j, b)| (j, *b)));
    if info.set.is_empty() {
        problems.push(Violation::Unsupported { item: id.clone() });
    } else {
        if let Some(bottom) = scene::flat_bottom_area(&body) {
            let ratio = info.contact_area / bottom;
            if ratio < opts.min_support_ratio - 1e-6 {
                problems.push(Violation::InsufficientSupportArea { item: id.clone(), ratio, required: opts.min_support_ratio });
            }
        }
        let roll = scene::roll_state(&body, &info.set, bodies.iter().copied(), size, opts.physics.use_chocks);
        if roll == scene::Roll::Free {
            problems.push(Violation::MayRoll { item: id.clone() });
        }
        let com = body.com();
        let required = scene::required_margin(&body, opts.stability_margin);
        let margin = scene::stability_margin(&info.set, Pt2::new(com[0], com[2]), roll.is_held());
        if margin < required - 1e-6 {
            problems.push(Violation::Unstable { item: id.clone(), margin, required });
        }
        for owner in &info.set.owners {
            if let Support::Item(j) = owner {
                let below = others[*j];
                let capacity = req.items.iter().find(|i| i.id == below.item_id).map_or(f64::INFINITY, |s| s.capacity());
                if capacity <= 0.0 && !problems.iter().any(|v| matches!(v, Violation::Overloaded { item, .. } if item == &below.instance_id)) {
                    problems.push(Violation::Overloaded { item: below.instance_id.clone(), load: p.mass, capacity });
                }
            }
        }
    }
    let total: f64 = others.iter().map(|q| q.mass).sum::<f64>() + p.mass;
    if let Some(max) = c.max_payload.filter(|m| total > m + 1e-6) {
        problems.push(Violation::PayloadExceeded { mass: total, max });
    }
    Ok(Probe { placement: p, problems })
}

/// Loading order of a hand-made plan: lower units first, then along the fill
/// direction (from the front wall for LIFO, from the door for FIFO), then
/// across. Supports only come from units loaded earlier, so a unit slid under
/// another one later still carries it.
pub fn resequence(req: &PackRequest, placements: &mut [Placement]) {
    let fifo = req.options.stop_order == StopOrder::Fifo;
    let round = |v: f64| (v * 10.0).round() as i64;
    placements.sort_by_key(|p| {
        let z = if fifo { -round(p.position[2] + p.size[2]) } else { round(p.position[2]) };
        (round(p.position[1]), z, round(p.position[0]))
    });
    for (seq, p) in placements.iter_mut().enumerate() {
        p.seq = seq;
    }
}

/// The full plan of a hand-made container, checked like an automatic one.
/// The units keep the order they were placed in (which is also what the
/// learned placement learns from), unless loading them lowest first
/// ([`resequence`]) gives fewer violations.
pub fn evaluate(req: &PackRequest, mut placements: Vec<Placement>) -> ContainerPlan {
    placements.sort_by_key(|p| p.seq);
    for (seq, p) in placements.iter_mut().enumerate() {
        p.seq = seq;
    }
    let check = validate::check_plan(&req.container, &req.items, &req.options, &placements);
    let (mut placements, check) = if check.violations.is_empty() {
        (placements, check)
    } else {
        let mut by_height = placements.clone();
        resequence(req, &mut by_height);
        let other = validate::check_plan(&req.container, &req.items, &req.options, &by_height);
        if other.violations.len() < check.violations.len() {
            (by_height, other)
        } else {
            (placements, check)
        }
    };
    for (i, p) in placements.iter_mut().enumerate() {
        p.needs_chocks = check.needs_chocks[i];
        p.load_on_top = check.load_on_top[i];
        p.support_margin = check.margin[i];
    }
    crate::build_plan(req, placements, 0.0, check, 0)
}

/// Units of each item still to place, in item order.
pub fn remaining(req: &PackRequest, placements: &[Placement]) -> Vec<(String, u32)> {
    req.items
        .iter()
        .map(|i| {
            let placed = placements.iter().filter(|p| p.item_id == i.id).count() as u32;
            (i.id.clone(), i.quantity.saturating_sub(placed))
        })
        .collect()
}
