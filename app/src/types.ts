// Mirrors the serde model in crates/omnipack-core (model.rs, plan.rs).
// Units: mm and kg. Y is up; the door is at z = depth.

export type Shape =
  | { kind: "box"; w: number; h: number; d: number }
  | { kind: "cylinder"; radius: number; length: number }
  | { kind: "sphere"; radius: number }
  | { kind: "cone"; radius: number; height: number }
  | { kind: "pyramid"; w: number; d: number; height: number }
  | { kind: "prism"; sides: number; radius: number; length: number }
  | { kind: "l_profile"; a: number; b: number; thickness: number; length: number };

export type ShapeKind = Shape["kind"];

export const SHAPE_KINDS: [ShapeKind, string][] = [
  ["box", "Box"],
  ["cylinder", "Cylinder / drum"],
  ["sphere", "Sphere"],
  ["cone", "Cone"],
  ["pyramid", "Pyramid"],
  ["prism", "Prism (n sides)"],
  ["l_profile", "L-profile (angle)"],
];

export function defaultShape(kind: ShapeKind): Shape {
  switch (kind) {
    case "box": return { kind, w: 400, h: 300, d: 300 };
    case "cylinder": return { kind, radius: 150, length: 800 };
    case "sphere": return { kind, radius: 200 };
    case "cone": return { kind, radius: 200, height: 500 };
    case "pyramid": return { kind, w: 500, d: 500, height: 400 };
    case "prism": return { kind, sides: 3, radius: 200, length: 1200 };
    case "l_profile": return { kind, a: 200, b: 200, thickness: 20, length: 2000 };
  }
}

export type Orientation = "WHD" | "DHW" | "HWD" | "WDH" | "HDW" | "DWH";
export type Zone = "any" | "back" | "front";
export type FillBias = "wall_building" | "floor_first" | "longitudinal" | "lateral" | "corner_first";
export type StopOrder = "lifo" | "fifo";
export type LoadPriority = "volume" | "mass" | "base_area" | "height" | "as_listed";

export interface TransportCase {
  name: string;
  forward: number;
  backward: number;
  sideways: number;
  vertical_min: number;
  vertical_max: number;
}

/** Direct lashing (EN 12195-1) used to count lashings. */
export interface LashingOptions {
  /** Lashing capacity LC of one lashing, daN. */
  capacity_dan: number;
  /** Strength of the lashing points, daN (ISO floor points: 1000). */
  anchor_dan: number;
  /** Angle α to the floor, degrees. */
  vertical_angle: number;
  /** Angle β to the direction held, degrees. */
  horizontal_angle: number;
}

export function defaultLashing(): LashingOptions {
  return { capacity_dan: 2000, anchor_dan: 1000, vertical_angle: 45, horizontal_angle: 30 };
}

export interface PhysicsOptions {
  transport: TransportCase[];
  check_sliding: boolean;
  check_tipping: boolean;
  /** Reject positions that would tip in transport; units with no alternative are loaded and flagged for lashing. */
  avoid_tipping: boolean;
  dynamic_stacking: boolean;
  default_friction: number;
  use_chocks: boolean;
  secure_load_end: boolean;
  /** Gaps up to this size (mm) are filled with dunnage and then block. */
  max_fill_gap: number;
  /** Cargo-to-floor friction; null = the item's own value. */
  floor_friction: number | null;
  anti_slip_mats: boolean;
  lashing: LashingOptions;
}

/** Same as the first Rust preset; used until the presets are loaded. */
export const ROAD: TransportCase = { name: "Road (EN 12195-1)", forward: 0.8, backward: 0.5, sideways: 0.5, vertical_min: 1.0, vertical_max: 1.3 };

export function defaultPhysics(): PhysicsOptions {
  return { transport: [ROAD], check_sliding: true, check_tipping: true, avoid_tipping: true, dynamic_stacking: false, default_friction: 0.4, use_chocks: true, secure_load_end: true, max_fill_gap: 50, floor_friction: null, anti_slip_mats: false, lashing: defaultLashing() };
}

export interface ItemSpec {
  id: string;
  shape: Shape;
  mass: number;
  quantity: number;
  max_load_on_top: number | null;
  fragile: boolean;
  floor_only: boolean;
  upright_only: boolean;
  allowed_orientations: Orientation[] | null;
  stop: number;
  zone: Zone;
  com_offset: [number, number, number];
  color: string | null;
  friction: number | null;
}

export interface Axle {
  z: number;
  max_load: number;
}

export interface CogLimits {
  max_lateral_offset: number | null;
  z_min: number | null;
  z_max: number | null;
  max_height: number | null;
}

