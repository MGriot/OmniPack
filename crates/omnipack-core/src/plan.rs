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
    /// Round item on a line/point support that must be secured with wedges.
    #[serde(default)]
    pub needs_chocks: bool,
    /// What it takes to secure this unit for the selected transport cases.
    #[serde(default)]
    pub securing: SecuringClass,
    /// Worst transport load on this unit (none without transport cases).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact: Option<Impact>,
    /// Pressure on the container floor under this unit (own weight plus
    /// everything it carries), kg/m². 0 = not on the floor.
    #[serde(default)]
    pub floor_pressure: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// How a unit is held in transport, from least to most effort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecuringClass {
    /// Held by friction, walls and touching neighbours alone.
    #[default]
    Secured,
    /// Held once the listed gaps next to it are filled with dunnage.
    Dunnage,
    /// A round item that must be wedged.
    Chocks,
    /// Needs lashing or extra blocking: a sliding or tipping force remains.
    Lashing,
    /// The dynamic load on top exceeds its stacking limit.
    Overloaded,
}

/// Worst transport load on a unit, over every selected case and direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    /// Force this unit must pass on to whatever blocks it: its own share that
    /// friction cannot hold plus what the units behind push into it, kN.
    pub force_kn: f64,
    /// Demand over the unit's own resistance (friction for sliding, base width
    /// for tipping). Above 1 it depends on blocking or lashing.
    pub ratio: f64,
    pub case: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnpackReason {
    /// No allowed orientation fits inside an empty container.
    TooLarge,
    /// Fits inside, but no allowed orientation passes the door opening.
    DoorTooSmall,
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
    /// Too wide or too tall, in its orientation, for the door opening.
    DoorTooSmall { item: String },
}

/// Direction of an acceleration acting on the cargo. Forward = towards the
/// front wall (`z = 0`), e.g. braking of a vehicle whose door is at the rear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    /// Friction alone cannot hold the item and nothing blocks it.
    Sliding,
    /// The item (with its load) would tip and nothing high enough blocks it.
    Tipping,
    /// The dynamic load on top exceeds the item's stacking limit.
    StackOverload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportIssue {
    pub item: String,
    pub kind: IssueKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// Acceleration of the case in that direction, g (vertical factor for overloads).
    pub acceleration: f64,
    /// Securing force still needed (lashing, blocking), kN. For stack
    /// overloads: the excess load, kg.
    pub required: f64,
    /// Direct lashings of the configured capacity that would provide it
    /// (EN 12195-1). 0 for stack overloads.
    #[serde(default)]
    pub lashings: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportResult {
    pub case: String,
    pub issues: Vec<TransportIssue>,
    /// Gaps that must be filled with dunnage for the blocking to hold.
    #[serde(default)]
    pub gaps: Vec<GapFill>,
}

/// A gap between a unit and a wall (`other` = none) or another unit that is
/// counted as blocking once filled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GapFill {
    pub item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
    /// Direction from `item` towards the gap.
    pub direction: Direction,
    pub gap_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPlan {
    pub id: String,
    pub size: [f64; 3],
    pub placements: Vec<Placement>,
    pub metrics: Metrics,
    /// Independent re-verification of this container. Empty = physically valid.
    pub violations: Vec<Violation>,
    /// Quasi-static transport checks, one entry per selected case.
    #[serde(default)]
    pub transport: Vec<TransportResult>,
    /// Load distribution: CTU Code checks, VGM, vehicle axle loads, floor.
    #[serde(default)]
    pub balance: BalanceReport,
}

/// Where the cargo's weight sits in the container and on the vehicle. The
/// issues are warnings; they do not make the plan invalid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceReport {
    /// The load was moved this far along Z to balance it (+ = towards the door), mm.
    pub shift: f64,
    /// Cargo centre of gravity minus the middle of the length (+ = towards the door), mm.
    pub lengthwise_offset: f64,
    /// Cargo centre of gravity minus the middle of the width (+ = right), mm.
    pub lateral_offset: f64,
    /// Share of the cargo mass between 25% and 75% of the length, 0..1.
    pub central_share: f64,
    /// Share of the cargo mass in the front half and in the door half, 0..1.
    pub half_shares: [f64; 2],
    /// Height of the cargo's centre of gravity over the inside height, 0..1.
    pub cog_height_ratio: f64,
    /// Free length between the load and the front wall, and the door, mm.
    pub end_gaps: [f64; 2],
    /// Verified gross mass (SOLAS VI/2, method 2): tare plus cargo, kg.
    /// Dunnage and lashing material come on top.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vgm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<VehicleLoads>,
    pub issues: Vec<BalanceIssue>,
    /// Sum of the exceedances, in percentage points (0 = every check met).
    pub excess: f64,
}

/// Ground loads of the road vehicle with the container and its cargo, kg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleLoads {
    pub steer: f64,
    pub drive: f64,
    /// Trailer axle group.
    pub trailer: f64,
    pub gross: f64,
    /// Limits in the same order: steer, drive, trailer, gross.
    pub limits: [f64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BalanceIssue {
    /// The centre of gravity is too far towards one end (CTU Code: 5% of the length).
    CogLengthwise { offset: f64, limit: f64 },
    /// The centre of gravity is too far to one side (CTU Code: 5% of the width).
    CogLateral { offset: f64, limit: f64 },
    /// Too little of the mass in the middle half of the length.
    CentralShare { share: f64, min: f64 },
    /// The centre of gravity is too high (CTU Code: below half the height).
    CogHeight { height: f64, limit: f64 },
    /// A vehicle axle (steer, drive, trailer) or the gross mass is over its limit.
    AxleOverload { axle: String, load: f64, max: f64 },
    /// The contact pressure under a unit exceeds the floor rating: spread it
    /// over at least `spread_area` m² with beams.
    FloorPressure { item: String, pressure: f64, limit: f64, spread_area: f64 },
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
