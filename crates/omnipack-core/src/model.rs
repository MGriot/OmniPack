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
    /// Clear door opening (width, height), mm. Units are loaded through it
    /// along Z, so their X and Y extents must pass. `None` = no door limit.
    #[serde(default)]
    pub door: Option<[f64; 2]>,
    /// Empty mass of the container (CSC plate), kg: verified gross mass and
    /// vehicle axle loads.
    #[serde(default)]
    pub tare_mass: Option<f64>,
    /// Floor rating, kg/m²: heavier contact pressure needs load-spreading
    /// beams (ISO 1496-1).
    #[serde(default)]
    pub floor_rating: Option<f64>,
    /// Road vehicle carrying the container, for axle loads with tares.
    #[serde(default)]
    pub vehicle: Option<RoadVehicle>,
}

impl ContainerSpec {
    pub fn volume(&self) -> f64 {
        self.width * self.height * self.depth
    }

    /// A plain box with no limits.
    pub fn new(id: &str, width: f64, height: f64, depth: f64) -> Self {
        ContainerSpec {
            id: id.into(),
            width,
            height,
            depth,
            max_payload: None,
            axles: None,
            cog_limits: CogLimits::default(),
            door: None,
            tare_mass: None,
            floor_rating: None,
            vehicle: None,
        }
    }

    /// Typical ISO containers (inside sizes, door opening, tare, payload for a
    /// 30 480 kg max gross mass) and a road semi-trailer. Check the CSC plate
    /// of the actual unit.
    pub fn presets() -> Vec<ContainerSpec> {
        let iso = |id: &str, h: f64, d: f64, door_h: f64, tare: f64| ContainerSpec {
            max_payload: Some(30_480.0 - tare),
            door: Some([2340.0, door_h]),
            tare_mass: Some(tare),
            floor_rating: Some(ISO_FLOOR_RATING),
            ..ContainerSpec::new(id, 2352.0, h, d)
        };
        vec![
            iso("20ft-dv", 2393.0, 5898.0, 2280.0, 2230.0),
            iso("40ft-dv", 2393.0, 12_032.0, 2280.0, 3750.0),
            iso("40ft-hc", 2698.0, 12_032.0, 2585.0, 3900.0),
            ContainerSpec { max_payload: Some(24_000.0), ..ContainerSpec::new("semi-trailer-13.6", 2480.0, 2700.0, 13_600.0) },
        ]
    }
}

/// Floor rating used for the ISO container presets, kg/m².
pub const ISO_FLOOR_RATING: f64 = 2500.0;

/// Tractor + semi-trailer carrying the container. Distances in mm along the
/// vehicle (rearwards positive), masses in kg. The container's front wall
/// (`z = 0`) faces the tractor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadVehicle {
    pub name: String,
    /// Kingpin → container front wall; positive = the wall is behind the kingpin.
    pub container_front: f64,
    /// Kingpin → centre of the trailer axle group (L_semi).
    pub trailer_wheelbase: f64,
    pub trailer_tare: f64,
    /// Kingpin → centre of gravity of the empty trailer.
    pub trailer_cog: f64,
    pub tractor_tare: f64,
    /// Front (steer) axle → drive axle.
    pub tractor_wheelbase: f64,
    /// Front axle → centre of gravity of the tractor.
    pub tractor_cog: f64,
    /// Front axle → kingpin (fifth wheel), d_ant.
    pub fifth_wheel: f64,
    pub max_steer: f64,
    pub max_drive: f64,
    pub max_trailer: f64,
    pub max_gross: f64,
}

impl RoadVehicle {
    /// A 4×2 tractor with a tri-axle container chassis; EU limits of Directive
    /// 96/53/EC as amended by 2015/719 (44 t for intermodal ISO containers).
    pub fn presets() -> Vec<RoadVehicle> {
        let base = |name: &str, container_front: f64| RoadVehicle {
            name: name.into(),
            container_front,
            trailer_wheelbase: 7700.0,
            trailer_tare: 4800.0,
            trailer_cog: 5000.0,
            tractor_tare: 7500.0,
            tractor_wheelbase: 3800.0,
            tractor_cog: 1100.0,
            fifth_wheel: 3200.0,
            max_steer: 10_000.0,
            max_drive: 11_500.0,
            max_trailer: 24_000.0,
            max_gross: 44_000.0,
        };
        vec![base("Tractor 4x2 + 3-axle chassis, 40 ft", -1000.0), base("Tractor 4x2 + 3-axle chassis, 20 ft centred", 2070.0)]
    }

