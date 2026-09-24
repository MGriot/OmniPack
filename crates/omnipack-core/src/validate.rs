//! Independent verification of a finished container plan.
//!
//! Recomputes everything from the placement records alone (no placer state):
//! bounds, exact interpenetration, supports in loading order, support area,
//! own and stacked centre-of-gravity margins, full load propagation to the
//! floor, container payload, axle loads and centre-of-gravity limits.

use crate::model::{ContainerSpec, ItemSpec, PackOptions};
use crate::plan::{Metrics, Placement, Violation};
use crate::scene::{self, compute_supports};
use crate::statics::{add, effective_mass, load_at, resultant, Load, Support};
use omnipack_geom::{overlaps, tol, Body, OrientedShape, Pt2};
use std::collections::HashMap;

pub fn rebuild_shape(p: &Placement) -> OrientedShape {
    // com_offset is recovered from the recorded centre of mass.
    let base = OrientedShape::new(&p.shape, p.orientation, [0.0; 3]);
    let mut s = base;
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

pub fn validate(
    container: &ContainerSpec,
    specs: &[ItemSpec],
    opts: &PackOptions,
    placements: &[Placement],
) -> Vec<Violation> {
    let mut v = Vec::new();
    let spec_of: HashMap<&str, &ItemSpec> = specs.iter().map(|s| (s.id.as_str(), s)).collect();
    let shapes: Vec<OrientedShape> = placements.iter().map(rebuild_shape).collect();
    let bodies: Vec<Body> = placements.iter().zip(&shapes).map(|(p, s)| Body::new(s, p.position)).collect();
    let size = [container.width, container.height, container.depth];
    let n = placements.len();

    // Candidate pairs by sweeping along x.
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

    for (i, (p, b)) in placements.iter().zip(&bodies).enumerate() {
        let mx = b.max();
        if (0..3).any(|k| b.min[k] < -tol::BOUNDS || mx[k] > size[k] + tol::BOUNDS) {
            v.push(Violation::OutOfBounds { item: p.instance_id.clone() });
        }
        for &j in &near[i] {
            if j > i && overlaps(b, &bodies[j]) {
                v.push(Violation::Overlap { a: p.instance_id.clone(), b: placements[j].instance_id.clone() });
            }
        }
        if let Some(spec) = spec_of.get(p.item_id.as_str()) {
            let allowed = spec.orientations(opts.allow_rotation);
            if !allowed.contains(&p.orientation) {
                v.push(Violation::OrientationNotAllowed { item: p.instance_id.clone() });
            }
            if spec.floor_only && b.min[1] > tol::CONTACT {
                v.push(Violation::NotOnFloor { item: p.instance_id.clone() });
            }
        }
    }

    // Supports as they existed when each item was loaded (earlier seq only).
    let infos: Vec<_> = (0..n)
        .map(|i| {
            let earlier = near[i].iter().copied().filter(|&j| placements[j].seq < placements[i].seq);
            compute_supports(&bodies[i], earlier.map(|j| (j, bodies[j])))
        })
        .collect();
    let chocked: Vec<bool> = (0..n)
        .map(|i| {
            scene::is_lying_cylinder(&bodies[i])
                && infos[i].set.is_degenerate()
                && scene::roll_blocked(&bodies[i], near[i].iter().map(|&j| bodies[j]), size)
        })
        .collect();
    let required: Vec<f64> = bodies.iter().map(|b| scene::required_margin(b, opts.stability_margin)).collect();

    for i in 0..n {
        let id = &placements[i].instance_id;
        let info = &infos[i];
        if info.set.is_empty() {
            v.push(Violation::Unsupported { item: id.clone() });
            continue;
        }
        if let Some(bottom) = scene::flat_bottom_area(&bodies[i]) {
            let ratio = info.contact_area / bottom;
            if ratio < opts.min_support_ratio - 1e-6 {
                v.push(Violation::InsufficientSupportArea { item: id.clone(), ratio, required: opts.min_support_ratio });
            }
        }
        if scene::is_lying_cylinder(&bodies[i]) && info.set.is_degenerate() && !chocked[i] {
            v.push(Violation::MayRoll { item: id.clone() });
        }
        let com = bodies[i].com();
        let m = scene::stability_margin(&info.set, Pt2::new(com[0], com[2]), chocked[i]);
        if m < required[i] - 1e-6 {
            v.push(Violation::Unstable { item: id.clone(), margin: m, required: required[i] });
        }
    }

    // Full load propagation, top to bottom.
    let mut incoming: Vec<Load> = vec![[0.0; 3]; n];
    let mut by_height: Vec<usize> = (0..n).collect();
    by_height.sort_by(|&a, &b| bodies[b].min[1].total_cmp(&bodies[a].min[1]));
    for &i in &by_height {
        let p = &placements[i];
        let info = &infos[i];
        if info.set.is_empty() {
            continue;
        }
        let capacity = spec_of.get(p.item_id.as_str()).map_or(f64::INFINITY, |s| s.capacity());
        if incoming[i][0] > capacity + 1e-6 {
            v.push(Violation::Overloaded { item: p.instance_id.clone(), load: incoming[i][0], capacity });
        }
        let com = bodies[i].com();
        let total = add(load_at(effective_mass(p.mass), Pt2::new(com[0], com[2])), incoming[i]);
        let Some(r) = resultant(total) else { continue };
        let m = scene::stability_margin(&info.set, r, chocked[i]);
        if m < required[i] - 1e-6 {
            v.push(Violation::Unstable { item: p.instance_id.clone(), margin: m, required: required[i] });
        }
        if let Some(shares) = info.set.distribute(total) {
            for (s, l) in shares {
                if let Support::Item(j) = s {
                    incoming[j] = add(incoming[j], l);
                }
            }
        }
    }

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
        min_support_margin: placements.iter().map(|p| p.support_margin).fold(f64::INFINITY, f64::min),
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
