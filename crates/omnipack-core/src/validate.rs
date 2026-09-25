//! Independent verification of a finished container plan.
//!
//! Recomputes everything from the placement records alone (no placer state):
//! bounds, exact interpenetration, supports in loading order, support area,
//! own and stacked centre-of-gravity margins, rolling, full load propagation to
//! the floor, container payload, axle loads and centre-of-gravity limits.
//! [`transport`] adds the quasi-static transport checks on top.

use crate::model::{ContainerSpec, ItemSpec, PackOptions, PhysicsOptions, TransportCase};
use crate::plan::{Direction, IssueKind, Metrics, Placement, TransportIssue, TransportResult, Violation};
use crate::scene::{self, compute_supports, Roll, SupportInfo};
use crate::statics::{add, effective_mass, load_at, resultant, Load, Support};
use omnipack_geom::{overlaps, ray_exit_distance, tol, Body, OrientedShape, Pt2};
use std::collections::HashMap;

const G: f64 = 9.81;

pub fn rebuild_shape(p: &Placement) -> OrientedShape {
    let mut s = OrientedShape::new(&p.shape, p.orientation, [0.0; 3]);
    // Any com_offset is recovered from the recorded centre of mass.
    s.com_from_min = [
        p.center_of_mass[0] - p.position[0],
        p.center_of_mass[1] - p.position[1],
        p.center_of_mass[2] - p.position[2],
    ];
    s
}

fn aabb_touch(a: &Body, b: &Body, margin: f64) -> bool {
    let (amin, amax, bmin, bmax) = (a.min, a.max(), b.min, b.max());
    (0..3).all(|k| amin[k] <= bmax[k] + margin && bmin[k] <= amax[k] + margin)
}

/// Geometry and statics of a finished plan, shared by the checks.
struct Analysis<'a> {
    bodies: Vec<Body<'a>>,
    near: Vec<Vec<usize>>,
    infos: Vec<SupportInfo>,
    roll: Vec<Roll>,
    required: Vec<f64>,
    /// Load resting on each item (from everything above), as (Σf, Σf·x, Σf·z).
    incoming: Vec<Load>,
    /// Σ mass·height of the load on top (for the combined centre of gravity).
    incoming_y: Vec<f64>,
    /// Static resultant of own weight plus load on top.
    resultant: Vec<Option<Pt2>>,
    /// Problems found while propagating loads (tipping, overload).
    statics_violations: Vec<Violation>,
}

