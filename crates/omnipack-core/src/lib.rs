//! OmniPack core: domain model, constructive placer, static physics and an
//! independent plan validator.
//!
//! ```no_run
//! let req: omnipack_core::PackRequest = serde_json::from_str("...").unwrap();
//! let plan = omnipack_core::pack(&req).unwrap();
//! assert!(plan.is_valid());
//! ```

pub mod balance;
pub mod generate;
pub mod grid;
pub mod learn;
pub mod manual;
pub mod model;
pub mod placer;
pub mod plan;
pub mod scene;
pub mod statics;
pub mod validate;

pub use model::*;
pub use plan::*;

use omnipack_geom::{tol, OrientedShape};
use placer::{ContainerState, Instance};
use std::time::Instant;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("container dimensions must be positive and finite")]
    InvalidContainer,
    #[error("item `{0}`: dimensions must be positive and finite")]
    InvalidShape(String),
    #[error("item `{0}`: mass must be non-negative and finite")]
    InvalidMass(String),
    #[error("item `{0}`: the centre of mass must lie inside the item's bounding box")]
    InvalidComOffset(String),
    #[error("duplicate item id `{0}`")]
    DuplicateId(String),
}

fn check(req: &PackRequest) -> Result<(), PackError> {
    let c = &req.container;
    if ![c.width, c.height, c.depth].iter().all(|v| v.is_finite() && *v > 0.0) {
        return Err(PackError::InvalidContainer);
    }
    let mut ids = std::collections::HashSet::new();
    for it in &req.items {
        if !ids.insert(it.id.as_str()) {
            return Err(PackError::DuplicateId(it.id.clone()));
        }
        if !it.shape.is_valid() {
            return Err(PackError::InvalidShape(it.id.clone()));
        }
        if !(it.mass.is_finite() && it.mass >= 0.0) {
            return Err(PackError::InvalidMass(it.id.clone()));
        }
        let (e, c) = (it.shape.local_extents(), it.shape.local_com());
        if !(0..3).all(|k| it.com_offset[k].is_finite() && (c[k] + it.com_offset[k]).abs() <= e[k] / 2.0 + tol::BOUNDS) {
            return Err(PackError::InvalidComOffset(it.id.clone()));
        }
    }
    Ok(())
}

/// Loading constraints of a unit as a sort key (smaller = loaded earlier):
/// delivery stop, zone, then floor-only units first. Any order that keeps this
/// key non-decreasing respects the loading strategy.
pub fn load_group(opts: &PackOptions, i: &Instance) -> (i64, u8, bool) {
    let stop = match (i.stop, opts.stop_order) {
        (0, _) => i64::MIN,
        (s, StopOrder::Lifo) => -(s as i64),
        (s, StopOrder::Fifo) => s as i64,
    };
    let zone = match i.zone {
        Zone::Back => 0,
        Zone::Any => 1,
        Zone::Front => 2,
    };
    (stop, zone, !i.floor_only)
}

/// Expands specs into units in loading order: delivery stops per
/// `options.stop_order` (LIFO: later stops first; FIFO: stop 1 first; stop 0 =
/// stays aboard and goes first either way), back-zone units before front-zone
/// ones, floor-only units first, then `options.priority`.
pub fn default_sequence(req: &PackRequest) -> Vec<Instance> {
    let mut out = Vec::new();
    for (si, it) in req.items.iter().enumerate() {
        let shapes: Vec<OrientedShape> = it
            .orientations(req.options.allow_rotation)
            .into_iter()
            .map(|o| OrientedShape::new(&it.shape, o, it.com_offset))
            .collect();
        for k in 0..it.quantity {
            out.push(Instance {
                spec: si,
                id: format!("{}#{}", it.id, k + 1),
                item_id: it.id.clone(),
                shapes: shapes.clone(),
                mass: it.mass,
                capacity: it.capacity(),
                floor_only: it.floor_only,
                stop: it.stop,
                zone: it.zone,
                volume: it.shape.volume(),
                tip_deficit: shapes.iter().map(|s| placer::tip_deficit(s, &req.options.physics)).collect(),
                orient_pref: None,
            });
        }
    }
    let opts = &req.options;
    let footprint = |i: &Instance| {
        i.shapes.iter().map(|s| s.extents[0] * s.extents[2]).fold(0.0, f64::max)
    };
    let height = |i: &Instance| i.shapes.first().map_or(0.0, |s| s.extents[1]);
    out.sort_by(|a, b| {
        let by_priority = match opts.priority {
            LoadPriority::Volume => b.volume.total_cmp(&a.volume).then(b.mass.total_cmp(&a.mass)),
            LoadPriority::Mass => b.mass.total_cmp(&a.mass).then(b.volume.total_cmp(&a.volume)),
            LoadPriority::BaseArea => footprint(b).total_cmp(&footprint(a)).then(b.mass.total_cmp(&a.mass)),
            LoadPriority::Height => height(b).total_cmp(&height(a)).then(b.volume.total_cmp(&a.volume)),
            LoadPriority::AsListed => std::cmp::Ordering::Equal,
        };
        load_group(opts, a).cmp(&load_group(opts, b)).then(by_priority).then(a.spec.cmp(&b.spec))
    });
    out
}