    /// Ground loads (kg) of the steer axle, drive axle, trailer axle group and
    /// the whole vehicle, for `mass` kg in the container with its centre of
    /// gravity at `cog_z` (container Z, mm).
    pub fn axle_loads(&self, mass: f64, cog_z: f64) -> [f64; 4] {
        let x = self.container_front + cog_z;
        let span = if self.trailer_wheelbase.abs() > 0.0 { self.trailer_wheelbase } else { 1.0 };
        let trailer = (mass * x + self.trailer_tare * self.trailer_cog) / span;
        let kingpin = mass + self.trailer_tare - trailer;
        let wheelbase = if self.tractor_wheelbase.abs() > 0.0 { self.tractor_wheelbase } else { 1.0 };
        let drive = (kingpin * self.fifth_wheel + self.tractor_tare * self.tractor_cog) / wheelbase;
        let steer = kingpin + self.tractor_tare - drive;
        [steer, drive, trailer, mass + self.trailer_tare + self.tractor_tare]
    }

    pub fn limits(&self) -> [f64; 4] {
        [self.max_steer, self.max_drive, self.max_trailer, self.max_gross]
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
    /// Score positions with the weights learned from saved plans
    /// ([`PackOptions::ranker`]); without a ranker, as `WallBuilding`.
    Learned,
}

impl FillBias {
    /// Weights on normalised (x, y, z).
    pub fn weights(self) -> [f64; 3] {
        const A: f64 = 1.0;
        const B: f64 = 1e-2;
        const C: f64 = 1e-4;
        match self {
            FillBias::WallBuilding | FillBias::Learned => [C, B, A],
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
            // Vertical 1.0 for friction (EN 12195-1); ±0.3 g road vibration on stacks.
            TransportCase::new("Road (EN 12195-1)", 0.8, 0.5, 0.5, 1.0, 1.3),
            TransportCase::new("Rail, combined transport", 0.5, 0.5, 0.5, 0.7, 1.3),
            TransportCase::new("Rail wagon, shunting impacts", 1.0, 1.0, 0.5, 0.7, 1.3),
            TransportCase::new("Sea area A (Baltic, sheltered)", 0.3, 0.3, 0.5, 0.5, 1.5),
            TransportCase::new("Sea area B (North Sea, Med.)", 0.3, 0.3, 0.7, 0.3, 1.7),
            TransportCase::new("Sea area C (unrestricted)", 0.4, 0.4, 0.8, 0.2, 1.8),
        ]
    }

    /// Accelerations at a stowage position from the ship's roll and pitch
    /// motion (rigid-body kinematics, harmonic motion at the amplitudes):
    /// - roll period `T_r = 2·c·B / √GM`;
    /// - sideways: gravity component `sin θ_r` plus the tangential term
    ///   `(2π/T_r)²·z·sin θ_r / g`;
    /// - fore and aft: surge plus `sin θ_p` plus `(2π/T_p)²·θ_p·z / g`;
    /// - vertical: `1 ± (heave + (2π/T_p)²·θ_p·|x| / g)`.
    pub fn from_ship(m: &ShipMotion) -> ShipCase {
        const G: f64 = 9.81;
        let gm = m.gm.max(1e-3);
        let roll_period = 2.0 * m.roll_coeff * m.beam / gm.sqrt();
        let wr = if roll_period > 0.0 { 2.0 * std::f64::consts::PI / roll_period } else { 0.0 };
        let wp = if m.pitch_period > 0.0 { 2.0 * std::f64::consts::PI / m.pitch_period } else { 0.0 };
        let (roll, pitch) = (m.roll_deg.to_radians(), m.pitch_deg.to_radians());
        let sideways = roll.sin() + wr * wr * m.height * roll.sin() / G;
        let fore_aft = m.surge_g + pitch.sin() + wp * wp * pitch * m.height / G;
        let vertical = m.heave_g + wp * wp * pitch * m.from_midship.abs() / G;
        let r2 = |v: f64| (v * 100.0).round() / 100.0;
        let mut notes = Vec::new();
        if m.gm < 0.15 {
            notes.push("tender ship: GM below the 0.15 m minimum of the IMO Intact Stability Code".to_string());
        }
        if roll_period < 10.0 {
            notes.push("stiff ship: short roll period, high transverse accelerations high up on deck".to_string());
        }
        let case = TransportCase::new(
            &format!("Sea, ship motion (roll {:.0}° / {:.1} s, pitch {:.0}° / {:.1} s)", m.roll_deg, roll_period, m.pitch_deg, m.pitch_period),
            r2(fore_aft),
            r2(fore_aft),
            r2(sideways),
            r2((1.0 - vertical).max(0.0)),
            r2(1.0 + vertical),
        );
        ShipCase { case, roll_period, notes }
    }
}

/// Ship parameters and a stowage position for [`TransportCase::from_ship`].
/// Metres, seconds, degrees; accelerations in g.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShipMotion {
    /// Beam (breadth) B, m.
    pub beam: f64,
    /// Metacentric height GM, m.
    pub gm: f64,
    /// Roll inertia coefficient c (about 0.38–0.42).
    pub roll_coeff: f64,
    /// Roll amplitude, degrees.
    pub roll_deg: f64,
    /// Pitch amplitude, degrees.
    pub pitch_deg: f64,
    /// Pitch period, s.
    pub pitch_period: f64,
    /// Height of the stowage position above the roll and pitch axes, m.
    pub height: f64,
    /// Distance of the stowage position from midship (pitch axis), m.
    pub from_midship: f64,
    /// Surge acceleration, g.
    pub surge_g: f64,
    /// Heave acceleration amplitude, g.
    pub heave_g: f64,
}

