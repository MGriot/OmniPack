//! Input model: what to pack, where, and how.

use omnipack_geom::{Orientation, Shape};
use serde::{Deserialize, Serialize};

fn one() -> u32 {
    1
}

/// Where in the container an item prefers to go, along the door axis (Z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    #[default]
    Any,
    /// Towards the far wall (`z = 0`): loaded first, unloaded last.
    Back,
    /// Towards the door (`z = depth`): loaded last, unloaded first.
    Front,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSpec {
    pub id: String,
    pub shape: Shape,
    /// Mass of one unit, kg.
    pub mass: f64,
    #[serde(default = "one")]
    pub quantity: u32,
    /// Maximum mass (kg) that may rest on this item, including everything stacked
    /// above it. `None` = unlimited.
    #[serde(default)]
    pub max_load_on_top: Option<f64>,
    /// Nothing may be placed on top.
    #[serde(default)]
    pub fragile: bool,
    /// Must stand on the container floor.
    #[serde(default)]
    pub floor_only: bool,
    /// Local height axis must stay vertical ("this side up").
    #[serde(default)]
    pub upright_only: bool,
    /// Explicit whitelist of orientations. `None` = all allowed.
    #[serde(default)]
    pub allowed_orientations: Option<Vec<Orientation>>,
    /// Delivery stop, 1 = unloaded first. 0 = no stop constraint.
    #[serde(default)]
    pub stop: u32,
    #[serde(default)]
    pub zone: Zone,
    /// Centre-of-mass offset from the geometric centre, local frame, mm.
    #[serde(default)]
    pub com_offset: [f64; 3],
    #[serde(default)]
    pub color: Option<String>,
    /// Friction coefficient against its support. `None` = the physics default.
    #[serde(default)]
    pub friction: Option<f64>,
}

impl ItemSpec {
    /// Load capacity on top, kg.
    pub fn capacity(&self) -> f64 {
        if self.fragile {
            0.0
        } else {
            self.max_load_on_top.unwrap_or(f64::INFINITY)
        }
    }

    pub fn orientations(&self, allow_rotation: bool) -> Vec<Orientation> {
        self.shape
            .distinct_orientations()
            .into_iter()
            .filter(|o| allow_rotation || *o == Orientation::Whd)
            .filter(|o| !self.upright_only || o.keeps_upright())
            .filter(|o| self.allowed_orientations.as_ref().is_none_or(|a| a.contains(o)))
            .collect()
    }
}

/// One axle (or kingpin) of the vehicle carrying the container.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Axle {
    /// Position along the container's Z axis, mm (may lie outside the box).
    pub z: f64,
    /// Maximum payload share this axle may carry, kg.
    pub max_load: f64,
}

/// Where the combined centre of gravity of the cargo may lie.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CogLimits {
    /// Max |com.x - width/2|, mm.
    #[serde(default)]
    pub max_lateral_offset: Option<f64>,
    #[serde(default)]
    pub z_min: Option<f64>,
    #[serde(default)]
    pub z_max: Option<f64>,
    #[serde(default)]
    pub max_height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    #[serde(default)]
    pub max_payload: Option<f64>,
    /// Exactly two axles: the lever rule splits the payload between them.
    #[serde(default)]
    pub axles: Option<[Axle; 2]>,
    #[serde(default)]
    pub cog_limits: CogLimits,
}

impl ContainerSpec {
    pub fn volume(&self) -> f64 {
        self.width * self.height * self.depth
    }
}

/// Order in which the container is filled. Each variant is a strict priority of
/// the three axes; see `docs/conventions.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillBias {
    /// Complete vertical walls across the width, from the back wall to the door.
    #[default]
    WallBuilding,
    /// Complete floor layers, rows across the width from the back.
    FloorFirst,
    /// Complete vertical walls along the length, from the left side to the right.
    Longitudinal,
    /// Complete floor layers, rows along the length from the left side.
    Lateral,
    /// Grow outward from the back-left-bottom corner.
    CornerFirst,
}

impl FillBias {
    /// Weights on normalised (x, y, z).
    pub fn weights(self) -> [f64; 3] {
        const A: f64 = 1.0;
        const B: f64 = 1e-2;
        const C: f64 = 1e-4;
        match self {
            FillBias::WallBuilding => [C, B, A],
            FillBias::FloorFirst => [C, A, B],
            FillBias::Longitudinal => [A, B, C],
            FillBias::Lateral => [B, A, C],
            FillBias::CornerFirst => [A, A, A],
        }
    }
}

/// Order in which delivery stops are loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopOrder {
    /// Last in, first out (rear-door vehicles): later stops are loaded first,
    /// deep inside, and the first stop ends up at the door.
    #[default]
    Lifo,
    /// First in, first out (side-loading, drive-through): the first stop is
    /// loaded first and placed nearest the unloading door; filling runs from
    /// the door towards the back.
    Fifo,
}

/// Which units go first within the same stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadPriority {
    /// Largest volume first (usually the best fill).
    #[default]
    Volume,
    /// Heaviest first (keeps the centre of gravity low).
    Mass,
    /// Largest footprint first (stable floor layers).
    BaseArea,
    /// Tallest first.
    Height,
    /// In the order the items are listed, all units of an item together.
    AsListed,
}