fn analyze<'a>(
    container: &ContainerSpec,
    spec_of: &HashMap<&str, &ItemSpec>,
    opts: &PackOptions,
    placements: &[Placement],
    shapes: &'a [OrientedShape],
) -> Analysis<'a> {
    let bodies: Vec<Body> = placements.iter().zip(shapes).map(|(p, s)| Body::new(s, p.position)).collect();
    let size = [container.width, container.height, container.depth];
    let n = placements.len();

    // Touching / overlapping pairs by sweeping along x.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| bodies[a].min[0].total_cmp(&bodies[b].min[0]));
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (oi, &a) in order.iter().enumerate() {
        for &b in &order[oi + 1..] {
            if bodies[b].min[0] > bodies[a].max()[0] + tol::CONTACT {
                break;
            }
            if aabb_touch(&bodies[a], &bodies[b], tol::CONTACT) {
                near[a].push(b);
                near[b].push(a);
            }
        }
    }

    // Supports as they existed when each item was loaded (earlier seq only).
    let infos: Vec<SupportInfo> = (0..n)
        .map(|i| {
            let earlier = near[i].iter().copied().filter(|&j| placements[j].seq < placements[i].seq);
            compute_supports(&bodies[i], earlier.map(|j| (j, bodies[j])))
        })
        .collect();
    let roll: Vec<Roll> = (0..n)
        .map(|i| scene::roll_state(&bodies[i], &infos[i].set, near[i].iter().map(|&j| bodies[j]), size, opts.physics.use_chocks))
        .collect();
    let required: Vec<f64> = bodies.iter().map(|b| scene::required_margin(b, opts.stability_margin)).collect();

    // Full load propagation, carrying mass-weighted heights too. Supports only
    // point to earlier-loaded items, so reverse loading order is top-down.
    let mut incoming: Vec<Load> = vec![[0.0; 3]; n];
    let mut incoming_y = vec![0.0; n];
    let mut res = vec![None; n];
    let mut statics_violations = Vec::new();
    let mut top_down: Vec<usize> = (0..n).collect();
    top_down.sort_by(|&a, &b| placements[b].seq.cmp(&placements[a].seq));
    for &i in &top_down {
        let p = &placements[i];
        let info = &infos[i];
        if info.set.is_empty() {
            continue;
        }
        let capacity = spec_of.get(p.item_id.as_str()).map_or(f64::INFINITY, |s| s.capacity());
        if incoming[i][0] > capacity + 1e-6 {
            statics_violations.push(Violation::Overloaded { item: p.instance_id.clone(), load: incoming[i][0], capacity });
        }
        let com = bodies[i].com();
        let own = effective_mass(p.mass);
        let total = add(load_at(own, Pt2::new(com[0], com[2])), incoming[i]);
        let Some(r) = resultant(total) else { continue };
        res[i] = Some(r);
        let held = roll[i].is_held();
        let m = scene::stability_margin(&info.set, r, held);
        if m < required[i] - 1e-6 {
            statics_violations.push(Violation::Unstable { item: p.instance_id.clone(), margin: m, required: required[i] });
        }
        let column_y = own * com[1] + incoming_y[i];
        if let Some(shares) = scene::distribute(&info.set, total, held) {
            for (s, l) in shares {
                if let Support::Item(j) = s {
                    incoming[j] = add(incoming[j], l);
                    incoming_y[j] += column_y * l[0] / total[0];
                }
            }
        }
    }

    Analysis { bodies, near, infos, roll, required, incoming, incoming_y, resultant: res, statics_violations }
}

pub fn validate(container: &ContainerSpec, specs: &[ItemSpec], opts: &PackOptions, placements: &[Placement]) -> Vec<Violation> {
    let spec_of: HashMap<&str, &ItemSpec> = specs.iter().map(|s| (s.id.as_str(), s)).collect();
    let shapes: Vec<OrientedShape> = placements.iter().map(rebuild_shape).collect();
    let a = analyze(container, &spec_of, opts, placements, &shapes);
    let size = [container.width, container.height, container.depth];
    let mut v = Vec::new();

    for (i, (p, b)) in placements.iter().zip(&a.bodies).enumerate() {
        let mx = b.max();
        if (0..3).any(|k| b.min[k] < -tol::BOUNDS || mx[k] > size[k] + tol::BOUNDS) {
            v.push(Violation::OutOfBounds { item: p.instance_id.clone() });
        }
        for &j in &a.near[i] {
            if j > i && overlaps(b, &a.bodies[j]) {
                v.push(Violation::Overlap { a: p.instance_id.clone(), b: placements[j].instance_id.clone() });
            }
        }
        if let Some(spec) = spec_of.get(p.item_id.as_str()) {
            if !spec.orientations(opts.allow_rotation).contains(&p.orientation) {
                v.push(Violation::OrientationNotAllowed { item: p.instance_id.clone() });
            }
            if spec.floor_only && b.min[1] > tol::CONTACT {
                v.push(Violation::NotOnFloor { item: p.instance_id.clone() });
            }
        }
    }

    for (i, (p, info)) in placements.iter().zip(&a.infos).enumerate() {
        let id = &p.instance_id;
        if info.set.is_empty() {
            v.push(Violation::Unsupported { item: id.clone() });
            continue;
        }
        if let Some(bottom) = scene::flat_bottom_area(&a.bodies[i]) {
            let ratio = info.contact_area / bottom;
            if ratio < opts.min_support_ratio - 1e-6 {
                v.push(Violation::InsufficientSupportArea { item: id.clone(), ratio, required: opts.min_support_ratio });
            }
        }
        if a.roll[i] == Roll::Free {
            v.push(Violation::MayRoll { item: id.clone() });
        }
        // Stable on its own at the moment it was loaded.
        let com = a.bodies[i].com();
        let m = scene::stability_margin(&info.set, Pt2::new(com[0], com[2]), a.roll[i].is_held());
        if m < a.required[i] - 1e-6 {
            v.push(Violation::Unstable { item: id.clone(), margin: m, required: a.required[i] });
        }
    }
    v.extend(a.statics_violations);

    // Container-level limits.
    let metrics = compute_metrics(container, placements);
    if let Some(max) = container.max_payload {
        if metrics.total_mass > max + 1e-6 {
            v.push(Violation::PayloadExceeded { mass: metrics.total_mass, max });
        }
    }
    if let (Some(axles), Some(loads)) = (container.axles, metrics.axle_loads) {
        for k in 0..2 {
            if loads[k] > axles[k].max_load + 1e-6 {
                v.push(Violation::AxleOverloaded { axle: k, load: loads[k], max: axles[k].max_load });
            }
        }
    }
    if metrics.item_count > 0 {
        let lim = &container.cog_limits;
        let c = metrics.center_of_mass;
        if lim.max_lateral_offset.is_some_and(|m| metrics.lateral_offset > m + 1e-6) {
            v.push(Violation::CogOutOfLimits { detail: format!("lateral offset {:.1} mm", metrics.lateral_offset) });
        }
        if lim.z_min.is_some_and(|z| c[2] < z - 1e-6) || lim.z_max.is_some_and(|z| c[2] > z + 1e-6) {
            v.push(Violation::CogOutOfLimits { detail: format!("longitudinal position {:.1} mm", c[2]) });
        }
        if lim.max_height.is_some_and(|h| c[1] > h + 1e-6) {
            v.push(Violation::CogOutOfLimits { detail: format!("height {:.1} mm", c[1]) });
        }
    }
    v
}

