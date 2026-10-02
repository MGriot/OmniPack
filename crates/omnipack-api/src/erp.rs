//! The simple ERP format: flat items with units, as an ERP system (SAP and
//! the like) has them, and a flat list of placements back.
//!
//! Lengths and weights are in the request's units (SAP unit codes and ISO
//! codes are understood); the engine works in mm and kg. Axes of the answer:
//! `x` across the width from the left wall, `y` up from the floor, `z` along
//! the length from the front wall towards the door. Positions are the
//! minimum corner of each unit's bounding box.

use omnipack_core::{
    BalanceIssue, ContainerSpec, FillBias, ItemSpec, PackOptions, PackRequest, PackResult, Placement, Ranker, RoadVehicle, StopOrder, TransportCase, Zone,
};
use omnipack_geom::Shape;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A request in the simple format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpRequest {
    /// Your reference (delivery, shipment, order); returned unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default)]
    pub units: Units,
    pub container: ErpContainer,
    /// Road vehicle preset name, for axle loads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<String>,
    pub items: Vec<ErpItem>,
    #[serde(default)]
    pub options: ErpOptions,
    /// Placements to check (`/validate` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placements: Option<Vec<ErpPlacementIn>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Units {
    /// mm, cm, m, in, ft (also the SAP/ISO codes MMT, CMT, MTR, INH, FOT).
    pub length: String,
    /// kg, g, t, lb (also KGM, GRM, TNE, TO, LBR).
    pub weight: String,
}

impl Default for Units {
    fn default() -> Self {
        Units { length: "mm".into(), weight: "kg".into() }
    }
}

impl Units {
    /// Millimetres per length unit.
    pub fn mm(&self) -> Result<f64, String> {
        Ok(match self.length.trim().to_ascii_lowercase().as_str() {
            "mm" | "mmt" => 1.0,
            "cm" | "cmt" => 10.0,
            "dm" | "dmt" => 100.0,
            "m" | "mtr" => 1000.0,
            "in" | "inh" | "inch" => 25.4,
            "ft" | "fot" | "foot" => 304.8,
            other => return Err(format!("unknown length unit `{other}` (use mm, cm, m, in or ft)")),
        })
    }

    /// Kilograms per weight unit.
    pub fn kg(&self) -> Result<f64, String> {
        Ok(match self.weight.trim().to_ascii_lowercase().as_str() {
            "kg" | "kgm" => 1.0,
            "g" | "grm" => 0.001,
            "t" | "to" | "tne" => 1000.0,
            "lb" | "lbr" | "lbs" => 0.453_592_37,
            other => return Err(format!("unknown weight unit `{other}` (use kg, g, t or lb)")),
        })
    }
}

/// The container: a preset (`20ft-dv`, `40ft-dv`, `40ft-hc`,
/// `semi-trailer-13.6`) and/or explicit inside sizes, which override it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ErpContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Inside length (door axis).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_payload: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub door_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub door_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tare: Option<f64>,
    /// kg/m² (not converted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_rating: Option<f64>,
}

/// One item type (material) with its quantity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpItem {
    /// Material number or any unique id.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "one")]
    pub quantity: u32,
    /// box (default), cylinder (standing drum: diameter × height) or sphere (diameter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(default)]
    pub length: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter: Option<f64>,
    /// Gross weight of one unit.
    #[serde(default)]
    pub weight: f64,
    /// Other units may be stacked on it (default true).
    #[serde(default = "yes")]
    pub stackable: bool,
    /// Most weight that may rest on it, in weight units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_load_on_top: Option<f64>,
    #[serde(default)]
    pub fragile: bool,
    #[serde(default)]
    pub this_side_up: bool,
    #[serde(default)]
    pub floor_only: bool,
    /// Delivery stop, 1 = unloaded first; 0 = none.
    #[serde(default)]
    pub stop: u32,
    /// any (default), front (near the door) or back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn one() -> u32 {
    1
}
fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ErpOptions {
    /// walls (default), floor, length, rows, corner, learned or best (search).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Search time for `best`, seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_seconds: Option<f64>,
    /// road, rail, rail_shunting, sea_a, sea_b, sea_c (default: road).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Vec<String>>,
    /// lifo (default) or fifo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_order: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centre_lengthwise: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctu_checks: Option<bool>,
    /// Largest gap filled with dunnage, mm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dunnage_mm: Option<f64>,
    /// Default friction coefficient.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_containers: Option<u32>,
    /// Any `PackOptions` fields, merged last (see docs/conventions.md).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced: Option<Value>,
}