/// Quasi-static acceleration case (in g) as used by EN 12195-1 and the
/// IMO/ILO/UNECE CTU Code. Forward = towards the front wall (`z = 0`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportCase {
    pub name: String,
    pub forward: f64,
    pub backward: f64,
    pub sideways: f64,
    /// Smallest vertical factor (reduces friction), e.g. 1.0 on roads, 0.5 at sea.
    pub vertical_min: f64,
    /// Largest vertical factor (dynamic load on stacks).
    pub vertical_max: f64,
}

impl TransportCase {
    fn new(name: &str, forward: f64, backward: f64, sideways: f64, vertical_min: f64, vertical_max: f64) -> Self {
        TransportCase { name: name.into(), forward, backward, sideways, vertical_min, vertical_max }
    }

    /// Built-in profiles. Values follow EN 12195-1:2010 (road) and the CTU
    /// Code 2014 annex 5 (rail, sea areas A/B/C); check the rules that apply
    /// to your transport.
    pub fn presets() -> Vec<TransportCase> {
        vec![
            TransportCase::new("Road (EN 12195-1)", 0.8, 0.5, 0.5, 1.0, 1.0),
            TransportCase::new("Rail, combined transport", 0.5, 0.5, 0.5, 0.7, 1.3),
            TransportCase::new("Rail wagon, shunting impacts", 1.0, 1.0, 0.5, 0.7, 1.3),
            TransportCase::new("Sea area A (Baltic, sheltered)", 0.3, 0.3, 0.5, 0.5, 1.5),
            TransportCase::new("Sea area B (North Sea, Med.)", 0.3, 0.3, 0.7, 0.3, 1.7),
            TransportCase::new("Sea area C (unrestricted)", 0.4, 0.4, 0.8, 0.2, 1.8),
        ]
    }
}

/// Which physics the plan is checked against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsOptions {
    /// Transport accelerations to check (empty = static loading only).
    pub transport: Vec<TransportCase>,
    /// Report items that would slide unless blocked or lashed.
    pub check_sliding: bool,
    /// Report items that would tip unless blocked or lashed.
    pub check_tipping: bool,
    /// Multiply loads on top by the vertical factor when checking stack limits.
    pub dynamic_stacking: bool,
    /// Friction coefficient for items that do not set their own.
    pub default_friction: f64,
    /// Round items resting on a line or point (lying drums, balls) are held by
    /// wedges. Off: they must be wedged in by walls or neighbours.
    pub use_chocks: bool,
    /// The open face of a partial load is closed with a locking bar, gate or
    /// dunnage, which blocks like a wall.
    pub secure_load_end: bool,
    /// Gaps up to this size (mm) between an item and a wall or a neighbour are
    /// filled with dunnage or airbags and then block like direct contact. They
    /// are listed in the transport report. 0 = only touching faces block.
    pub max_fill_gap: f64,
    /// Friction between cargo and the container floor. `None` = the item's own
    /// value (or `default_friction`).
    pub floor_friction: Option<f64>,
    /// Anti-slip mats under every item and between layers: every contact has at
    /// least μ = [`ANTI_SLIP_FRICTION`].
    pub anti_slip_mats: bool,
}

/// Friction of rubber anti-slip mats (EN 12195-1 Annex B, typical value).
pub const ANTI_SLIP_FRICTION: f64 = 0.6;

impl Default for PhysicsOptions {
    fn default() -> Self {
        PhysicsOptions {
            transport: vec![TransportCase::presets().remove(0)],
            check_sliding: true,
            check_tipping: true,
            dynamic_stacking: false,
            default_friction: 0.4,
            use_chocks: true,
            secure_load_end: true,
            max_fill_gap: 50.0,
            floor_friction: None,
            anti_slip_mats: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PackOptions {
    pub bias: FillBias,
    pub stop_order: StopOrder,
    pub priority: LoadPriority,
    pub physics: PhysicsOptions,
    /// Required distance from the centre of gravity (or load resultant) to the
    /// edge of the support polygon, as a fraction of the item's smaller
    /// half-footprint. 0 = merely not tipping; 0.5 = a lot of safety.
    pub stability_margin: f64,
    /// Minimum fraction of a flat bottom face that must be in contact.
    pub min_support_ratio: f64,
    /// 0..1: how strongly to keep the centre of gravity near the centreline.
    pub balance_weight: f64,
    pub allow_rotation: bool,
    pub max_containers: u32,
    /// Stop after checking this many ranked candidates per item.
    pub max_stability_checks: usize,
    pub seed: u64,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            bias: FillBias::default(),
            stop_order: StopOrder::default(),
            priority: LoadPriority::default(),
            physics: PhysicsOptions::default(),
            stability_margin: 0.1,
            min_support_ratio: 0.5,
            balance_weight: 0.3,
            allow_rotation: true,
            max_containers: 50,
            max_stability_checks: 5000,
            seed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRequest {
    pub container: ContainerSpec,
    pub items: Vec<ItemSpec>,
    #[serde(default)]
    pub options: PackOptions,
}