/// Which round items need wedges, in placement order.
pub fn needs_chocks(container: &ContainerSpec, specs: &[ItemSpec], opts: &PackOptions, placements: &[Placement]) -> Vec<bool> {
    let spec_of: HashMap<&str, &ItemSpec> = specs.iter().map(|s| (s.id.as_str(), s)).collect();
    let shapes: Vec<OrientedShape> = placements.iter().map(rebuild_shape).collect();
    analyze(container, &spec_of, opts, placements, &shapes).roll.iter().map(|r| *r == Roll::NeedsChocks).collect()
}

pub(crate) const DIRS: [(Direction, [f64; 2]); 4] = [
    (Direction::Forward, [0.0, -1.0]),
    (Direction::Backward, [0.0, 1.0]),
    (Direction::Left, [-1.0, 0.0]),
    (Direction::Right, [1.0, 0.0]),
];

pub(crate) fn accel(case: &TransportCase, d: Direction) -> f64 {
    match d {
        Direction::Forward => case.forward,
        Direction::Backward => case.backward,
        Direction::Left | Direction::Right => case.sideways,
    }
}

/// For every item and direction: is it held by a wall, or by a touching
/// neighbour that is itself held (a blocking chain)? The second value also
/// requires the blocker to reach above the item's centre of gravity, which is
/// what stops tipping.
fn blocking(a: &Analysis, size: [f64; 3], secure_load_end: bool) -> (Vec<[bool; 4]>, Vec<[bool; 4]>) {
    let n = a.bodies.len();
    // A locking bar / gate across the open ends of the load acts as a wall.
    let (load_front, load_back) = a.bodies.iter().fold((f64::MAX, f64::MIN), |(f, b), body| (f.min(body.min[2]), b.max(body.max()[2])));
    let mut slide = vec![[false; 4]; n];
    let mut tip = vec![[false; 4]; n];
    // Neighbours that touch face-to-face in each direction.
    let mut touching: Vec<[Vec<usize>; 4]> = vec![Default::default(); n];
    for i in 0..n {
        let (mn, mx) = (a.bodies[i].min, a.bodies[i].max());
        for (di, (_, d)) in DIRS.iter().enumerate() {
            let k = if d[0] != 0.0 { 0 } else { 2 };
            let other = 2 - k;
            let positive = d[0] + d[1] > 0.0;
            let mut wall = if positive { mx[k] >= size[k] - tol::CONTACT } else { mn[k] <= tol::CONTACT };
            if secure_load_end && k == 2 {
                wall |= if positive { mx[2] >= load_back - tol::CONTACT } else { mn[2] <= load_front + tol::CONTACT };
            }
            slide[i][di] = wall;
            tip[i][di] = wall;
            for &j in &a.near[i] {
                let (omn, omx) = (a.bodies[j].min, a.bodies[j].max());
                let faces = if positive { (omn[k] - mx[k]).abs() <= tol::CONTACT } else { (omx[k] - mn[k]).abs() <= tol::CONTACT };
                let overlap = omn[other] < mx[other] - tol::CONTACT
                    && mn[other] < omx[other] - tol::CONTACT
                    && omn[1] < mx[1] - tol::CONTACT
                    && mn[1] < omx[1] - tol::CONTACT;
                if faces && overlap {
                    touching[i][di].push(j);
                }
            }
        }
    }
    // Propagate blocking along chains until nothing changes.
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            let com_y = a.bodies[i].com()[1];
            for di in 0..4 {
                if !slide[i][di] && touching[i][di].iter().any(|&j| slide[j][di]) {
                    slide[i][di] = true;
                    changed = true;
                }
                if !tip[i][di] && touching[i][di].iter().any(|&j| slide[j][di] && a.bodies[j].max()[1] >= com_y) {
                    tip[i][di] = true;
                    changed = true;
                }
            }
        }
    }
    (slide, tip)
}