impl Default for ShipMotion {
    fn default() -> Self {
        ShipMotion {
            beam: 32.2,
            gm: 1.5,
            roll_coeff: 0.4,
            roll_deg: 22.0,
            pitch_deg: 5.0,
            pitch_period: 8.0,
            height: 15.0,
            from_midship: 60.0,
            surge_g: 0.1,
            heave_g: 0.3,
        }
    }
}

/// A transport case derived from ship motion, with the roll period and any
/// stability notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipCase {
    pub case: TransportCase,
    /// Natural roll period, s.
    pub roll_period: f64,
    pub notes: Vec<String>,
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
    /// Treat tipping in transport as a placement constraint: positions and
    /// orientations that would tip unless lashed are rejected. A unit with no
    /// other way to fit is still loaded and reported as needing lashing.
    pub avoid_tipping: bool,
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
    /// Lashing used to count the direct lashings a unit needs.
    pub lashing: LashingOptions,
}

/// Friction of rubber anti-slip mats (EN 12195-1 Annex B, typical value).
pub const ANTI_SLIP_FRICTION: f64 = 0.6;

/// Friction factor for direct lashing (EN 12195-1:2010).
pub const LASHING_FRICTION_FACTOR: f64 = 0.75;

/// Direct lashing (EN 12195-1) used to turn securing forces into a number of
/// lashings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LashingOptions {
    /// Lashing capacity LC of one lashing, daN (a common strap: 2000 daN).
    pub capacity_dan: f64,
    /// Strength of the lashing points it is fixed to, daN (ISO 1496-1 floor
    /// points: 1000 daN). The weaker of the two counts.
    pub anchor_dan: f64,
    /// Angle α between the lashing and the floor, degrees.
    pub vertical_angle: f64,
    /// Angle β between the lashing and the direction it holds, degrees.
    pub horizontal_angle: f64,
}

impl Default for LashingOptions {
    fn default() -> Self {
        LashingOptions { capacity_dan: 2000.0, anchor_dan: 1000.0, vertical_angle: 45.0, horizontal_angle: 30.0 }
    }
}

impl LashingOptions {
    /// Effective capacity of one lashing, kN.
    pub fn effective_kn(&self) -> f64 {
        self.capacity_dan.min(self.anchor_dan).max(0.0) / 100.0
    }

    /// Direct lashings against sliding (EN 12195-1:2010):
    /// `n·LC ≥ m·g·(c − μ·f_μ·c_z) / (μ·f_μ·sin α + cos α·cos β)`, with `f_μ = 0.75`.
    pub fn against_sliding(&self, mass: f64, accel: f64, mu: f64, vertical: f64) -> u32 {
        let (a, b) = (self.vertical_angle.to_radians(), self.horizontal_angle.to_radians());
        let fmu = mu * LASHING_FRICTION_FACTOR;
        let need = mass * 9.81 * (accel - fmu * vertical) / 1000.0;
        let per = self.effective_kn() * (fmu * a.sin() + a.cos() * b.cos());
        count(need, per)
    }