fn fits_empty(inst: &Instance, c: &ContainerSpec) -> bool {
    inst.shapes.iter().any(|s| fits_inside(s, c))
}

fn fits_inside(s: &OrientedShape, c: &ContainerSpec) -> bool {
    s.extents[0] <= c.width + tol::BOUNDS && s.extents[1] <= c.height + tol::BOUNDS && s.extents[2] <= c.depth + tol::BOUNDS
}

/// Packs the request with the default loading sequence.
pub fn pack(req: &PackRequest) -> Result<PackResult, PackError> {
    check(req)?;
    let seq = default_sequence(req);
    Ok(pack_sequence(req, &seq))
}

/// Packs the units around `fixed` placements (for example placed by hand),
/// which keep their positions in the first container even if they fail the
/// checks. Nothing is stacked on a fixed unit that fails them; the plan's
/// violations report it.
pub fn pack_with_fixed(req: &PackRequest, fixed: &[Placement]) -> Result<PackResult, PackError> {
    check(req)?;
    let seq = default_sequence(req);
    Ok(pack_inner(req, &seq, fixed))
}

/// Packs units in the given order (the optimizer supplies other orders).
pub fn pack_sequence(req: &PackRequest, instances: &[Instance]) -> PackResult {
    pack_inner(req, instances, &[])
}

fn pack_inner(req: &PackRequest, instances: &[Instance], fixed: &[Placement]) -> PackResult {
    let t0 = Instant::now();
    let c = &req.container;
    // Fixed units: (instance, orientation, position), in their loading order.
    let index_of: std::collections::HashMap<&str, usize> = instances.iter().enumerate().map(|(i, x)| (x.id.as_str(), i)).collect();
    let mut by_seq: Vec<&Placement> = fixed.iter().collect();
    by_seq.sort_by_key(|p| p.seq);
    let fixed_units: Vec<(usize, usize, [f64; 3])> = by_seq
        .iter()
        .filter_map(|p| {
            let i = *index_of.get(p.instance_id.as_str())?;
            let o = instances[i].shapes.iter().position(|s| s.orientation == p.orientation)?;
            Some((i, o, p.position))
        })
        .collect();
    let is_fixed: std::collections::HashSet<usize> = fixed_units.iter().map(|f| f.0).collect();
    let mut unpacked = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    for (i, inst) in instances.iter().enumerate() {
        if is_fixed.contains(&i) {
            continue;
        }
        let reason = if inst.shapes.is_empty() {
            Some(UnpackReason::NoOrientation)
        } else if !fits_empty(inst, c) {
            Some(UnpackReason::TooLarge)
        } else if !inst.shapes.iter().any(|s| fits_inside(s, c) && validate::passes_door(c, s)) {
            Some(UnpackReason::DoorTooSmall)
        } else if c.max_payload.is_some_and(|m| inst.mass > m) {
            Some(UnpackReason::TooHeavy)
        } else {
            None
        };
        match reason {
            Some(r) => unpacked.push(Unpacked { instance_id: inst.id.clone(), item_id: inst.item_id.clone(), reason: r }),
            None => pending.push(i),
        }
    }

    let mut containers = Vec::new();
    while !pending.is_empty() || (containers.is_empty() && !fixed_units.is_empty()) {
        if containers.len() as u32 >= req.options.max_containers {
            for &i in &pending {
                unpacked.push(Unpacked {
                    instance_id: instances[i].id.clone(),
                    item_id: instances[i].item_id.clone(),
                    reason: UnpackReason::ContainerLimit,
                });
            }
            break;
        }
        let mut state = ContainerState::new(c, &req.options, instances);
        let with_fixed = containers.is_empty() && !fixed_units.is_empty();
        if with_fixed {
            for &(i, o, min) in &fixed_units {
                state.force_place(i, o, min);
            }
        }
        let mut left = Vec::new();
        // A spec that failed stays failed until something new is placed.
        let mut failed_at: std::collections::HashMap<usize, usize> = Default::default();
        for &i in &pending {
            let spec = instances[i].spec;
            if failed_at.get(&spec) == Some(&state.placed.len()) {
                left.push(i);
                continue;
            }
            if !state.try_place(i) {
                failed_at.insert(spec, state.placed.len());
                left.push(i);
            }
        }
        if state.placed.is_empty() {
            for &i in &left {
                unpacked.push(Unpacked {
                    instance_id: instances[i].id.clone(),
                    item_id: instances[i].item_id.clone(),
                    reason: UnpackReason::NoStablePosition,
                });
            }
            break;
        }
        // Hand-placed units keep their positions: no lengthwise centring.
        containers.push(finish_container(req, instances, &state, containers.len(), !with_fixed));
        pending = left;
    }

    let packed_units: usize = containers.iter().map(|c: &ContainerPlan| c.placements.len()).sum();
    let used_volume: f64 = containers.len() as f64 * c.volume();
    let packed_volume: f64 = containers.iter().flat_map(|c| &c.placements).map(|p| p.shape.volume()).sum();
    PackResult {
        schema: PLAN_SCHEMA.to_string(),
        containers,
        unpacked,
        requested_units: instances.len(),
        packed_units,
        volume_utilization: if used_volume > 0.0 { packed_volume / used_volume } else { 0.0 },
        elapsed_ms: t0.elapsed().as_millis() as u64,
    }
}