/// Quasi-static transport checks (EN 12195-1 method) for every selected case.
pub fn transport(container: &ContainerSpec, specs: &[ItemSpec], opts: &PackOptions, placements: &[Placement]) -> Vec<TransportResult> {
    let phys: &PhysicsOptions = &opts.physics;
    if phys.transport.is_empty() || placements.is_empty() {
        return phys.transport.iter().map(|c| TransportResult { case: c.name.clone(), issues: Vec::new() }).collect();
    }
    let spec_of: HashMap<&str, &ItemSpec> = specs.iter().map(|s| (s.id.as_str(), s)).collect();
    let shapes: Vec<OrientedShape> = placements.iter().map(rebuild_shape).collect();
    let a = analyze(container, &spec_of, opts, placements, &shapes);
    let size = [container.width, container.height, container.depth];
    let (slide_blocked, tip_blocked) = blocking(&a, size, phys.secure_load_end);
    let friction = |p: &Placement| {
        spec_of
            .get(p.item_id.as_str())
            .and_then(|s| s.friction)
            .unwrap_or(phys.default_friction)
            .max(0.0)
    };

    phys.transport
        .iter()
        .map(|case| {
            let mut issues = Vec::new();
            for (i, p) in placements.iter().enumerate() {
                let info = &a.infos[i];
                if info.set.is_empty() {
                    continue;
                }
                // Everything resting on this item moves with it.
                let column_mass = p.mass + a.incoming[i][0];
                let mu = info
                    .set
                    .owners
                    .iter()
                    .filter_map(|o| match o {
                        Support::Item(j) => Some(friction(&placements[*j])),
                        Support::Floor => None,
                    })
                    .fold(friction(p), f64::min);
                let com = a.bodies[i].com();
                let column_y = (effective_mass(p.mass) * com[1] + a.incoming_y[i]) / (effective_mass(p.mass) + a.incoming[i][0]);
                let lever = (column_y - a.bodies[i].min[1]).max(0.0);
                for (di, (dir, d)) in DIRS.iter().enumerate() {
                    let acc = accel(case, *dir);
                    if acc <= 0.0 {
                        continue;
                    }
                    if phys.check_sliding && !slide_blocked[i][di] && acc > mu * case.vertical_min + 1e-9 {
                        issues.push(TransportIssue {
                            item: p.instance_id.clone(),
                            kind: IssueKind::Sliding,
                            direction: Some(*dir),
                            acceleration: acc,
                            required: column_mass * G * (acc - mu * case.vertical_min) / 1000.0,
                        });
                    }
                    let held_round = a.roll[i].is_held();
                    if phys.check_tipping && !tip_blocked[i][di] && !held_round && lever > 0.0 {
                        if let Some(r) = a.resultant[i] {
                            let arm = ray_exit_distance(&info.set.hull, r, *d);
                            if acc * lever > case.vertical_min * arm + 1e-9 {
                                issues.push(TransportIssue {
                                    item: p.instance_id.clone(),
                                    kind: IssueKind::Tipping,
                                    direction: Some(*dir),
                                    acceleration: acc,
                                    required: column_mass * G * (acc - case.vertical_min * arm / lever) / 1000.0,
                                });
                            }
                        }
                    }
                }
                if phys.dynamic_stacking {
                    let capacity = spec_of.get(p.item_id.as_str()).map_or(f64::INFINITY, |s| s.capacity());
                    let dynamic = a.incoming[i][0] * case.vertical_max;
                    if dynamic > capacity + 1e-6 {
                        issues.push(TransportIssue {
                            item: p.instance_id.clone(),
                            kind: IssueKind::StackOverload,
                            direction: None,
                            acceleration: case.vertical_max,
                            required: dynamic - capacity,
                        });
                    }
                }
            }
            TransportResult { case: case.name.clone(), issues }
        })
        .collect()
}