/** Tractor + semi-trailer (mm along the vehicle, rearwards positive; kg). */
export interface RoadVehicle {
  name: string;
  /** Kingpin → container front wall; positive = behind the kingpin. */
  container_front: number;
  /** Kingpin → centre of the trailer axle group. */
  trailer_wheelbase: number;
  trailer_tare: number;
  /** Kingpin → centre of gravity of the empty trailer. */
  trailer_cog: number;
  tractor_tare: number;
  /** Steer axle → drive axle. */
  tractor_wheelbase: number;
  /** Steer axle → centre of gravity of the tractor. */
  tractor_cog: number;
  /** Steer axle → kingpin (fifth wheel). */
  fifth_wheel: number;
  max_steer: number;
  max_drive: number;
  max_trailer: number;
  max_gross: number;
}

export interface ContainerSpec {
  id: string;
  width: number;
  height: number;
  depth: number;
  max_payload: number | null;
  axles: [Axle, Axle] | null;
  cog_limits: CogLimits;
  /** Clear door opening [width, height], mm; null = no door limit. */
  door: [number, number] | null;
  /** Empty container mass (CSC plate), kg. */
  tare_mass: number | null;
  /** Floor rating, kg/m². */
  floor_rating: number | null;
  vehicle: RoadVehicle | null;
}

/** Load distribution checks (CTU Code); warnings, not violations. */
export interface BalanceOptions {
  ctu_checks: boolean;
  /** Largest CoG offset from the middle, share of length / width. */
  max_eccentricity: number;
  /** Smallest share of the mass between 25% and 75% of the length. */
  min_central_share: number;
  /** Highest CoG, share of the inside height. */
  max_cog_height: number;
  /** Slide a partial load lengthwise to meet the window and axle limits. */
  centre_lengthwise: boolean;
}

export function defaultBalance(): BalanceOptions {
  return { ctu_checks: true, max_eccentricity: 0.05, min_central_share: 0.6, max_cog_height: 0.5, centre_lengthwise: false };
}

export interface PackOptions {
  bias: FillBias;
  stop_order: StopOrder;
  priority: LoadPriority;
  physics: PhysicsOptions;
  stability_margin: number;
  min_support_ratio: number;
  balance_weight: number;
  allow_rotation: boolean;
  max_containers: number;
  max_stability_checks: number;
  seed: number;
  balance: BalanceOptions;
}

export interface PackRequest {
  container: ContainerSpec;
  items: ItemSpec[];
  options: PackOptions;
}

export interface Placement {
  instance_id: string;
  item_id: string;
  seq: number;
  shape: Shape;
  orientation: Orientation;
  position: [number, number, number];
  size: [number, number, number];
  center_of_mass: [number, number, number];
  mass: number;
  load_on_top: number;
  support_margin: number;
  stop: number;
  needs_chocks: boolean;
  securing: SecuringClass;
  impact?: Impact;
  color?: string;
  /** Pressure on the container floor, kg/m² (0 = not on the floor). */
  floor_pressure: number;
}

export type SecuringClass = "secured" | "dunnage" | "chocks" | "lashing" | "overloaded";

export interface Impact {
  /** Force passed on to whatever blocks the unit, kN. */
  force_kn: number;
  /** Demand over own resistance (friction, base width); above 1 it relies on blocking or lashing. */
  ratio: number;
  case: string;
  direction: Direction;
}

export interface Metrics {
  item_count: number;
  total_mass: number;
  volume_utilization: number;
  weight_utilization: number | null;
  center_of_mass: [number, number, number];
  lateral_offset: number;
  axle_loads: [number, number] | null;
  accessibility: number;
  min_support_margin: number;
}

export type Violation = { kind: string } & Record<string, unknown>;

export type Direction = "forward" | "backward" | "left" | "right";
export type IssueKind = "sliding" | "tipping" | "stack_overload";

export interface TransportIssue {
  item: string;
  kind: IssueKind;
  direction?: Direction;
  acceleration: number;
  /** kN of securing force; kg of excess load for stack overloads. */
  required: number;
  /** Direct lashings of the configured capacity that provide it. */
  lashings: number;
}

export interface TransportResult {
  case: string;
  issues: TransportIssue[];
  gaps: GapFill[];
}

export interface GapFill {
  item: string;
  /** Neighbour on the other side; absent = wall or load end. */
  other?: string;
  direction: Direction;
  gap_mm: number;
}

export interface VehicleLoads {
  steer: number;
  drive: number;
  trailer: number;
  gross: number;
  /** steer, drive, trailer, gross */
  limits: [number, number, number, number];
}

export type BalanceIssue =
  | { kind: "cog_lengthwise"; offset: number; limit: number }
  | { kind: "cog_lateral"; offset: number; limit: number }
  | { kind: "central_share"; share: number; min: number }
  | { kind: "cog_height"; height: number; limit: number }
  | { kind: "axle_overload"; axle: string; load: number; max: number }
  | { kind: "floor_pressure"; item: string; pressure: number; limit: number; spread_area: number };