fn placements_of(req: &PackRequest, instances: &[Instance], state: &ContainerState) -> Vec<Placement> {
    state
        .placed
        .iter()
        .enumerate()
        .map(|(seq, p)| {
            let inst = &instances[p.inst];
            let shape = &inst.shapes[p.orient];
            let spec = &req.items[inst.spec];
            Placement {
                instance_id: inst.id.clone(),
                item_id: inst.item_id.clone(),
                seq,
                shape: shape.shape.clone(),
                orientation: shape.orientation,
                position: p.min,
                size: shape.extents,
                center_of_mass: std::array::from_fn(|k| p.min[k] + shape.com_from_min[k]),
                mass: inst.mass,
                load_on_top: p.incoming[0],
                support_margin: p.margin,
                stop: inst.stop,
                needs_chocks: p.roll == scene::Roll::NeedsChocks,
                securing: SecuringClass::Secured,
                impact: None,
                color: spec.color.clone(),
                floor_pressure: 0.0,
            }
        })
        .collect()
}

/// Lengthwise centring (if enabled) plus the full check of the result. Exact
/// contact queries are only translation-invariant up to rounding, so the
/// moved load is rebuilt unit by unit through the placer's own checks (every
/// loading step stays verified) and kept only if it then validates cleanly.
fn centre(req: &PackRequest, instances: &[Instance], state: &ContainerState, allow_shift: bool) -> (f64, Vec<Placement>, validate::PlanCheck) {
    let (c, opts) = (&req.container, &req.options);
    let placements = placements_of(req, instances, state);
    if opts.balance.centre_lengthwise && allow_shift {
        let mut moved = placements.clone();
        let shift = balance::centre_lengthwise(c, opts, &mut moved);
        if shift != 0.0 {
            let mut again = ContainerState::new(c, opts, instances);
            if state.placed.iter().all(|p| again.place_at(p.inst, p.orient, [p.min[0], p.min[1], p.min[2] + shift])) {
                let moved = placements_of(req, instances, &again);
                let check = validate::check_plan(c, &req.items, opts, &moved);
                if check.violations.is_empty() {
                    return (shift, moved, check);
                }
            }
        }
    }
    let check = validate::check_plan(c, &req.items, opts, &placements);
    (0.0, placements, check)
}

fn finish_container(req: &PackRequest, instances: &[Instance], state: &ContainerState, index: usize, allow_shift: bool) -> ContainerPlan {
    let (shift, placements, check) = centre(req, instances, state, allow_shift);
    build_plan(req, placements, shift, check, index)
}

/// The finished container: metrics, violations, transport and balance.
pub(crate) fn build_plan(req: &PackRequest, mut placements: Vec<Placement>, shift: f64, check: validate::PlanCheck, index: usize) -> ContainerPlan {
    let c = &req.container;
    let metrics = validate::compute_metrics(c, &placements);
    let report = check.transport;
    let mut floor_load = Vec::with_capacity(placements.len());
    for (((p, s), im), fp) in placements.iter_mut().zip(report.securing).zip(report.impact).zip(check.floor_pressure) {
        p.securing = s;
        p.impact = im;
        p.floor_pressure = fp.pressure;
        floor_load.push(fp.load);
    }
    let balance = balance::report(c, &req.options, &placements, &metrics, &floor_load, shift);
    ContainerPlan {
        id: format!("{}-{}", c.id, index + 1),
        size: [c.width, c.height, c.depth],
        placements,
        metrics,
        violations: check.violations,
        transport: report.results,
        balance,
    }
}