/// A placement to check, in the request's units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpPlacementIn {
    pub item_id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// WHD (as defined, default), DHW, HWD, WDH, HDW, DWH.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<String>,
}

/// What a request asks the engine to do.
#[derive(Debug, Clone)]
pub struct Plan {
    pub request: PackRequest,
    /// Search time (ms) for the `best` fill pattern; `None` = one pack.
    pub search_ms: Option<u64>,
    pub mm: f64,
    pub kg: f64,
}

fn transport_case(name: &str) -> Result<TransportCase, String> {
    let presets = TransportCase::presets();
    let i = match name.trim().to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
        "road" => 0,
        "rail" | "rail_combined" => 1,
        "rail_shunting" | "shunting" => 2,
        "sea_a" => 3,
        "sea_b" => 4,
        "sea_c" | "sea" => 5,
        other => return Err(format!("unknown transport `{other}` (road, rail, rail_shunting, sea_a, sea_b, sea_c)")),
    };
    Ok(presets[i].clone())
}

fn fill(name: &str) -> Result<Option<FillBias>, String> {
    Ok(Some(match name.trim().to_ascii_lowercase().as_str() {
        "walls" | "wall_building" => FillBias::WallBuilding,
        "floor" | "floor_first" => FillBias::FloorFirst,
        "length" | "longitudinal" => FillBias::Longitudinal,
        "rows" | "lateral" => FillBias::Lateral,
        "corner" | "corner_first" => FillBias::CornerFirst,
        "learned" => FillBias::Learned,
        "best" => return Ok(None),
        other => return Err(format!("unknown fill `{other}` (walls, floor, length, rows, corner, learned, best)")),
    }))
}

/// JSON-merges `patch` into `base` (objects merge, everything else replaces).
pub fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