export interface BalanceReport {
  /** mm the load was moved towards the door (+). */
  shift: number;
  lengthwise_offset: number;
  lateral_offset: number;
  central_share: number;
  half_shares: [number, number];
  cog_height_ratio: number;
  /** Free length to the front wall and to the door, mm. */
  end_gaps: [number, number];
  vgm?: number;
  vehicle?: VehicleLoads;
  issues: BalanceIssue[];
  excess: number;
}

export interface ContainerPlan {
  id: string;
  size: [number, number, number];
  placements: Placement[];
  metrics: Metrics;
  violations: Violation[];
  transport: TransportResult[];
  balance: BalanceReport;
}

/** Ship and stowage position for a sea case from ship motion (m, s, degrees, g). */
export interface ShipMotion {
  beam: number;
  gm: number;
  roll_coeff: number;
  roll_deg: number;
  pitch_deg: number;
  pitch_period: number;
  height: number;
  from_midship: number;
  surge_g: number;
  heave_g: number;
}

export function defaultShipMotion(): ShipMotion {
  return { beam: 32.2, gm: 1.5, roll_coeff: 0.4, roll_deg: 22, pitch_deg: 5, pitch_period: 8, height: 15, from_midship: 60, surge_g: 0.1, heave_g: 0.3 };
}

export interface ShipCase {
  case: TransportCase;
  roll_period: number;
  notes: string[];
}

export interface RenderMesh {
  positions: number[];
  indices: number[];
}

export interface Unpacked {
  instance_id: string;
  item_id: string;
  reason: string;
}

export interface PackResult {
  schema: string;
  containers: ContainerPlan[];
  unpacked: Unpacked[];
  requested_units: number;
  packed_units: number;
  volume_utilization: number;
  elapsed_ms: number;
}

/** Which world axis (0 = x, 1 = y, 2 = z) holds each local axis, per orientation. */
export const AXIS_MAP: Record<Orientation, [number, number, number]> = {
  WHD: [0, 1, 2],
  DHW: [2, 1, 0],
  HWD: [1, 0, 2],
  WDH: [0, 2, 1],
  HDW: [1, 2, 0],
  DWH: [2, 0, 1],
};

export function defaultOptions(): PackOptions {
  return {
    bias: "wall_building",
    stop_order: "lifo",
    priority: "volume",
    physics: defaultPhysics(),
    stability_margin: 0.1,
    min_support_ratio: 0.5,
    balance_weight: 0.3,
    allow_rotation: true,
    max_containers: 50,
    max_stability_checks: 5000,
    seed: 0,
    balance: defaultBalance(),
  };
}

export function newItem(n: number): ItemSpec {
  return {
    id: `item${n}`,
    shape: { kind: "box", w: 400, h: 300, d: 300 },
    mass: 10,
    quantity: 1,
    max_load_on_top: null,
    fragile: false,
    floor_only: false,
    upright_only: false,
    allowed_orientations: null,
    stop: 0,
    zone: "any",
    com_offset: [0, 0, 0],
    color: null,
    friction: null,
  };
}

// ---------- search (omnipack-opt) ----------

export interface Objective {
  density: number;
  securing: number;
  dunnage: number;
  stability: number;
  balance: number;
}

export interface OptimizeOptions {
  budget_ms: number;
  max_evaluations: number;
  population: number;
  elite_fraction: number;
  mutant_fraction: number;
  inherit: number;
  keep: number;
  objective: Objective;
  threads: number;
  seed: number;
}

export interface Score {
  all_packed: boolean;
  packed_units: number;
  containers: number;
  value: number;
  volume_utilization: number;
  /** Units that would tip in transport unless lashed. */
  tipping_units: number;
  lashing_units: number;
  lashing_kn: number;
  dunnage_mm: number;
  min_margin: number | null;
  /** CTU window / central share / axle warnings over all containers. */
  balance_issues: number;
  balance_excess: number;
  /** Units over the floor rating. */
  floor_overloads: number;
}

/** An item on its own (unrotated), for the preview in its card. */
export interface ItemPreview {
  mesh: RenderMesh;
  /** Bounding box W, H, D, mm. */
  extents: [number, number, number];
  /** Uniform-density centre of mass, from the min corner, mm. */
  centroid: [number, number, number];
  /** Centre of mass including `com_offset`, from the min corner, mm. */
  com: [number, number, number];
}

export interface Solution {
  label: string;
  score: Score;
  result: PackResult;
}

export interface OptimizeResult {
  solutions: Solution[];
  baseline: Score;
  evaluated: number;
  elapsed_ms: number;
  cancelled: boolean;
}

export type SearchPhase = "sweep" | "evolve" | "polish";

export interface SearchProgress {
  phase: SearchPhase;
  evaluated: number;
  elapsed_ms: number;
  best: Score;
}