pub fn compute_metrics(container: &ContainerSpec, placements: &[Placement]) -> Metrics {
    let mass: f64 = placements.iter().map(|p| p.mass).sum();
    let volume: f64 = placements.iter().map(|p| p.shape.volume()).sum();
    // Mass-weighted centre of gravity; plain centroid for massless cargo.
    let weight = |p: &Placement| if mass > 0.0 { p.mass / mass } else { 1.0 / placements.len() as f64 };
    let com: [f64; 3] = std::array::from_fn(|k| placements.iter().map(|p| weight(p) * p.center_of_mass[k]).sum());
    let axle_loads = container.axles.map(|[a, b]| {
        let span = b.z - a.z;
        let rb = if span.abs() > 0.0 { mass * (com[2] - a.z) / span } else { mass / 2.0 };
        [mass - rb, rb]
    });
    Metrics {
        item_count: placements.len(),
        total_mass: mass,
        volume_utilization: volume / container.volume(),
        weight_utilization: container.max_payload.map(|m| mass / m),
        center_of_mass: com,
        lateral_offset: (com[0] - container.width / 2.0).abs(),
        axle_loads,
        accessibility: accessibility(placements),
        min_support_margin: placements
            .iter()
            .map(|p| p.support_margin)
            .filter(|m| m.is_finite())
            .fold(f64::INFINITY, f64::min),
    }
}

/// Fraction of stop-tagged items not blocked (in front of them towards the
/// door, or on top of them) by an item of a later stop.
pub fn accessibility(placements: &[Placement]) -> f64 {
    let tagged: Vec<&Placement> = placements.iter().filter(|p| p.stop > 0).collect();
    if tagged.is_empty() {
        return 1.0;
    }
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| a0 < b1 - tol::CONTACT && b0 < a1 - tol::CONTACT;
    let blocked = tagged
        .iter()
        .filter(|i| {
            let (imin, imax) = (i.position, [i.position[0] + i.size[0], i.position[1] + i.size[1], i.position[2] + i.size[2]]);
            placements.iter().any(|j| {
                if (j.stop <= i.stop && j.stop != 0) || std::ptr::eq(**i, j) {
                    return false;
                }
                let (jmin, jmax) = (j.position, [j.position[0] + j.size[0], j.position[1] + j.size[1], j.position[2] + j.size[2]]);
                let x = overlap(imin[0], imax[0], jmin[0], jmax[0]);
                let in_front = x && overlap(imin[1], imax[1], jmin[1], jmax[1]) && jmin[2] >= imax[2] - tol::CONTACT;
                let on_top = x && overlap(imin[2], imax[2], jmin[2], jmax[2]) && jmin[1] >= imax[1] - tol::CONTACT;
                in_front || on_top
            })
        })
        .count();
    1.0 - blocked as f64 / tagged.len() as f64
}