impl ErpRequest {
    /// The engine request. `max_search_s` caps the `best` search time;
    /// `ranker` is the learned model for `fill: learned`.
    pub fn to_plan(&self, max_search_s: f64, ranker: Option<&Ranker>) -> Result<Plan, String> {
        let (mm, kg) = (self.units.mm()?, self.units.kg()?);
        let c = &self.container;
        let mut container = match &c.preset {
            Some(p) => ContainerSpec::presets().into_iter().find(|x| x.id.eq_ignore_ascii_case(p)).ok_or_else(|| {
                format!("unknown container preset `{p}` (20ft-dv, 40ft-dv, 40ft-hc, semi-trailer-13.6)")
            })?,
            None => {
                let (Some(l), Some(w), Some(h)) = (c.length, c.width, c.height) else {
                    return Err("container: give a preset or length, width and height".into());
                };
                ContainerSpec::new("container", w * mm, h * mm, l * mm)
            }
        };
        if let Some(id) = &c.id {
            container.id = id.clone();
        }
        if let Some(v) = c.length {
            container.depth = v * mm;
        }
        if let Some(v) = c.width {
            container.width = v * mm;
        }
        if let Some(v) = c.height {
            container.height = v * mm;
        }
        if let Some(v) = c.max_payload {
            container.max_payload = Some(v * kg);
        }
        if let Some(v) = c.tare {
            container.tare_mass = Some(v * kg);
        }
        if let Some(v) = c.floor_rating {
            container.floor_rating = Some(v);
        }
        if c.door_width.is_some() || c.door_height.is_some() {
            let [dw, dh] = container.door.unwrap_or([container.width, container.height]);
            container.door = Some([c.door_width.map_or(dw, |v| v * mm), c.door_height.map_or(dh, |v| v * mm)]);
        }
        if let Some(name) = &self.vehicle {
            container.vehicle = Some(
                RoadVehicle::presets().into_iter().find(|v| v.name.eq_ignore_ascii_case(name)).ok_or_else(|| format!("unknown vehicle `{name}`"))?,
            );
        }

        let mut items = Vec::with_capacity(self.items.len());
        for it in &self.items {
            let bad = |what: &str| format!("item `{}`: {what}", it.id);
            let shape = match it.shape.as_deref().unwrap_or("box").to_ascii_lowercase().as_str() {
                "box" | "carton" | "pallet" => Shape::Box { w: it.width * mm, h: it.height * mm, d: it.length * mm },
                "cylinder" | "drum" | "roll" => {
                    let d = it.diameter.ok_or_else(|| bad("a cylinder needs a diameter"))?;
                    let h = if it.height > 0.0 { it.height } else { it.length };
                    Shape::Cylinder { radius: d * mm / 2.0, length: h * mm }
                }
                "sphere" | "ball" => Shape::Sphere { radius: it.diameter.ok_or_else(|| bad("a sphere needs a diameter"))? * mm / 2.0 },
                other => return Err(bad(&format!("unknown shape `{other}` (box, cylinder, sphere)"))),
            };
            if !shape.is_valid() {
                return Err(bad("sizes must be positive"));
            }
            let zone = match it.zone.as_deref().unwrap_or("any").to_ascii_lowercase().as_str() {
                "any" | "" => Zone::Any,
                "front" | "door" => Zone::Front,
                "back" => Zone::Back,
                other => return Err(bad(&format!("unknown zone `{other}` (any, front, back)"))),
            };
            items.push(ItemSpec {
                id: it.id.clone(),
                shape,
                mass: it.weight * kg,
                quantity: it.quantity,
                max_load_on_top: if it.stackable { it.max_load_on_top.map(|v| v * kg) } else { Some(0.0) },
                fragile: it.fragile,
                floor_only: it.floor_only,
                upright_only: it.this_side_up,
                allowed_orientations: None,
                stop: it.stop,
                zone,
                com_offset: [0.0; 3],
                color: it.color.clone(),
                friction: it.friction,
            });
        }

        let o = &self.options;
        let mut options = PackOptions::default();
        let bias = fill(o.fill.as_deref().unwrap_or("walls"))?;
        options.bias = bias.unwrap_or(FillBias::WallBuilding);
        if let Some(t) = &o.transport {
            options.physics.transport = t.iter().map(|n| transport_case(n)).collect::<Result<_, _>>()?;
        }
        if let Some(s) = &o.stop_order {
            options.stop_order = match s.to_ascii_lowercase().as_str() {
                "lifo" => StopOrder::Lifo,
                "fifo" => StopOrder::Fifo,
                other => return Err(format!("unknown stop_order `{other}` (lifo, fifo)")),
            };
        }
        if let Some(v) = o.rotation {
            options.allow_rotation = v;
        }
        if let Some(v) = o.centre_lengthwise {
            options.balance.centre_lengthwise = v;
        }
        if let Some(v) = o.ctu_checks {
            options.balance.ctu_checks = v;
        }
        if let Some(v) = o.dunnage_mm {
            options.physics.max_fill_gap = v.max(0.0);
        }
        if let Some(v) = o.friction {
            options.physics.default_friction = v.max(0.0);
        }
        if let Some(v) = o.max_containers {
            options.max_containers = v.max(1);
        }
        options.ranker = ranker.cloned();
        if let Some(adv) = &o.advanced {
            let mut v = serde_json::to_value(&options).map_err(|e| e.to_string())?;
            merge(&mut v, adv);
            options = serde_json::from_value(v).map_err(|e| format!("options.advanced: {e}"))?;
        }
        let search_ms = bias.is_none().then(|| (o.search_seconds.unwrap_or(10.0).clamp(1.0, max_search_s.max(1.0)) * 1000.0) as u64);
        Ok(Plan { request: PackRequest { container, items, options }, search_ms, mm, kg })
    }
}