    /// Direct lashings against tipping. Conservative: only the horizontal
    /// component of each lashing, acting at the unit's top (`top` above its
    /// base), resists the excess tipping moment `force_kn · lever`.
    pub fn against_tipping(&self, force_kn: f64, lever: f64, top: f64) -> u32 {
        let (a, b) = (self.vertical_angle.to_radians(), self.horizontal_angle.to_radians());
        if top <= 0.0 {
            return 0;
        }
        count(force_kn * lever / top, self.effective_kn() * a.cos() * b.cos())
    }
}

fn count(need: f64, per: f64) -> u32 {
    if need <= 1e-9 {
        0
    } else if per <= 1e-12 {
        u32::MAX
    } else {
        (need / per - 1e-9).ceil().max(1.0).min(u32::MAX as f64) as u32
    }
}

/// Load distribution rules of the CTU Code (IMO/ILO/UNECE 2014, annex 7). They
/// are reported as warnings; [`CogLimits`] are the hard limits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BalanceOptions {
    /// Report the CTU Code centre-of-gravity checks below.
    pub ctu_checks: bool,
    /// Largest offset of the cargo's centre of gravity from the middle, as a
    /// share of the length (lengthwise) and of the width (sideways).
    pub max_eccentricity: f64,
    /// Smallest share of the cargo mass between 25% and 75% of the length.
    pub min_central_share: f64,
    /// Highest centre of gravity, as a share of the inside height.
    pub max_cog_height: f64,
    /// Slide a partial load lengthwise by the least amount that best meets the
    /// centre-of-gravity window, the central share and the axle limits. The
    /// gaps left at the end walls must be braced.
    pub centre_lengthwise: bool,
}

impl Default for BalanceOptions {
    fn default() -> Self {
        BalanceOptions { ctu_checks: true, max_eccentricity: 0.05, min_central_share: 0.6, max_cog_height: 0.5, centre_lengthwise: false }
    }
}

impl Default for PhysicsOptions {
    fn default() -> Self {
        PhysicsOptions {
            transport: vec![TransportCase::presets().remove(0)],
            check_sliding: true,
            check_tipping: true,
            avoid_tipping: true,
            dynamic_stacking: false,
            default_friction: 0.4,
            use_chocks: true,
            secure_load_end: true,
            max_fill_gap: 50.0,
            floor_friction: None,
            anti_slip_mats: false,
            lashing: LashingOptions::default(),
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
    /// Tie-break weights of the placement score.
    pub weights: ScoreWeights,
    /// Load distribution checks (CTU Code) and lengthwise centring.
    pub balance: BalanceOptions,
    /// Placement scoring learned from saved plans, used by
    /// [`FillBias::Learned`] (see `learn.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<Ranker>,
}

/// Weights of the placement features (`placer::FEATURES`), learned from plans
/// the user marked as good. Lower scores win.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ranker {
    pub weights: Vec<f64>,
    /// Placement decisions it was trained on.
    #[serde(default)]
    pub examples: usize,
    /// Share of those decisions it ranks first, 0..1.
    #[serde(default)]
    pub accuracy: f64,
}

/// Secondary terms of the placement score. The fill pattern decides the main
/// direction (weight 1 per container length); these break ties between
/// positions that are about equally far along it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoreWeights {
    /// Reward for the fraction of the four side faces touching walls or items
    /// (denser, more interlocked packings).
    pub contact_area: f64,
    /// Reward for sides blocked against sliding, directly or across a
    /// fillable gap (only when transport checks are on).
    pub blocking: f64,
    /// Penalty per side that leaves a gap wider than the dunnage limit but
    /// narrower than the smallest unit: space nothing can use.
    pub dead_gap: f64,
    /// Reward for a top level with a touching neighbour's top: flat surfaces
    /// for the next layer.
    pub flat_top: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        ScoreWeights { contact_area: 4e-3, blocking: 4e-3, dead_gap: 4e-3, flat_top: 2e-3 }
    }
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
            weights: ScoreWeights::default(),
            balance: BalanceOptions::default(),
            ranker: None,
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
