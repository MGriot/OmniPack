// Mirrors the serde model in crates/omnipack-core (model.rs, plan.rs).
// Units: mm and kg. Y is up; the door is at z = depth.

export type Shape =
  | { kind: "box"; w: number; h: number; d: number }
  | { kind: "cylinder"; radius: number; length: number };

export type Orientation = "WHD" | "DHW" | "HWD" | "WDH" | "HDW" | "DWH";
export type Zone = "any" | "back" | "front";
export type FillBias = "wall_building" | "floor_first" | "longitudinal" | "lateral" | "corner_first";

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

export interface ContainerPlan {
  id: string;
  size: [number, number, number];
  placements: Placement[];
  metrics: Metrics;
  violations: Violation[];
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
    stability_margin: 0.1,
    min_support_ratio: 0.5,
    balance_weight: 0.3,
    allow_rotation: true,
    max_containers: 50,
    max_stability_checks: 400,
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
  };
}
