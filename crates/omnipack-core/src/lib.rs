//! OmniPack core: domain model, constructive placer, static physics and an
//! independent plan validator.
//!
//! ```no_run
//! let req: omnipack_core::PackRequest = serde_json::from_str("...").unwrap();
//! let plan = omnipack_core::pack(&req).unwrap();
//! assert!(plan.is_valid());
//! ```

pub mod generate;
pub mod grid;
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
    inst.shapes.iter().any(|s| {
        s.extents[0] <= c.width + tol::BOUNDS && s.extents[1] <= c.height + tol::BOUNDS && s.extents[2] <= c.depth + tol::BOUNDS
    })
}

/// Packs the request with the default loading sequence.
pub fn pack(req: &PackRequest) -> Result<PackResult, PackError> {
    check(req)?;
    let seq = default_sequence(req);
    Ok(pack_sequence(req, &seq))
}

/// Packs units in the given order (the optimizer supplies other orders).
pub fn pack_sequence(req: &PackRequest, instances: &[Instance]) -> PackResult {
    let t0 = Instant::now();
    let c = &req.container;
    let mut unpacked = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    for (i, inst) in instances.iter().enumerate() {
        let reason = if inst.shapes.is_empty() {
            Some(UnpackReason::NoOrientation)
        } else if !fits_empty(inst, c) {
            Some(UnpackReason::TooLarge)
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
    while !pending.is_empty() {
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
        containers.push(finish_container(req, instances, &state, containers.len()));
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

fn finish_container(req: &PackRequest, instances: &[Instance], state: &ContainerState, index: usize) -> ContainerPlan {
    let c = &req.container;
    let placements: Vec<Placement> = state
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
            }
        })
        .collect();
    let mut placements = placements;
    let metrics = validate::compute_metrics(c, &placements);
    let violations = validate::validate(c, &req.items, &req.options, &placements);
    let report = validate::transport_report(c, &req.items, &req.options, &placements);
    for ((p, s), im) in placements.iter_mut().zip(report.securing).zip(report.impact) {
        p.securing = s;
        p.impact = im;
    }
    let transport = report.results;
    ContainerPlan {
        id: format!("{}-{}", c.id, index + 1),
        size: [c.width, c.height, c.depth],
        placements,
        metrics,
        violations,
        transport,
    }
}