/// The answer in the simple format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// ok (everything placed, no violations), partial (not everything fits)
    /// or invalid (violations: only for hand-made plans checked by /validate).
    pub status: String,
    pub units: Units,
    pub summary: Summary,
    pub containers: Vec<ErpContainerOut>,
    pub placements: Vec<ErpPlacement>,
    pub not_placed: Vec<NotPlaced>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub units_requested: usize,
    pub units_placed: usize,
    pub containers: usize,
    /// Share of the used containers' volume, 0..1.
    pub volume_utilization: f64,
    pub total_weight: f64,
    pub valid: bool,
    pub balance_warnings: usize,
    /// Units that need lashing or blocking for the selected transport.
    pub units_to_lash: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpContainerOut {
    /// 1-based.
    pub index: usize,
    pub id: String,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub units: usize,
    pub weight: f64,
    pub volume_utilization: f64,
    /// Centre of gravity of the cargo (x, y, z).
    pub center_of_gravity: [f64; 3],
    /// Verified gross mass (tare + cargo), when the tare is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vgm: Option<f64>,
    /// Problems that make the plan physically invalid (hand-made plans).
    pub violations: Vec<String>,
    /// Load balance warnings (CTU Code window, axle loads, floor pressure).
    pub warnings: Vec<String>,
    /// Securing needed per transport leg.
    pub transport: Vec<TransportOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportOut {
    pub case: String,
    /// One line per unit and direction: what slides or tips and the force or lashings to hold it.
    pub issues: Vec<String>,
    /// Total width of the gaps to fill with dunnage, in length units.
    pub dunnage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErpPlacement {
    /// 1-based container index.
    pub container: usize,
    /// Loading order within the container, from 1.
    pub sequence: usize,
    pub item_id: String,
    /// Which unit of the item, from 1.
    pub unit: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Size as placed: along the container length, width and height.
    pub length: f64,
    pub width: f64,
    pub height: f64,
    /// Which of the item's dimensions lie along the container's width, height
    /// and length: WHD = as defined, DHW = turned 90°, ... (docs/conventions.md).
    pub rotation: String,
    pub on_floor: bool,
    pub load_on_top: f64,
    /// secured, dunnage, chocks, lashing or overloaded.
    pub securing: String,
    /// Direct lashings needed (largest over the transport legs).
    pub lashings: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotPlaced {
    pub item_id: String,
    pub unit: u32,
    pub reason: String,
}

fn enum_name<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// A tagged enum value as one readable line: `kind: field value, …`.
pub fn describe<T: Serialize>(v: &T) -> String {
    let Ok(Value::Object(m)) = serde_json::to_value(v) else { return String::new() };
    let kind = m.get("kind").and_then(Value::as_str).unwrap_or("").replace('_', " ");
    let fields: Vec<String> = m
        .iter()
        .filter(|(k, _)| *k != "kind")
        .map(|(k, v)| match v {
            Value::Number(n) => format!("{k} {:.1}", n.as_f64().unwrap_or(0.0)),
            Value::String(s) => format!("{k} {s}"),
            other => format!("{k} {other}"),
        })
        .collect();
    if fields.is_empty() {
        kind
    } else {
        format!("{kind}: {}", fields.join(", "))
    }
}

/// A load balance warning in words, with lengths and weights in the
/// request's units (`lu`, `wu`: their names).
fn balance_text(i: &BalanceIssue, l: impl Fn(f64) -> f64, w: impl Fn(f64) -> f64, lu: &str, wu: &str) -> String {
    match i {
        BalanceIssue::CogLengthwise { offset, limit } => format!(
            "centre of gravity {:.1} {lu} towards the {} (limit {:.1} {lu})",
            l(offset.abs()),
            if *offset > 0.0 { "door" } else { "front wall" },
            l(*limit)
        ),
        BalanceIssue::CogLateral { offset, limit } => {
            format!("centre of gravity {:.1} {lu} to the {} (limit {:.1} {lu})", l(offset.abs()), if *offset > 0.0 { "right" } else { "left" }, l(*limit))
        }
        BalanceIssue::CentralShare { share, min } => format!("{:.0}% of the weight in the middle half of the length (at least {:.0}%)", share * 100.0, min * 100.0),
        BalanceIssue::CogHeight { height, limit } => format!("centre of gravity {:.1} {lu} high (limit {:.1} {lu})", l(*height), l(*limit)),
        BalanceIssue::AxleOverload { axle, load, max } => format!("{axle}: {:.1} {wu} over the {:.1} {wu} limit", w(*load), w(*max)),
        BalanceIssue::FloorPressure { item, pressure, limit, spread_area } => {
            format!("{item}: {pressure:.0} kg/m² on a {limit:.0} kg/m² floor, spread over at least {spread_area:.2} m²")
        }
    }
}

/// Unit number from an instance id such as `MAT-1#3`.
pub fn unit_of(instance_id: &str) -> u32 {
    instance_id.rsplit_once('#').and_then(|(_, n)| n.parse().ok()).unwrap_or(0)
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The engine's result in the simple format, in the request's units.
pub fn response(reference: Option<String>, units: &Units, result: &PackResult) -> Result<ErpResponse, String> {
    let (mm, kg) = (units.mm()?, units.kg()?);
    let l = |v: f64| round(v / mm);
    let w = |v: f64| round(v / kg);
    let mut placements = Vec::new();
    let mut containers = Vec::new();
    let mut to_lash = 0;
    for (ci, c) in result.containers.iter().enumerate() {
        let lashings = |p: &Placement| c.transport.iter().flat_map(|t| &t.issues).filter(|i| i.item == p.instance_id).map(|i| i.lashings).max().unwrap_or(0);
        let mut ordered: Vec<&Placement> = c.placements.iter().collect();
        ordered.sort_by_key(|p| p.seq);
        for p in ordered {
            to_lash += usize::from(p.securing >= omnipack_core::SecuringClass::Lashing);
            placements.push(ErpPlacement {
                container: ci + 1,
                sequence: p.seq + 1,
                item_id: p.item_id.clone(),
                unit: unit_of(&p.instance_id),
                x: l(p.position[0]),
                y: l(p.position[1]),
                z: l(p.position[2]),
                length: l(p.size[2]),
                width: l(p.size[0]),
                height: l(p.size[1]),
                rotation: enum_name(&p.orientation),
                on_floor: p.position[1] < 0.5,
                load_on_top: w(p.load_on_top),
                securing: enum_name(&p.securing),
                lashings: lashings(p),
            });
        }
        let m = &c.metrics;
        containers.push(ErpContainerOut {
            index: ci + 1,
            id: c.id.clone(),
            length: l(c.size[2]),
            width: l(c.size[0]),
            height: l(c.size[1]),
            units: c.placements.len(),
            weight: w(m.total_mass),
            volume_utilization: round(m.volume_utilization),
            center_of_gravity: [l(m.center_of_mass[0]), l(m.center_of_mass[1]), l(m.center_of_mass[2])],
            vgm: c.balance.vgm.map(w),
            // Engine values: mm and kg.
            violations: c.violations.iter().map(|v| format!("{} (mm, kg)", describe(v))).collect(),
            warnings: c.balance.issues.iter().map(|i| balance_text(i, l, w, &units.length, &units.weight)).collect(),
            transport: c
                .transport
                .iter()
                .map(|t| TransportOut {
                    case: t.case.clone(),
                    issues: t
                        .issues
                        .iter()
                        .map(|i| {
                            let dir = i.direction.map(|d| format!(" {}", enum_name(&d))).unwrap_or_default();
                            let lash = if i.lashings > 0 { format!(" or {} lashing(s)", i.lashings) } else { String::new() };
                            format!("{}: {}{dir} at {} g, block ≥ {:.2} kN{lash}", i.item, enum_name(&i.kind), i.acceleration, i.required)
                        })
                        .collect(),
                    dunnage: l(t.gaps.iter().map(|g| g.gap_mm).sum()),
                })
                .collect(),
        });
    }
    let valid = result.is_valid();
    let balance_warnings = result.containers.iter().flat_map(|c| &c.balance.issues).filter(|i| !matches!(i, BalanceIssue::FloorPressure { .. })).count();
    let status = if !valid {
        "invalid"
    } else if result.unpacked.is_empty() {
        "ok"
    } else {
        "partial"
    };
    Ok(ErpResponse {
        reference,
        status: status.into(),
        units: units.clone(),
        summary: Summary {
            units_requested: result.requested_units,
            units_placed: result.packed_units,
            containers: result.containers.len(),
            volume_utilization: round(result.volume_utilization),
            total_weight: w(result.containers.iter().map(|c| c.metrics.total_mass).sum()),
            valid,
            balance_warnings,
            units_to_lash: to_lash,
        },
        containers,
        placements,
        not_placed: result.unpacked.iter().map(|u| NotPlaced { item_id: u.item_id.clone(), unit: unit_of(&u.instance_id), reason: enum_name(&u.reason) }).collect(),
    })
}
