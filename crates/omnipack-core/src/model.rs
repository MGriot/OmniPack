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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PackOptions {
    pub bias: FillBias,
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
            stability_margin: 0.1,
            min_support_ratio: 0.5,
            balance_weight: 0.3,
            allow_rotation: true,
            max_containers: 50,
            max_stability_checks: 400,
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
