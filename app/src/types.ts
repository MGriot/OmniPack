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

export interface PhysicsOptions {
  transport: TransportCase[];
  check_sliding: boolean;
  check_tipping: boolean;
  dynamic_stacking: boolean;
  default_friction: number;
  use_chocks: boolean;
  secure_load_end: boolean;
}

/** Same as the first Rust preset; used until the presets are loaded. */
export const ROAD: TransportCase = { name: "Road (EN 12195-1)", forward: 0.8, backward: 0.5, sideways: 0.5, vertical_min: 1.0, vertical_max: 1.0 };

export function defaultPhysics(): PhysicsOptions {
  return { transport: [ROAD], check_sliding: true, check_tipping: true, dynamic_stacking: false, default_friction: 0.4, use_chocks: true, secure_load_end: true };
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

export interface ContainerSpec {
  id: string;
  width: number;
  height: number;
  depth: number;
  max_payload: number | null;
  axles: [Axle, Axle] | null;
  cog_limits: CogLimits;
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
  color?: string;
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
}

export interface TransportResult {
  case: string;
  issues: TransportIssue[];
}

export interface ContainerPlan {
  id: string;
  size: [number, number, number];
  placements: Placement[];
  metrics: Metrics;
  violations: Violation[];
  transport: TransportResult[];
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
