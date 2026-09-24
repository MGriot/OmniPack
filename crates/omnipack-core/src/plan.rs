//! Output model: a load plan and its verification report.

use omnipack_geom::{Orientation, Shape};
use serde::{Deserialize, Serialize};

pub const PLAN_SCHEMA: &str = "omnipack.plan/1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    /// Unique per unit, e.g. `"crate#3"`.
    pub instance_id: String,
    pub item_id: String,
    /// Loading order within the container, from 0.
    pub seq: usize,
    pub shape: Shape,
    pub orientation: Orientation,
    /// Minimum corner of the world-space AABB, mm.
    pub position: [f64; 3],
    /// World-space AABB size, mm.
    pub size: [f64; 3],
    pub center_of_mass: [f64; 3],
    pub mass: f64,
    /// Mass resting on this item (everything above, transmitted), kg.
    pub load_on_top: f64,
    /// Distance from the load resultant to the edge of the support polygon, mm.
    pub support_margin: f64,
    pub stop: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnpackReason {
    /// No allowed orientation fits inside an empty container.
    TooLarge,
    /// Heavier than the container's max payload.
    TooHeavy,
    /// No allowed orientation exists (constraints exclude all).
    NoOrientation,
    /// Fits geometrically but no stable, load-safe position was found.
    NoStablePosition,
    /// `max_containers` was reached.
    ContainerLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unpacked {
    pub instance_id: String,
    pub item_id: String,
    pub reason: UnpackReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metrics {
    pub item_count: usize,
    pub total_mass: f64,
    /// Sum of true item volumes / container volume, 0..1.
    pub volume_utilization: f64,
    pub weight_utilization: Option<f64>,
    pub center_of_mass: [f64; 3],
    /// |com.x - width/2|, mm.
    pub lateral_offset: f64,
    pub axle_loads: Option<[f64; 2]>,
    /// Fraction of items that can be unloaded at their stop without moving
    /// an item of a later stop, 0..1.
    pub accessibility: f64,
    /// Smallest support margin of any item, mm.
    pub min_support_margin: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Violation {
    OutOfBounds { item: String },
    Overlap { a: String, b: String, },
    Unsupported { item: String },
    Unstable { item: String, margin: f64, required: f64 },
    InsufficientSupportArea { item: String, ratio: f64, required: f64 },
    MayRoll { item: String },
    Overloaded { item: String, load: f64, capacity: f64 },
    NotOnFloor { item: String },
    PayloadExceeded { mass: f64, max: f64 },
    AxleOverloaded { axle: usize, load: f64, max: f64 },
    CogOutOfLimits { detail: String },
    OrientationNotAllowed { item: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPlan {
    pub id: String,
    pub size: [f64; 3],
    pub placements: Vec<Placement>,
    pub metrics: Metrics,
    /// Independent re-verification of this container. Empty = physically valid.
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackResult {
    pub schema: String,
    pub containers: Vec<ContainerPlan>,
    pub unpacked: Vec<Unpacked>,
    pub requested_units: usize,
    pub packed_units: usize,
    /// Sum of packed volumes / sum of used container volumes, 0..1.
    pub volume_utilization: f64,
    pub elapsed_ms: u64,
}

impl PackResult {
    pub fn is_valid(&self) -> bool {
        self.containers.iter().all(|c| c.violations.is_empty())
    }
}
