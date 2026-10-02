import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import {
  type ApiStatus,
  type LocalApiSettings,
  defaultBalance,
  defaultLashing,
  defaultOptions,
  defaultPhysics,
  defaultShape,
  defaultShipMotion,
  newItem,
  ROAD,
  SHAPE_KINDS,
  type BalanceIssue,
  type ContainerPlan,
  type ContainerSpec,
  type Direction,
  type FillBias,
  type ItemSpec,
  type ItemPreview,
  type LoadPriority,
  type ManualView,
  type ModelFile,
  type Objective,
  type Orientation,
  type Pose,
  type Probe,
  type SavedPlan,
  type SolutionMeta,
  type OptimizeResult,
  type PackRequest,
  type PackResult,
  type Placement,
  type RenderMesh,
  type RoadVehicle,
  type Score,
  type Shape,
  type SearchPhase,
  type SearchProgress,
  type SecuringClass,
  type ShapeKind,
  type ShipCase,
  type ShipMotion,
  type StopOrder,
  type TransportCase,
  type TransportIssue,
  type Zone,
} from "./types";
import { previewCanvas, prunePreviews } from "./preview";
import { PlanViewer, type BalanceGuides, type Hit, type Interaction } from "./viewer";

// ---------- state ----------

let req: PackRequest = emptyRequest();
let result: PackResult | null = null;
let stale = false;
let current = 0;
let selected: Placement | null = null;
let playing: number | null = null;
let presets: TransportCase[] = [ROAD];
let containerPresets: ContainerSpec[] = [];
let vehiclePresets: RoadVehicle[] = [];
/** Inputs of the "sea case from ship motion" builder, and its last result. */
const ship: ShipMotion = defaultShipMotion();
let shipCase: ShipCase | null = null;
/** "Best" fill mode: search patterns, orders and orientations (omnipack-opt). */
const search = loadSearchSettings();
/** Plans returned by the last search, best first; `result` is one of them. */
let searchResult: OptimizeResult | null = null;
let searchPick = 0;
let searching = false;

/** Auto: the packer places everything. Manual: the user places units by hand. */
type Mode = "auto" | "manual";
let mode: Mode = "auto";
/** The automatic plan (and its search), kept while the manual one is shown. */
let autoResult: PackResult | null = null;
let autoSearch: OptimizeResult | null = null;
let autoStale = false;
/** The learned ranker, if one has been trained (see the Solutions dialog). */
let model: ModelFile | null = null;

/** The hand-made plan; the engine keeps the same list in its manual session. */
const manual = {
  started: false,
  placements: [] as Placement[],
  /** Earlier placement lists, for undo. */
  history: [] as Placement[][],
  remaining: [] as [string, number][],
  /** Item armed for placing, its allowed poses and the chosen one. */
  item: null as string | null,
  poses: [] as Pose[],
  pose: 0,
  gravity: true,
  magnet: true,
  source: "manual",
};

function loadSearchSettings(): { enabled: boolean; budget: number; securing: number } {
  const fallback = { enabled: false, budget: /Android/i.test(navigator.userAgent) ? 8 : 15, securing: 0.35 };
  try {
    return { ...fallback, ...JSON.parse(localStorage.getItem("omnipack.search") ?? "{}") };
  } catch {
    return fallback;
  }
}

function saveSearchSettings() {
  try {
    localStorage.setItem("omnipack.search", JSON.stringify(search));
  } catch {
    // Storage may be unavailable (private mode); the setting just isn't remembered.
  }
}

/** Legend keys (item ids, stops, …) the user isolated; empty = show all. */
const focusKeys = new Set<string>();
const meshCache = new Map<string, RenderMesh>();

function emptyRequest(): PackRequest {
  return {
    container: {
      id: "container",
      width: 2350,
      height: 2390,
      depth: 5900,
      max_payload: 28000,
      axles: null,
      cog_limits: { max_lateral_offset: null, z_min: null, z_max: null, max_height: null },
      door: null,
      tare_mass: null,
      floor_rating: null,
      vehicle: null,
    },
    items: [],
    options: defaultOptions(),
  };
}

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const viewer = new PlanViewer($<HTMLCanvasElement>("canvas"));

// ---------- DOM helpers ----------

type Attrs = Record<string, string | number | boolean | EventListener | undefined>;

function h(tag: string, attrs: Attrs = {}, ...children: (Node | string | null | undefined)[]): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.replace(/^on/, ""), v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, String(v));
  }
  for (const c of children) if (c != null) el.append(c);
  return el;
}

function changed() {
  if (mode === "manual" && manual.started) {
    // The hand-made plan is re-checked against the new setup right away.
    autoStale = autoResult !== null;
    clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => manualCall("manual_set", { request: withModel(), placements: manual.placements }, false), 250);
    return;
  }
  if (result) {
    stale = true;
    renderResults();
  }
}

let refreshTimer = 0;

/** The request with the learned ranker attached (used by the "learned" fill pattern). */
function withModel(): PackRequest {
  return { ...req, options: { ...req.options, ranker: model?.ranker ?? null } };
}

/** Numeric input bound to a getter/setter. `nullable`: empty = null. */
function num(label: string, get: () => number | null, set: (v: number | null) => void, opts: { nullable?: boolean; min?: number; max?: number; step?: string; placeholder?: string; quiet?: boolean } = {}) {
  const input = h("input", {
    type: "number",
    step: opts.step ?? "any",
    min: opts.min,
    max: opts.max,
    placeholder: opts.placeholder ?? "",
    value: get() ?? "",
  }) as HTMLInputElement;
  input.addEventListener("change", () => {
    const raw = input.value.trim();
    if (raw === "" && opts.nullable) set(null);
    else {
      const v = Number(raw);
      if (Number.isFinite(v)) set(v);
      input.value = String(get() ?? "");
    }
    if (!opts.quiet) changed();
  });
  return h("label", { class: "field" }, h("span", {}, label), input);
}

function text(label: string, get: () => string, set: (v: string) => void) {
  const input = h("input", { type: "text", value: get() }) as HTMLInputElement;
  input.addEventListener("change", () => {
    set(input.value.trim());
    changed();
  });
  return h("label", { class: "field" }, h("span", {}, label), input);
}

function check(label: string, get: () => boolean, set: (v: boolean) => void, title?: string) {
  const input = h("input", { type: "checkbox" }) as HTMLInputElement;
  input.checked = get();
  input.addEventListener("change", () => {
    set(input.checked);
    changed();
  });
  return h("label", { title }, input, label);
}

function select<T extends string>(label: string, options: [T, string][], get: () => T, set: (v: T) => void) {
  const sel = h("select", {}, ...options.map(([v, t]) => h("option", { value: v }, t))) as HTMLSelectElement;
  sel.value = get();
  sel.addEventListener("change", () => {
    set(sel.value as T);
    changed();
  });
  return h("label", { class: "field" }, h("span", {}, label), sel);
}

function slider(label: string, min: number, max: number, step: number, get: () => number, set: (v: number) => void, fmt = (v: number) => v.toFixed(2)) {
  const input = h("input", { type: "range", min, max, step, value: get() }) as HTMLInputElement;
  const out = h("output", {}, fmt(get()));
  input.addEventListener("input", () => {
    set(Number(input.value));
    out.textContent = fmt(Number(input.value));
  });
  input.addEventListener("change", changed);
  return h("div", { class: "field" }, h("span", {}, label), h("div", { class: "slider" }, input, out));
}

// ---------- colours ----------

const PALETTE = ["#4e79a7", "#f28e2b", "#59a14f", "#e15759", "#76b7b2", "#edc948", "#b07aa1", "#ff9da7", "#9c755f", "#bab0ac"];

/** Fixed palette for the first 10 indices, then golden-angle hues so colours stay distinct. */
function paletteColor(i: number): string {
  if (i < PALETTE.length) return PALETTE[i];
  const hue = (i * 137.508) % 360;
  const l = i % 2 ? 0.62 : 0.5;
  const a = 0.55 * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + hue / 30) % 12;
    return Math.round(255 * (l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1))))
      .toString(16)
      .padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

function itemColor(id: string): string {
  const idx = req.items.findIndex((i) => i.id === id);
  return req.items[idx]?.color ?? paletteColor(Math.max(0, idx));
}

/** Green → yellow → red for t in [0, 1]. */
function heat(t: number): string {
  const c = Math.min(1, Math.max(0, t));
  const r = c < 0.5 ? Math.round(510 * c) : 255;
  const g = c < 0.5 ? 200 : Math.round(200 - (c - 0.5) * 400);
  return `#${[r, Math.max(0, g), 70].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function capacityOf(itemId: string): number {
  const s = req.items.find((i) => i.id === itemId);
  if (!s) return Infinity;
  return s.fragile ? 0 : (s.max_load_on_top ?? Infinity);
}

/** Friction of typical cargo on a vehicle floor (EN 12195-1:2010 Annex B, dry). */
const FRICTION_PRESETS: [string, number][] = [
  ["Sawn-wood pallet on plywood", 0.45],
  ["Sawn-wood pallet on grooved aluminium", 0.4],
  ["Sawn-wood pallet on steel sheet", 0.3],
  ["Plastic pallet on plywood", 0.2],
  ["Cardboard on wooden pallet / cardboard", 0.5],
  ["Rough steel on sawn wood", 0.5],
  ["Metal on wooden floor", 0.3],
];
const ANTI_SLIP_FRICTION = 0.6;

const SECURING: [SecuringClass, string, string][] = [
  ["lashing", "#ff5c6c", "needs lashing"],
  ["overloaded", "#b565f0", "stack overloaded"],
  ["chocks", "#f5b041", "needs chocks"],
  ["dunnage", "#4aa3ff", "held once gaps are filled"],
  ["secured", "#3ecf8e", "secured"],
];

/** Largest impact force in a container, kN (at least a tiny positive value). */
function maxImpact(plan: ContainerPlan | null): number {
  return Math.max(1e-9, ...(plan?.placements ?? []).map((p) => p.impact?.force_kn ?? 0));
}

const IMPACT_BUCKETS = ["< 25 %", "25–50 %", "50–75 %", "≥ 75 %"];

/** Transport issues per unit id in the current container. */
function issuesByItem(plan: ContainerPlan | null): Map<string, TransportIssue[]> {
  const m = new Map<string, TransportIssue[]>();
  for (const r of plan?.transport ?? []) for (const i of r.issues) m.set(i.item, [...(m.get(i.item) ?? []), i]);
  return m;
}

const colorMode = () => $<HTMLSelectElement>("color-mode").value;

/** Legend group a placement belongs to in the current colour mode. */
function groupOf(p: Placement): string {
  switch (colorMode()) {
    case "stop":
      return `stop:${p.stop}`;
    case "securing":
      return `sec:${p.securing ?? (p.needs_chocks ? "chocks" : "secured")}`;
    case "impact": {
      const t = (p.impact?.force_kn ?? 0) / maxImpact(currentPlan());
      return `imp:${Math.min(3, Math.floor(t * 4))}`;
    }
    case "item":
      return `item:${p.item_id}`;
    case "floor":
      return `floor:${floorBand(p)}`;
    default:
      return "";
  }
}

/** Floor pressure against the rating (or the highest in the container). */
function floorRatio(p: Placement): number {
  const ref = req.container.floor_rating ?? Math.max(1e-9, ...(currentPlan()?.placements ?? []).map((q) => q.floor_pressure ?? 0));
  return (p.floor_pressure ?? 0) / ref;
}

/** 0 = not on the floor, 1 = below half, 2 = up to the rating, 3 = over it. */
function floorBand(p: Placement): number {
  if (!p.floor_pressure) return 0;
  const t = floorRatio(p);
  return t < 0.5 ? 1 : t <= 1 + 1e-9 ? 2 : 3;
}

function colorFor(p: Placement): string {
  switch (colorMode()) {
    case "stop":
      return paletteColor(p.stop);
    case "load": {
      const cap = capacityOf(p.item_id);
      if (!Number.isFinite(cap)) return heat(0);
      return cap <= 0 ? (p.load_on_top > 0 ? heat(1) : "#6b7280") : heat(p.load_on_top / cap);
    }
    case "margin": {
      if (!Number.isFinite(p.support_margin)) return heat(0);
      const half = Math.min(p.size[0], p.size[2]) / 2;
      return heat(1 - Math.min(1, p.support_margin / (0.5 * half)));
    }
    case "securing":
      return SECURING.find(([k]) => `sec:${k}` === groupOf(p))?.[1] ?? "#3ecf8e";
    case "impact":
      return p.impact ? heat(p.impact.force_kn / maxImpact(currentPlan())) : "#6b7280";
    case "floor":
      return p.floor_pressure ? (floorRatio(p) > 1 + 1e-9 ? "#b565f0" : heat(floorRatio(p))) : "#6b7280";
    default:
      return p.color ?? itemColor(p.item_id);
  }
}

function applyFocus() {
  viewer.setFocus(focusKeys.size ? (p) => focusKeys.has(groupOf(p)) : null);
}

function renderLegend() {
  const legend = $("legend");
  legend.replaceChildren();
  const plan = currentPlan();
  if (!plan) return;
  const row = (color: string, label: string, key?: string) => {
    const r = h("div", { class: `row${key ? " clickable" : ""}${key && focusKeys.size && !focusKeys.has(key) ? " off" : ""}` }, h("span", { class: "swatch", style: `background:${color}` }), label);
    if (key) {
      r.title = "Click to show only this group; click more to add; click again to remove";
      r.addEventListener("click", () => {
        if (focusKeys.has(key)) focusKeys.delete(key);
        else focusKeys.add(key);
        applyFocus();
        renderLegend();
      });
    }
    return r;
  };
  const mode = colorMode();
  const count = (key: string) => plan.placements.filter((p) => groupOf(p) === key).length;
  if (mode === "item") {
    const ids = [...new Set(plan.placements.map((p) => p.item_id))].sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
    for (const id of ids) legend.append(row(itemColor(id), `${id} (${count(`item:${id}`)})`, `item:${id}`));
  } else if (mode === "stop") {
    for (const s of [...new Set(plan.placements.map((p) => p.stop))].sort((a, b) => a - b))
      legend.append(row(paletteColor(s), `${s === 0 ? "no stop" : `stop ${s}`} (${count(`stop:${s}`)})`, `stop:${s}`));
  } else if (mode === "securing") {
    for (const [k, color, label] of SECURING) {
      const n = count(`sec:${k}`);
      if (n || k === "lashing" || k === "secured") legend.append(row(color, `${label} (${n})`, `sec:${k}`));
    }
  } else if (mode === "impact") {
    const max = maxImpact(plan);
    legend.append(h("div", { class: "hint" }, `Transport force per unit, max ${fmt(max, 2)} kN`));
    IMPACT_BUCKETS.forEach((label, b) => legend.append(row(heat((b + 0.5) / 4), `${label} (${count(`imp:${b}`)})`, `imp:${b}`)));
  } else if (mode === "floor") {
    const rating = req.container.floor_rating;
    legend.append(h("div", { class: "hint" }, rating ? `Floor rating ${fmt(rating)} kg/m²` : "No floor rating set: relative to the highest pressure"));
    const labels = ["not on the floor", rating ? "< 50 % of rating" : "< 50 % of max", rating ? "50–100 % of rating" : "50–100 % of max", "over the rating: spread with beams"];
    const colors = ["#6b7280", heat(0.25), heat(0.75), "#b565f0"];
    labels.forEach((label, b) => {
      if (b === 3 && !rating) return;
      legend.append(row(colors[b], `${label} (${count(`floor:${b}`)})`, `floor:${b}`));
    });
  } else if (mode === "load") {
    legend.append(row(heat(0), "unloaded / no limit"), row(heat(0.5), "50% of limit"), row(heat(1), "at limit"), row("#6b7280", "fragile (nothing on top)"));
  } else {
    legend.append(row(heat(0), "large margin"), row(heat(0.5), "moderate"), row(heat(1), "near tipping edge"));
  }
  if (focusKeys.size) {
    legend.append(h("button", { class: "small", onclick: () => { focusKeys.clear(); applyFocus(); renderLegend(); } }, "Show all"));
  } else if (["item", "stop", "securing", "impact", "floor"].includes(mode)) {
    legend.append(h("p", { class: "hint" }, "Click an entry to isolate it"));
  }
}

// ---------- editor ----------

type EditorTab = "container" | "cargo" | "strategy" | "physics" | "place";

/** Remembered layout of the setup panel (this browser only). */
const ui = loadUi();

function loadUi(): { tab: EditorTab; open: Record<string, boolean>; width: number; item: string | null } {
  const fallback = { tab: "container" as EditorTab, open: {} as Record<string, boolean>, width: 380, item: null as string | null };
  try {
    return { ...fallback, ...JSON.parse(localStorage.getItem("omnipack.ui") ?? "{}") };
  } catch {
    return fallback;
  }
}

function saveUi() {
  try {
    localStorage.setItem("omnipack.ui", JSON.stringify(ui));
  } catch {
    // Storage may be unavailable (private mode); the layout just isn't remembered.
  }
}

/** A foldable group with a one-line summary; its open state is remembered. */
function section(key: string, title: string, summary: string, openByDefault: boolean, ...children: (Node | string | null | undefined)[]): HTMLElement {
  const d = h("details", { class: "section" }, h("summary", {}, h("b", {}, title), h("span", { class: "sum" }, summary)), ...children) as HTMLDetailsElement;
  d.open = ui.open[key] ?? openByDefault;
  d.addEventListener("toggle", () => {
    ui.open[key] = d.open;
    saveUi();
  });
  return d;
}

function renderEditor() {
  const units = req.items.reduce((s, i) => s + i.quantity, 0);
  const tabs: [EditorTab, string][] = [["container", "Container"], ["cargo", `Cargo (${units})`], ["strategy", "Strategy"], ["physics", "Physics"]];
  if (mode === "manual") tabs.push(["place", "Place"]);
  if (!tabs.some(([k]) => k === ui.tab)) ui.tab = mode === "manual" ? "place" : "container";
  const body = { container: containerTab, cargo: cargoTab, strategy: strategyTab, physics: physicsTab, place: placeTab }[ui.tab]();
  $("editor").replaceChildren(
    h(
      "nav",
      { class: "editor-tabs" },
      ...tabs.map(([k, label]) =>
        h("button", { class: k === ui.tab ? "active" : "", onclick: () => {
          ui.tab = k;
          saveUi();
          renderEditor();
        } }, label),
      ),
    ),
    ...body,
  );
  prunePreviews(req.items);
  renderManualBar();
}

function containerTab(): (HTMLElement | string)[] {
  const c = req.container;
  const preset = containerPresets.find((p) => p.width === c.width && p.height === c.height && p.depth === c.depth);
  const summary = [
    preset ? (CONTAINER_LABELS[preset.id] ?? preset.id) : c.id,
    `${fmt(c.width)} × ${fmt(c.height)} × ${fmt(c.depth)} mm`,
    c.door ? `door ${fmt(c.door[0])} × ${fmt(c.door[1])}` : "",
    c.tare_mass != null ? `tare ${fmt(c.tare_mass)} kg` : "",
  ].filter(Boolean).join(" · ");
  return [
    section(
      "container.box",
      "Container",
      summary,
      true,
      containerPresetSelect(),
      h(
        "div",
        { class: "grid" },
        text("Name", () => c.id, (v) => (c.id = v || "container")),
        num("Max payload kg", () => c.max_payload, (v) => (c.max_payload = v), { nullable: true, placeholder: "∞" }),
        num("Max CoG offset mm", () => c.cog_limits.max_lateral_offset, (v) => (c.cog_limits.max_lateral_offset = v), { nullable: true, placeholder: "—" }),
        num("Width mm (X)", () => c.width, (v) => (c.width = Math.max(1, v ?? 1)), { min: 1 }),
        num("Height mm (Y)", () => c.height, (v) => (c.height = Math.max(1, v ?? 1)), { min: 1 }),
        num("Depth mm (Z)", () => c.depth, (v) => (c.depth = Math.max(1, v ?? 1)), { min: 1 }),
        num("Door width mm", () => c.door?.[0] ?? null, (v) => setDoor(0, v), { nullable: true, min: 1, placeholder: "no limit" }),
        num("Door height mm", () => c.door?.[1] ?? null, (v) => setDoor(1, v), { nullable: true, min: 1, placeholder: "no limit" }),
        num("Tare kg (for VGM)", () => c.tare_mass, (v) => (c.tare_mass = v === null ? null : Math.max(0, v)), { nullable: true, min: 0, placeholder: "—" }),
        num("Floor rating kg/m²", () => c.floor_rating, (v) => (c.floor_rating = v === null ? null : Math.max(1, v)), { nullable: true, min: 1, placeholder: "—" }),
      ),
      h("p", { class: "hint" }, "The door is at the far end of the depth axis (orange outline); the front wall is at depth 0. Units must pass the door opening standing as loaded."),
    ),
    section("container.vehicle", "Road vehicle", c.vehicle?.name ?? "none", false, vehicleEditor()),
  ];
}

const PATTERNS: [FillBias | "best", string][] = [
  ["best", "★ Best: search all patterns & orders"],
  ["wall_building", "Walls across width"],
  ["floor_first", "Floor layers first"],
  ["longitudinal", "Walls along length"],
  ["lateral", "Floor rows along length"],
  ["corner_first", "From a corner"],
];

const PRIORITIES: [LoadPriority, string][] = [
  ["volume", "Largest first"],
  ["mass", "Heaviest first"],
  ["base_area", "Largest base first"],
  ["height", "Tallest first"],
  ["as_listed", "As listed (grouped)"],
];

function strategyTab(): (HTMLElement | string)[] {
  const o = req.options;
  const patterns: [FillBias | "best", string][] = model ? [...PATTERNS, ["learned", "Learned (from your saved plans)"]] : PATTERNS;
  const pattern = search.enabled ? "★ Best" : (patterns.find(([k]) => k === o.bias)?.[1] ?? o.bias);
  const b = o.balance;
  return [
    section(
      "strategy.order",
      "Order & pattern",
      `${pattern} · ${o.stop_order.toUpperCase()} · ${PRIORITIES.find(([k]) => k === o.priority)?.[1] ?? o.priority}`,
      true,
      h(
        "div",
        { class: "grid two" },
        select<StopOrder>("Unloading order", [["lifo", "LIFO – last stop loaded first"], ["fifo", "FIFO – first stop loaded first"]], () => o.stop_order, (v) => (o.stop_order = v)),
        select<LoadPriority>("Within a stop, load", PRIORITIES, () => o.priority, (v) => (o.priority = v)),
        select<FillBias | "best">(
          "Fill pattern",
          patterns,
          () => (search.enabled ? "best" : o.bias),
          (v) => {
            search.enabled = v === "best";
            if (v !== "best") o.bias = v;
            saveSearchSettings();
            renderEditor();
          },
        ),
        num("Max containers", () => o.max_containers, (v) => (o.max_containers = Math.max(1, Math.round(v ?? 1))), { min: 1, step: "1" }),
      ),
      search.enabled
        ? h(
            "div",
            { class: "search-box" },
            h("p", { class: "hint" }, `Tries every fill pattern${model ? " (including your learned one)" : ""} and load priority, then evolves loading orders and orientations (genetic search + local search). Every plan gets the full physics check; stops, zones and floor-only rules are kept.`),
            slider("Search time", 5, 120, 5, () => search.budget, (v) => ((search.budget = v), saveSearchSettings()), (v) => `${v.toFixed(0)} s`),
            slider("Prefer", 0, 1, 0.05, () => search.securing, (v) => ((search.securing = v), saveSearchSettings()), (v) => (v < 0.2 ? "max. density" : v > 0.8 ? "least securing" : "balanced")),
          )
        : "",
      o.bias === "learned" && !search.enabled
        ? h("p", { class: "hint" }, model ? `Scores positions with the weights learned from ${model.report.steps} decisions in your saved plans.` : "No model trained yet: packs like “Walls across width”.")
        : "",
      h("p", { class: "hint" }, o.stop_order === "fifo"
        ? "FIFO fills from the door towards the back, so the units loaded first are unloaded first (side loading / drive-through)."
        : "LIFO fills from the back wall towards the door; the first stop ends up at the door (rear-door vehicles)."),
    ),
    section(
      "strategy.stability",
      "Stability",
      `margin ${o.stability_margin.toFixed(2)} · support ${Math.round(o.min_support_ratio * 100)} % · balance ${o.balance_weight.toFixed(2)}${o.allow_rotation ? "" : " · no rotation"}`,
      false,
      slider("Stability margin (share of half-footprint)", 0, 0.5, 0.01, () => o.stability_margin, (v) => (o.stability_margin = v)),
      slider("Minimum support area", 0, 1, 0.05, () => o.min_support_ratio, (v) => (o.min_support_ratio = v), (v) => `${Math.round(v * 100)}%`),
      slider("Balance (keep CoG centred)", 0, 1, 0.05, () => o.balance_weight, (v) => (o.balance_weight = v)),
      h("div", { class: "checks" }, check("Allow rotation", () => o.allow_rotation, (v) => (o.allow_rotation = v))),
    ),
    section(
      "strategy.balance",
      "Load balance (CTU Code)",
      `${b.ctu_checks ? `±${Math.round(b.max_eccentricity * 100)} % · ${Math.round(b.min_central_share * 100)} % middle · CoG ≤ ${Math.round(b.max_cog_height * 100)} % H` : "checks off"}${b.centre_lengthwise ? " · centring on" : ""}`,
      false,
      h(
        "div",
        { class: "checks" },
        check("CTU Code balance checks", () => b.ctu_checks, (v) => ((b.ctu_checks = v), renderEditor()), "Warn when the cargo's centre of gravity leaves the CTU Code window, too little mass is in the middle half, or the centre of gravity is high"),
        check("Centre load lengthwise", () => b.centre_lengthwise, (v) => (b.centre_lengthwise = v), "Slide a partial load along the length by the least amount that meets the centre-of-gravity window and the axle limits. The gaps left at the end walls must be braced."),
      ),
      b.ctu_checks ? slider("Max CoG offset (share of length / width)", 0.01, 0.2, 0.01, () => b.max_eccentricity, (v) => (b.max_eccentricity = v), (v) => `±${Math.round(v * 100)} %`) : "",
      b.ctu_checks ? slider("Min mass in the middle half (25–75 % of length)", 0, 1, 0.05, () => b.min_central_share, (v) => (b.min_central_share = v), (v) => `${Math.round(v * 100)} %`) : "",
      b.ctu_checks ? slider("Max CoG height (share of height)", 0.2, 1, 0.05, () => b.max_cog_height, (v) => (b.max_cog_height = v), (v) => `${Math.round(v * 100)} %`) : "",
      b.ctu_checks ? h("p", { class: "hint" }, "An evenly filled container has 50 % of its mass in the middle half, so the 60 % rule asks for heavy units towards the middle.") : "",
    ),
  ];
}

function physicsTab(): (HTMLElement | string)[] {
  const ph = req.options.physics;
  const legs = ph.transport.map((t) => t.name.split(" (")[0]).join(" + ") || "none (static only)";
  const checks = [
    ph.check_sliding && "sliding",
    ph.check_tipping && "tipping",
    ph.avoid_tipping && "avoid tipping",
    ph.dynamic_stacking && "dynamic stacking",
    ph.use_chocks && "chocks",
    ph.secure_load_end && "load end secured",
    ph.anti_slip_mats && "anti-slip mats",
  ].filter(Boolean).join(", ");
  return [
    section(
      "physics.cases",
      "Transport legs",
      legs,
      true,
      h("p", { class: "hint" }, "Always checked: gravity support, tipping at rest, stacking loads, payload. Pick the transport legs to check (quasi-static, EN 12195-1 / CTU Code):"),
      h(
        "div",
        { class: "cases" },
        ...presets.map((pc) => {
          const on = () => ph.transport.some((t) => t.name === pc.name);
          return h(
            "label",
            { title: `forward ${pc.forward} g, backward ${pc.backward} g, sideways ${pc.sideways} g, vertical ${pc.vertical_min}–${pc.vertical_max} g` },
            (() => {
              const cb = h("input", { type: "checkbox" }) as HTMLInputElement;
              cb.checked = on();
              cb.addEventListener("change", () => {
                ph.transport = cb.checked ? [...ph.transport.filter((t) => t.name !== pc.name), pc] : ph.transport.filter((t) => t.name !== pc.name);
                renderEditor();
                changed();
              });
              return cb;
            })(),
            pc.name,
            h("small", {}, ` ${pc.forward}/${pc.backward}/${pc.sideways} g`),
          );
        }),
        ...ph.transport
          .filter((t) => !presets.some((pc) => pc.name === t.name))
          .map((t) =>
            h(
              "label",
              { class: "custom-case", title: `vertical ${t.vertical_min}–${t.vertical_max} g` },
              h("button", { type: "button", class: "small", title: "Remove this case", onclick: () => {
                ph.transport = ph.transport.filter((x) => x !== t);
                renderEditor();
                changed();
              } }, "✕"),
              t.name,
              h("small", {}, ` ${t.forward}/${t.backward}/${t.sideways} g`),
            ),
          ),
      ),
    ),
    section("physics.ship", "Sea case from ship motion", "roll & pitch → accelerations", false, shipMotionEditor()),
    section(
      "physics.checks",
      "Checks",
      checks || "none",
      false,
      h(
        "div",
        { class: "checks" },
        check("Sliding", () => ph.check_sliding, (v) => (ph.check_sliding = v), "Friction vs acceleration, unless blocked by walls or neighbours"),
        check("Tipping", () => ph.check_tipping, (v) => (ph.check_tipping = v), "Tipping moment vs restoring moment, unless blocked above the CoG"),
        check("Avoid tipping", () => ph.avoid_tipping, (v) => (ph.avoid_tipping = v), "Never place a unit where it would tip in transport (e.g. lay slender items down). A unit with no other way to fit is still loaded and flagged for lashing."),
        check("Dynamic stacking", () => ph.dynamic_stacking, (v) => (ph.dynamic_stacking = v), "Multiply loads on top by the vertical factor"),
        check("Chocks for round items", () => ph.use_chocks, (v) => (ph.use_chocks = v), "Lying drums and balls are held by wedges; off = they must be wedged in by neighbours"),
        check("Load end secured", () => ph.secure_load_end, (v) => (ph.secure_load_end = v), "A locking bar / gate / dunnage closes the open end of the load"),
        check("Anti-slip mats", () => ph.anti_slip_mats, (v) => (ph.anti_slip_mats = v), `Rubber mats under every item and between layers: μ ≥ ${ANTI_SLIP_FRICTION}`),
      ),
    ),
    section(
      "physics.friction",
      "Friction & dunnage",
      `μ ${ph.default_friction.toFixed(2)} · dunnage up to ${fmt(ph.max_fill_gap)} mm`,
      false,
      (() => {
        const sel = h("select", {}, h("option", { value: "" }, "Custom"), ...FRICTION_PRESETS.map(([name, mu]) => h("option", { value: String(mu) }, `${name} (μ ${mu})`))) as HTMLSelectElement;
        sel.value = FRICTION_PRESETS.some(([, mu]) => mu === ph.default_friction) ? String(ph.default_friction) : "";
        sel.addEventListener("change", () => {
          if (!sel.value) return;
          ph.default_friction = Number(sel.value);
          renderEditor();
          changed();
        });
        return h("label", { class: "field", title: "Typical dry values from EN 12195-1 Annex B; check them for your load" }, h("span", {}, "Cargo on floor"), sel);
      })(),
      slider("Default friction μ", 0.1, 0.8, 0.05, () => ph.default_friction, (v) => (ph.default_friction = v)),
      slider("Dunnage fills gaps up to", 0, 200, 5, () => ph.max_fill_gap, (v) => (ph.max_fill_gap = v), (v) => `${v.toFixed(0)} mm`),
    ),
    section(
      "physics.lashing",
      "Direct lashing (EN 12195-1)",
      `LC ${fmt(ph.lashing.capacity_dan)} daN · points ${fmt(ph.lashing.anchor_dan)} daN · α ${fmt(ph.lashing.vertical_angle)}° β ${fmt(ph.lashing.horizontal_angle)}°`,
      false,
      h(
        "div",
        { class: "grid two" },
        num("Lashing capacity LC daN", () => ph.lashing.capacity_dan, (v) => (ph.lashing.capacity_dan = Math.max(1, v ?? 1)), { min: 1 }),
        num("Lashing points daN", () => ph.lashing.anchor_dan, (v) => (ph.lashing.anchor_dan = Math.max(1, v ?? 1)), { min: 1 }),
        num("Angle to floor α °", () => ph.lashing.vertical_angle, (v) => (ph.lashing.vertical_angle = Math.min(89, Math.max(0, v ?? 0))), { min: 0, max: 89 }),
        num("Angle to direction β °", () => ph.lashing.horizontal_angle, (v) => (ph.lashing.horizontal_angle = Math.min(89, Math.max(0, v ?? 0))), { min: 0, max: 89 }),
      ),
      h("p", { class: "hint" }, "Each securing force is also given as a number of direct lashings: n·LC ≥ m·g·(c − μ·0.75·c_z) / (μ·0.75·sin α + cos α·cos β). The weaker of strap and lashing point counts (ISO container floor points: 1000 daN)."),
    ),
  ];
}

/** Short description of a shape for the cargo list. */
function shapeSummary(s: Shape): string {
  switch (s.kind) {
    case "box":
      return `box ${fmt(s.w)}×${fmt(s.h)}×${fmt(s.d)}`;
    case "cylinder":
      return `drum r${fmt(s.radius)} × ${fmt(s.length)}`;
    case "sphere":
      return `ball r${fmt(s.radius)}`;
    case "cone":
      return `cone r${fmt(s.radius)} h${fmt(s.height)}`;
    case "pyramid":
      return `pyramid ${fmt(s.w)}×${fmt(s.d)} h${fmt(s.height)}`;
    case "prism":
      return `${s.sides}-prism r${fmt(s.radius)} × ${fmt(s.length)}`;
    case "l_profile":
      return `L ${fmt(s.a)}×${fmt(s.b)}×${fmt(s.thickness)} × ${fmt(s.length)}`;
  }
}

function cargoTab(): (HTMLElement | string)[] {
  const add = h(
    "select",
    { title: "Add an item of this shape" },
    h("option", { value: "" }, "+ Add item…"),
    ...SHAPE_KINDS.map(([k, t]) => h("option", { value: k }, t)),
    ...PALLETS.map(([k, t]) => h("option", { value: k }, t)),
  ) as HTMLSelectElement;
  add.addEventListener("change", () => {
    if (add.value.startsWith("pallet:")) addPallet(add.value);
    else if (add.value) addItem(add.value as ShapeKind);
    add.value = "";
  });
  const list = h("div", { class: "cargo-list" }, ...req.items.map(cargoRow));
  const filter = h("input", { type: "search", placeholder: "Filter items…" }) as HTMLInputElement;
  filter.addEventListener("input", () => {
    const q = filter.value.trim().toLowerCase();
    for (const el of list.children) (el as HTMLElement).hidden = !!q && !((el as HTMLElement).dataset.id ?? "").toLowerCase().includes(q);
  });
  const mass = req.items.reduce((s, i) => s + i.mass * i.quantity, 0);
  return [
    h("div", { class: "cargo-head" }, add, req.items.length > 6 ? filter : h("span", { class: "hint" }, req.items.length ? `${req.items.length} types · ${fmt(mass)} kg` : "")),
    req.items.length ? list : h("p", { class: "hint" }, "No items yet: add one, or load a sample."),
  ];
}

function cargoRow(it: ItemSpec, index: number): HTMLElement {
  const color = it.color ?? paletteColor(index);
  const open = ui.item === it.id;
  const row = h(
    "div",
    { class: `cargo-row${open ? " open" : ""}`, style: `border-left-color:${color}` },
    h(
      "button",
      { class: "row-main", title: open ? "Close" : "Edit this item", onclick: () => {
        ui.item = open ? null : it.id;
        saveUi();
        renderEditor();
      } },
      h("span", { class: "swatch", style: `background:${color}` }),
      h("b", {}, it.id),
      h("span", { class: "muted" }, shapeSummary(it.shape)),
      h("span", { class: "qty" }, `×${it.quantity}`),
      h("span", { class: "muted" }, `${fmt(it.mass, 1)} kg`),
    ),
    h("button", { class: "small", title: "Duplicate", onclick: () => duplicateItem(index) }, "⧉"),
    h("button", { class: "small", title: "Remove", onclick: () => {
      req.items.splice(index, 1);
      renderEditor();
      changed();
    } }, "✕"),
  );
  const wrap = h("div", { class: "cargo-entry" }, row, open ? itemCard(it, index) : null);
  wrap.dataset.id = it.id;
  return wrap;
}

function duplicateItem(index: number) {
  const copy: ItemSpec = JSON.parse(JSON.stringify(req.items[index]));
  copy.id = `${copy.id}-copy`;
  while (req.items.some((i) => i.id === copy.id)) copy.id += "'";
  req.items.splice(index + 1, 0, copy);
  ui.item = copy.id;
  saveUi();
  renderEditor();
  changed();
}

function placeTab(): (HTMLElement | string)[] {
  const left = new Map(manual.remaining);
  const quiet = (label: string, get: () => boolean, set: (v: boolean) => void, title: string) => {
    const cb = h("input", { type: "checkbox" }) as HTMLInputElement;
    cb.checked = get();
    cb.addEventListener("change", () => set(cb.checked));
    return h("label", { title }, cb, label);
  };
  return [
    h("p", { class: "hint" }, TOUCH
      ? "Pick a unit, then tap in the 3D view to place it. To move a unit, choose “Select / move” in the bar over the 3D view, then drag it. Tap a unit to select it: ⟳ rotates, ✕ removes, ↶ undoes, and Results has exact X / Y / Z fields."
      : "Pick a unit, then click in the 3D view to place it. Drag a placed unit to move it; click one to select it. Keys: R rotates, arrows nudge (Shift ×10), Delete removes, Ctrl+Z undoes, Esc drops the unit you hold."),
    h(
      "div",
      { class: "place-list" },
      ...req.items.map((it, i) =>
        h(
          "button",
          { class: `place-item${manual.item === it.id ? " active" : ""}`, disabled: (left.get(it.id) ?? 0) <= 0, onclick: () => armItem(it.id) },
          h("span", { class: "swatch", style: `background:${it.color ?? paletteColor(i)}` }),
          h("b", {}, it.id),
          h("span", { class: "muted" }, shapeSummary(it.shape)),
          h("span", { class: "qty" }, `${left.get(it.id) ?? 0} left`),
        ),
      ),
    ),
    manual.item && manual.poses.length
      ? h(
          "div",
          { class: "poses" },
          h("span", { class: "hint" }, "Orientation (W × H × D in the container):"),
          ...manual.poses.map((p, k) =>
            h("button", { class: `small${k === manual.pose ? " active" : ""}`, onclick: () => {
              manual.pose = k;
              renderEditor();
            } }, p.extents.map((e) => fmt(e)).join(" × ")),
          ),
        )
      : "",
    h(
      "div",
      { class: "checks" },
      quiet("Snap down with gravity", () => manual.gravity, (v) => (manual.gravity = v), "Off: units stay at the height you point at or type (they may float; that is flagged)"),
      quiet("Snap to walls and faces", () => manual.magnet, (v) => (manual.magnet = v), "Within 30 mm of a wall or a unit's face, the unit lines up with it"),
    ),
    h(
      "div",
      { class: "actions" },
      h("button", { class: "primary", onclick: manualAutoFill, title: "Pack the units still to place around yours" }, "Auto-fill the rest"),
      h("button", { onclick: undoManual, disabled: !manual.history.length }, "Undo"),
      h("button", { onclick: clearManual, disabled: !manual.placements.length }, "Clear"),
      h("button", { onclick: saveSolution, disabled: !manual.placements.length }, "Save solution…"),
    ),
  ];
}

/** Loaded pallets: footprint per EPAL / ISO, 1 m high, 500 kg, this side up, wood on plywood. */
const PALLETS: [string, string, number][] = [
  ["pallet:epal1", "Euro pallet EPAL 1 (1200 × 800)", 800],
  ["pallet:epal2", "Industrial pallet EPAL 2 (1200 × 1000)", 1000],
];

function addPallet(key: string) {
  const [, label, width] = PALLETS.find(([k]) => k === key)!;
  const it = newItem(req.items.length + 1);
  it.id = key === "pallet:epal1" ? "epal1" : "epal2";
  while (req.items.some((i) => i.id === it.id)) it.id += "'";
  it.shape = { kind: "box", w: width, h: 1000, d: 1200 };
  it.mass = 500;
  it.upright_only = true;
  it.friction = 0.45;
  req.items.push(it);
  ui.item = it.id;
  ui.tab = "cargo";
  saveUi();
  renderEditor();
  changed();
  setStatus(`Added ${label}: set its height and mass.`);
}

function setDoor(k: 0 | 1, v: number | null) {
  const c = req.container;
  const door: [number | null, number | null] = [c.door?.[0] ?? null, c.door?.[1] ?? null];
  door[k] = v === null ? null : Math.max(1, v);
  c.door = door[0] === null && door[1] === null ? null : [door[0] ?? c.width, door[1] ?? c.height];
}

const CONTAINER_LABELS: Record<string, string> = {
  "20ft-dv": "20 ft standard (20' DV)",
  "40ft-dv": "40 ft standard (40' DV)",
  "40ft-hc": "40 ft high cube (40' HC)",
  "semi-trailer-13.6": "Semi-trailer 13.6 m (side loading)",
};

function containerPresetSelect(): HTMLElement {
  const c = req.container;
  const sel = h(
    "select",
    {},
    h("option", { value: "" }, "Custom"),
    ...containerPresets.map((p) => h("option", { value: p.id }, CONTAINER_LABELS[p.id] ?? p.id)),
  ) as HTMLSelectElement;
  sel.value = containerPresets.find((p) => p.width === c.width && p.height === c.height && p.depth === c.depth)?.id ?? "";
  sel.addEventListener("change", () => {
    const p = containerPresets.find((x) => x.id === sel.value);
    if (!p) return;
    // Axle positions belong to the previous body; the road vehicle is kept.
    Object.assign(c, { id: p.id, width: p.width, height: p.height, depth: p.depth, max_payload: p.max_payload, door: p.door ? [...p.door] : null, tare_mass: p.tare_mass, floor_rating: p.floor_rating, axles: null });
    renderEditor();
    changed();
  });
  return h("label", { class: "field", title: "Typical inside sizes, door opening, tare and payload (30 480 kg gross). Check the CSC plate of the actual unit." }, h("span", {}, "Type"), sel);
}

const VEHICLE_FIELDS: [keyof RoadVehicle, string][] = [
  ["container_front", "Kingpin → front wall mm"],
  ["trailer_wheelbase", "Kingpin → trailer axles mm"],
  ["trailer_tare", "Trailer tare kg"],
  ["trailer_cog", "Kingpin → trailer CoG mm"],
  ["tractor_tare", "Tractor tare kg"],
  ["tractor_wheelbase", "Tractor wheelbase mm"],
  ["tractor_cog", "Steer axle → tractor CoG mm"],
  ["fifth_wheel", "Steer axle → kingpin mm"],
  ["max_steer", "Max steer axle kg"],
  ["max_drive", "Max drive axle kg"],
  ["max_trailer", "Max trailer axles kg"],
  ["max_gross", "Max gross kg"],
];

function vehicleEditor(): HTMLElement {
  const c = req.container;
  const sel = h("select", {}, h("option", { value: "" }, "None"), ...vehiclePresets.map((v) => h("option", { value: v.name }, v.name))) as HTMLSelectElement;
  sel.value = c.vehicle ? (vehiclePresets.some((v) => v.name === c.vehicle!.name) ? c.vehicle.name : "") : "";
  if (c.vehicle && !sel.value) sel.append(h("option", { value: "__custom" }, c.vehicle.name || "Custom vehicle"));
  if (c.vehicle && !sel.value) sel.value = "__custom";
  sel.addEventListener("change", () => {
    const v = vehiclePresets.find((x) => x.name === sel.value);
    if (sel.value !== "__custom") c.vehicle = v ? { ...v } : null;
    renderEditor();
    changed();
  });
  const v = c.vehicle;
  return h(
    "div",
    {},
    h("label", { class: "field", title: "Axle loads of a tractor + semi-trailer with the container (EU limits: Directive 96/53/EC as amended by 2015/719)" }, h("span", {}, "Vehicle"), sel),
    v
      ? h(
          "details",
          { class: "sub" },
          h("summary", {}, "Vehicle geometry and limits"),
          h("div", { class: "grid two" }, ...VEHICLE_FIELDS.map(([k, label]) => num(label, () => v[k] as number, (x) => ((v[k] as number) = x ?? 0)))),
          h("p", { class: "hint" }, "Distances along the vehicle, rearwards positive. The container tare counts at mid-length; set the tare above."),
        )
      : "",
  );
}

function shipMotionEditor(): HTMLElement {
  const out = h("p", { class: "hint" });
  const show = () => {
    if (!shipCase) return void (out.textContent = "");
    const t = shipCase.case;
    out.textContent = `Roll period ${fmt(shipCase.roll_period, 1)} s → fore/aft ${t.forward} g, sideways ${t.sideways} g, vertical ${t.vertical_min}–${t.vertical_max} g.${shipCase.notes.length ? ` ⚠ ${shipCase.notes.join("; ")}.` : ""}`;
  };
  const update = async () => {
    try {
      shipCase = await invoke<ShipCase>("ship_motion_case", { motion: ship });
    } catch (e) {
      shipCase = null;
      out.textContent = String(e);
      return;
    }
    show();
  };
  const field = (label: string, k: keyof ShipMotion, title?: string) => {
    const f = num(label, () => ship[k], (v) => {
      ship[k] = Math.max(0, v ?? 0);
      update();
    }, { min: 0, quiet: true });
    if (title) f.title = title;
    return f;
  };
  show();
  if (!shipCase) update();
  return h(
    "div",
    {},
    h(
      "div",
      { class: "grid two" },
      field("Beam B m", "beam"),
      field("GM m", "gm", "Metacentric height: large = stiff ship (short, violent roll), small = tender ship"),
      field("Roll coefficient c", "roll_coeff", "Roll period T = 2·c·B / √GM, c ≈ 0.38–0.42"),
      field("Roll amplitude °", "roll_deg"),
      field("Pitch amplitude °", "pitch_deg"),
      field("Pitch period s", "pitch_period"),
      field("Height above roll axis m", "height", "Stowage height of the container above the ship's roll and pitch axes"),
      field("Distance from midship m", "from_midship", "Towards bow or stern: pitch adds vertical acceleration there"),
      field("Surge g", "surge_g"),
      field("Heave g", "heave_g"),
    ),
    out,
    h("button", { type: "button", class: "small", onclick: () => {
      if (!shipCase) return;
      const t = shipCase.case;
      req.options.physics.transport = [...req.options.physics.transport.filter((x) => x.name !== t.name), { ...t }];
      renderEditor();
      changed();
    } }, "Add this case"),
  );
}

function addItem(kind: ShapeKind) {
  const it = newItem(req.items.length + 1);
  while (req.items.some((i) => i.id === it.id)) it.id += "'";
  it.shape = defaultShape(kind);
  req.items.push(it);
  ui.item = it.id;
  ui.tab = "cargo";
  saveUi();
  renderEditor();
  changed();
}

function shapeFields(s: Shape): HTMLElement[] {
  const pos = (label: string, get: () => number, set: (v: number) => void) => num(label, get, (v) => set(Math.max(1, v ?? 1)), { min: 1 });
  switch (s.kind) {
    case "box":
      return [pos("W mm", () => s.w, (v) => (s.w = v)), pos("H mm", () => s.h, (v) => (s.h = v)), pos("D mm", () => s.d, (v) => (s.d = v))];
    case "cylinder":
      return [pos("Radius mm", () => s.radius, (v) => (s.radius = v)), pos("Length mm", () => s.length, (v) => (s.length = v)), h("span")];
    case "sphere":
      return [pos("Radius mm", () => s.radius, (v) => (s.radius = v)), h("span"), h("span")];
    case "cone":
      return [pos("Base radius mm", () => s.radius, (v) => (s.radius = v)), pos("Height mm", () => s.height, (v) => (s.height = v)), h("span")];
    case "pyramid":
      return [pos("Base W mm", () => s.w, (v) => (s.w = v)), pos("Base D mm", () => s.d, (v) => (s.d = v)), pos("Height mm", () => s.height, (v) => (s.height = v))];
    case "prism":
      return [
        num("Sides", () => s.sides, (v) => (s.sides = Math.min(64, Math.max(3, Math.round(v ?? 3)))), { min: 3, max: 64, step: "1" }),
        pos("Radius mm", () => s.radius, (v) => (s.radius = v)),
        pos("Length mm", () => s.length, (v) => (s.length = v)),
      ];
    case "l_profile":
      return [
        pos("Leg A mm", () => s.a, (v) => (s.a = v)),
        pos("Leg B mm", () => s.b, (v) => (s.b = v)),
        pos("Thickness mm", () => s.thickness, (v) => (s.thickness = Math.min(v, Math.min(s.a, s.b) - 1))),
        pos("Length mm", () => s.length, (v) => (s.length = v)),
        h("span"),
        h("span"),
      ];
  }
}

function itemCard(it: ItemSpec, index: number): HTMLElement {
  const color = h("input", { type: "color", value: it.color ?? paletteColor(index) }) as HTMLInputElement;
  color.addEventListener("change", () => {
    it.color = color.value;
    renderEditor();
    changed();
  });
  const idInput = h("input", { type: "text", value: it.id }) as HTMLInputElement;
  idInput.addEventListener("change", () => {
    const v = idInput.value.trim();
    if (!v || req.items.some((o) => o !== it && o.id === v)) idInput.value = it.id;
    else it.id = v;
    changed();
  });
  const shapeSel = h("select", {}, ...SHAPE_KINDS.map(([k, t]) => h("option", { value: k }, t))) as HTMLSelectElement;
  shapeSel.value = it.shape.kind;
  shapeSel.addEventListener("change", () => {
    it.shape = defaultShape(shapeSel.value as ShapeKind);
    renderEditor();
    changed();
  });
  const remove = h("button", { class: "small", title: "Remove", onclick: () => {
    req.items.splice(index, 1);
    renderEditor();
    changed();
  } }, "✕");

  const s = it.shape;
  const roundish = s.kind === "cylinder" || s.kind === "sphere";
  // Centre of mass as X/Y/Z from the base corner; filled in once the preview
  // knows the item's box and uniform centre (the engine stores the offset).
  const comRow = h("div", { class: "com-row" });
  const fillCom = (info: ItemPreview) => {
    const axis = (k: number, label: string) =>
      num(label, () => (it.com_offset[k] === 0 ? null : Math.round((info.centroid[k] + it.com_offset[k]) * 10) / 10), (v) => {
        it.com_offset[k] = v === null ? 0 : Math.min(info.extents[k], Math.max(0, v)) - info.centroid[k];
      }, { nullable: true, min: 0, max: info.extents[k], placeholder: String(Math.round(info.centroid[k])) });
    const reset = h("button", { type: "button", class: "small", title: "Back to the geometric centre of mass", onclick: () => {
      it.com_offset = [0, 0, 0];
      changed();
      preview.refresh();
    } }, "↺");
    comRow.replaceChildren(
      h("span", { class: "com-title", title: "Measured from the bottom-left-back corner of the unrotated item (Y = height above its base). Empty = geometric centre." }, "Centre of mass, mm from base corner"),
      h("div", { class: "grid com" }, axis(0, "X (W)"), axis(1, "Y (H)"), axis(2, "Z (D)"), reset),
    );
  };
  const dims = h("small", { class: "dims" });
  let comFor = "";
  const preview = previewCanvas(it, it.color ?? paletteColor(index), (info) => {
    dims.textContent = info.extents.map((e) => fmt(e, 0)).join(" × ") + " mm";
    // Rebuild the inputs only when the box changed, so typing keeps focus.
    const key = JSON.stringify([info.extents, info.centroid]);
    if (s.kind !== "sphere" && key !== comFor) {
      comFor = key;
      fillCom(info);
    }
  });
  return h(
    "div",
    { class: "card", style: `border-left-color:${it.color ?? paletteColor(index)}`, onchange: () => preview.refresh() },
    h("div", { class: "card-head" }, color, idInput, shapeSel, remove),
    h("div", { class: "card-body" },
      h("div", { class: "preview-box" }, preview.canvas, dims),
      h(
      "div",
      { class: "grid" },
      ...shapeFields(s),
      num("Mass kg", () => it.mass, (v) => (it.mass = Math.max(0, v ?? 0)), { min: 0 }),
      num("Quantity", () => it.quantity, (v) => (it.quantity = Math.max(0, Math.round(v ?? 0))), { min: 0, step: "1" }),
      num("Max load on top kg", () => it.max_load_on_top, (v) => (it.max_load_on_top = v), { nullable: true, placeholder: "∞" }),
      num("Stop (1 = first off)", () => it.stop, (v) => (it.stop = Math.max(0, Math.round(v ?? 0))), { min: 0, step: "1" }),
      select<Zone>("Zone", [["any", "Anywhere"], ["back", "Back"], ["front", "Near door"]], () => it.zone, (v) => (it.zone = v)),
      num("Friction μ", () => it.friction, (v) => (it.friction = v === null ? null : Math.max(0, v)), { nullable: true, min: 0, placeholder: String(req.options.physics.default_friction) }),
      ),
    ),
    comRow,
    h(
      "div",
      { class: "checks" },
      check("Fragile", () => it.fragile, (v) => (it.fragile = v)),
      s.kind === "sphere" || s.kind === "cone"
        ? null
        : check(roundish ? "Upright only" : "This side up", () => it.upright_only, (v) => (it.upright_only = v)),
      check("Floor only", () => it.floor_only, (v) => (it.floor_only = v)),
    ),
  );
}

// ---------- results ----------

function currentPlan(): ContainerPlan | null {
  return result?.containers[current] ?? null;
}

const fmt = (v: number, d = 0) => v.toLocaleString(undefined, { maximumFractionDigits: d, minimumFractionDigits: d });
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

function describeViolation(v: Record<string, unknown>): string {
  const { kind, ...rest } = v;
  return `${String(kind).replace(/_/g, " ")}: ${Object.values(rest).map((x) => (typeof x === "number" ? fmt(x, 1) : String(x))).join(", ")}`;
}

const DIR_LABEL: Record<Direction, string> = { forward: "forward (braking)", backward: "backward", left: "left", right: "right" };

function describeIssue(i: TransportIssue): string {
  if (i.kind === "stack_overload") return `${i.item}: stack overloaded by ${fmt(i.required, 1)} kg at ${i.acceleration} g`;
  const force = i.required < 0.01 ? "< 0.01" : fmt(i.required, 2);
  const l = req.options.physics.lashing;
  const lash = i.lashings ? ` or ${i.lashings} direct lashing${i.lashings > 1 ? "s" : ""} (${fmt(Math.min(l.capacity_dan, l.anchor_dan))} daN)` : "";
  return `${i.item}: ${i.kind} ${DIR_LABEL[i.direction!]} at ${i.acceleration} g → block with ≥ ${force} kN${lash}`;
}

function describeBalance(i: BalanceIssue): string {
  switch (i.kind) {
    case "cog_lengthwise":
      return `centre of gravity ${fmt(Math.abs(i.offset))} mm towards the ${i.offset > 0 ? "door" : "front wall"} (limit ${fmt(i.limit)} mm)`;
    case "cog_lateral":
      return `centre of gravity ${fmt(Math.abs(i.offset))} mm to the ${i.offset > 0 ? "right" : "left"} (limit ${fmt(i.limit)} mm)`;
    case "central_share":
      return `only ${fmt(i.share * 100)} % of the mass in the middle half (min ${fmt(i.min * 100)} %)`;
    case "cog_height":
      return `centre of gravity ${fmt(i.height)} mm high (limit ${fmt(i.limit)} mm)`;
    case "axle_overload":
      return `${i.axle}: ${fmt(i.load)} kg over the ${fmt(i.max)} kg limit`;
    case "floor_pressure":
      return `${i.item}: ${fmt(i.pressure)} kg/m² on a ${fmt(i.limit)} kg/m² floor → spread over ≥ ${fmt(i.spread_area, 2)} m² with beams`;
  }
}

/** Container-level balance warnings (floor pressure counted apart). */
const balanceWarnings = (c: ContainerPlan) => (c.balance?.issues ?? []).filter((i) => i.kind !== "floor_pressure").length;
const floorWarnings = (c: ContainerPlan) => (c.balance?.issues ?? []).filter((i) => i.kind === "floor_pressure").length;

function balanceSection(plan: ContainerPlan): (HTMLElement | string)[] {
  const b = plan.balance;
  if (!b) return [];
  const [W, H, D] = plan.size;
  const o = req.options.balance;
  const mark = (ok: boolean, checked = true) => (checked ? (ok ? "✓ " : "⚠ ") : "");
  const ctu = o.ctu_checks;
  const limL = o.max_eccentricity * D;
  const limW = o.max_eccentricity * W;
  const fill = req.options.physics.max_fill_gap;
  const sign = (v: number) => (v > 0 ? "+" : v < 0 ? "−" : "") + fmt(Math.abs(v));
  const out: (HTMLElement | string)[] = [
    h("h3", {}, "Load balance", h("span", { class: "spacer" }), ctu || b.vehicle ? h("span", { class: `badge ${balanceWarnings(plan) ? "warn" : "ok"}` }, balanceWarnings(plan) ? `${balanceWarnings(plan)} warnings` : "OK") : null),
    stat(`${mark(Math.abs(b.lengthwise_offset) <= limL + 1e-6, ctu)}CoG lengthwise`, `${sign(b.lengthwise_offset)} mm (${fmt((b.lengthwise_offset / D) * 100, 1)} % of L${ctu ? `, limit ±${fmt(o.max_eccentricity * 100)} %` : ""})`),
    stat(`${mark(Math.abs(b.lateral_offset) <= limW + 1e-6, ctu)}CoG sideways`, `${sign(b.lateral_offset)} mm (${fmt((b.lateral_offset / W) * 100, 1)} % of W)`),
    stat(`${mark(b.central_share >= o.min_central_share - 1e-9, ctu)}Middle half (25–75 % L)`, `${fmt(b.central_share * 100)} % of the mass${ctu ? ` (min ${fmt(o.min_central_share * 100)} %)` : ""}`),
    stat(`${mark(b.cog_height_ratio <= o.max_cog_height + 1e-9, ctu)}CoG height`, `${fmt(b.cog_height_ratio * 100)} % of H (${fmt(b.cog_height_ratio * H)} mm)`),
    stat("Front half / door half", `${fmt(b.half_shares[0] * 100)} % / ${fmt(b.half_shares[1] * 100)} %`),
    b.shift ? stat("Moved lengthwise", `${sign(b.shift)} mm towards the ${b.shift > 0 ? "door" : "front wall"}`) : "",
    stat("Free length front / door", `${fmt(b.end_gaps[0])}${b.end_gaps[0] > fill ? " (brace)" : ""} / ${fmt(b.end_gaps[1])}${b.end_gaps[1] > fill ? " (brace)" : ""} mm`),
    b.vgm != null ? stat("VGM (tare + cargo)", `${fmt(b.vgm)} kg + dunnage & lashing`) : "",
  ];
  if (b.vehicle) {
    const v = b.vehicle;
    const names = ["Steer axle", "Drive axle", "Trailer axles", "Gross mass"];
    [v.steer, v.drive, v.trailer, v.gross].forEach((load, k) => out.push(stat(`${mark(load <= v.limits[k] + 1e-6)}${names[k]}`, `${fmt(load)} / ${fmt(v.limits[k])} kg`)));
  }
  const floor = b.issues.filter((i) => i.kind === "floor_pressure");
  if (floor.length) {
    out.push(
      h(
        "details",
        {},
        h("summary", {}, `⚠ ${floor.length} units over the floor rating: spread their load with beams`),
        h("ul", { class: "issue-list" }, ...floor.slice(0, 30).map((i) => h("li", {}, describeBalance(i))), floor.length > 30 ? h("li", {}, `… ${floor.length - 30} more`) : null),
      ),
    );
  }
  const other = b.issues.filter((i) => i.kind === "axle_overload" || i.kind === "cog_lengthwise");
  if (other.length && !o.centre_lengthwise && b.end_gaps[0] + b.end_gaps[1] > 1) {
    out.push(h("p", { class: "hint" }, "Tip: \"Centre load lengthwise\" slides this load to meet the window and the axle limits."));
  }
  return out;
}

function scoreLine(s: Score): string {
  const lash = s.lashing_units ? `${s.lashing_units} to lash (${fmt(s.lashing_kn, 1)} kN)` : "nothing to lash";
  const tip = s.tipping_units ? `${s.tipping_units} may tip · ` : "";
  const bal = s.balance_issues ? ` · ${s.balance_issues} balance warn.` : "";
  const floor = s.floor_overloads ? ` · ${s.floor_overloads} over floor rating` : "";
  return `${fmt(s.volume_utilization * 100, 1)} % vol · ${s.containers} cont. · ${tip}${lash} · dunnage ${fmt(Math.max(0, s.dunnage_mm) / 1000, 2)} m${bal}${floor}`;
}

/** The distinct plans a search returned, with the plain placer for comparison. */
function searchPicker(r: OptimizeResult): HTMLElement {
  return h(
    "div",
    { class: "search-picker" },
    h("p", { class: "hint" }, `Search: ${r.evaluated} plans in ${fmt(r.elapsed_ms / 1000, 1)} s. Your settings alone: ${scoreLine(r.baseline)}`),
    ...r.solutions.map((s, i) =>
      h(
        "button",
        {
          class: `pick${i === searchPick ? " active" : ""}`,
          title: s.label,
          onclick: async () => {
            searchPick = i;
            result = s.result;
            current = 0;
            selected = null;
            focusKeys.clear();
            await showPlan();
            renderResults();
          },
        },
        h("b", {}, `Plan ${i + 1}`),
        ` ${scoreLine(s.score)}`,
      ),
    ),
  );
}

function renderResults() {
  const panel = $("results");
  panel.replaceChildren();
  if (!result) {
    panel.append(h("h3", {}, "Results"), h("p", { class: "hint" }, "No plan yet."));
    return;
  }
  const r = result;
  const valid = r.containers.every((c) => c.violations.length === 0);
  const countClass = (k: SecuringClass) => r.containers.reduce((n, c) => n + c.placements.filter((p) => p.securing === k).length, 0);
  const unsecured = countClass("lashing") + countClass("overloaded");
  const dunnage = countClass("dunnage");
  const chocks = r.containers.reduce((n, c) => n + c.placements.filter((p) => p.needs_chocks).length, 0);
  const tipping = new Set(r.containers.flatMap((c) => c.transport.flatMap((t) => t.issues.filter((i) => i.kind === "tipping").map((i) => i.item)))).size;
  const balance = r.containers.reduce((n, c) => n + balanceWarnings(c), 0);
  const floor = r.containers.reduce((n, c) => n + floorWarnings(c), 0);
  panel.append(
    h("h3", {}, mode === "manual" ? "Manual plan" : "Plan", h("span", { class: "spacer" }), stale ? h("span", { class: "badge warn" }, "out of date") : null),
    h(
      "div",
      { class: "actions" },
      h("span", { class: `badge ${valid ? "ok" : "bad"}` }, valid ? "✓ Stable at rest" : mode === "manual" ? `✗ ${plural(r.containers.reduce((n, c) => n + c.violations.length, 0), "problem")} flagged` : "✗ Violations found"),
      req.options.physics.transport.length
        ? h("span", { class: `badge ${unsecured ? "warn" : "ok"}` }, unsecured ? `⚠ ${unsecured} unit${unsecured === 1 ? " needs" : "s need"} lashing` : "✓ Secured for transport")
        : null,
      tipping ? h("span", { class: "badge warn", title: "No position without tipping was found for these units: lash them" }, `${tipping} would tip`) : null,
      dunnage ? h("span", { class: "badge" }, `${dunnage} held once gaps are filled`) : null,
      chocks ? h("span", { class: "badge warn" }, `${chocks} need chocks`) : null,
      req.options.balance.ctu_checks || req.container.vehicle
        ? h("span", { class: `badge ${balance ? "warn" : "ok"}`, title: "CTU Code centre-of-gravity window, middle-half share, CoG height and vehicle axle limits" }, balance ? `⚠ ${plural(balance, "balance warning")}` : "✓ Balanced")
        : null,
      floor ? h("span", { class: "badge warn", title: "Contact pressure above the floor rating: spread the load with beams" }, `${floor} over floor rating`) : null,
    ),
    searchResult ? searchPicker(searchResult) : "",
    stat(mode === "manual" ? "Units placed" : "Units packed", `${r.packed_units} / ${r.requested_units}`),
    stat("Containers", String(r.containers.length)),
    stat("Volume used", `${fmt(r.volume_utilization * 100, 1)} %`),
    stat("Computed in", `${r.elapsed_ms} ms`),
  );

  const plan = currentPlan();
  if (plan) {
    const m = plan.metrics;
    panel.append(
      h("h3", {}, `Container ${current + 1}: ${plan.id}`),
      stat("Items", String(m.item_count)),
      stat("Volume used", `${fmt(m.volume_utilization * 100, 1)} %`),
      stat("Cargo mass", `${fmt(m.total_mass)} kg${m.weight_utilization != null ? ` (${fmt(m.weight_utilization * 100, 0)} %)` : ""}`),
      stat("Centre of gravity", `${fmt(m.center_of_mass[0])}, ${fmt(m.center_of_mass[1])}, ${fmt(m.center_of_mass[2])} mm`),
      stat("Lateral CoG offset", `${fmt(m.lateral_offset)} mm`),
      m.axle_loads ? stat("Axle loads", `${fmt(m.axle_loads[0])} / ${fmt(m.axle_loads[1])} kg`) : "",
      stat("Unloading accessibility", `${fmt(m.accessibility * 100)} %`),
      stat("Smallest stability margin", Number.isFinite(m.min_support_margin) ? `${fmt(m.min_support_margin, 1)} mm` : "—"),
      ...balanceSection(plan),
    );
    if (plan.violations.length) {
      panel.append(h("h3", {}, "Violations"), h("ul", { class: "list" }, ...plan.violations.map((v) => h("li", {}, describeViolation(v)))));
    }
    for (const t of plan.transport) {
      const units = new Set(t.issues.map((i) => i.item)).size;
      const largest = t.issues.filter((i) => i.kind !== "stack_overload").reduce((m, i) => Math.max(m, i.required), 0);
      const shown = [...t.issues].sort((a, b) => b.required - a.required).slice(0, 12);
      const gaps = t.gaps ?? [];
      const gapTotal = gaps.reduce((s, g) => s + g.gap_mm, 0);
      panel.append(
        h("h3", {}, t.case, h("span", { class: "spacer" }), h("span", { class: `badge ${units ? "warn" : "ok"}` }, units ? `${units} units` : "OK")),
        units
          ? h(
              "div",
              {},
              h("p", { class: "hint" }, `Largest single securing force: ${fmt(largest, 2)} kN. Lash or block the listed units.`),
              h("ul", { class: "issue-list" }, ...shown.map((i) => h("li", {}, describeIssue(i))), t.issues.length > shown.length ? h("li", {}, `… ${t.issues.length - shown.length} more`) : null),
            )
          : h("p", { class: "hint" }, gaps.length ? "Nothing slides or tips once the gaps below are filled." : "Nothing slides or tips: friction and blocking hold everything."),
        gaps.length
          ? h(
              "details",
              {},
              h("summary", {}, `Fill ${gaps.length} gaps with dunnage (${fmt(gapTotal)} mm in total)`),
              h("ul", { class: "issue-list" }, ...gaps.map((g) => h("li", {}, `${g.item} ${DIR_LABEL[g.direction]} → ${g.other ?? "wall / load end"}: ${fmt(g.gap_mm)} mm${g.gap_mm >= 150 ? " (airbag)" : ""}`))),
            )
          : "",
      );
    }
  }

  if (selected) {
    const p = selected;
    const cap = capacityOf(p.item_id);
    const own = issuesByItem(plan).get(p.instance_id) ?? [];
    panel.append(
      h("h3", {}, `Selected: ${p.instance_id}`),
      stat("Load order", `#${p.seq + 1}`),
      stat("Shape", `${p.shape.kind.replace("_", "-")}, ${p.orientation}`),
      stat("Position", p.position.map((v) => fmt(v)).join(", ") + " mm"),
      stat("Size", p.size.map((v) => fmt(v)).join(" × ") + " mm"),
      stat("Mass", `${fmt(p.mass, 1)} kg`),
      stat("Load on top", `${fmt(p.load_on_top, 1)} kg${Number.isFinite(cap) ? ` of ${fmt(cap)} kg` : ""}`),
      stat("Stability margin", Number.isFinite(p.support_margin) ? `${fmt(p.support_margin, 1)} mm` : "held by chocks"),
      stat("Stop", p.stop === 0 ? "—" : String(p.stop)),
      stat("Floor pressure", p.floor_pressure ? `${fmt(p.floor_pressure)} kg/m²${req.container.floor_rating ? ` of ${fmt(req.container.floor_rating)}` : ""}` : "not on the floor"),
      p.needs_chocks ? stat("Chocks", "required") : "",
      p.securing ? stat("Securing", SECURING.find(([k]) => k === p.securing)?.[2] ?? p.securing) : "",
      p.impact ? stat("Transport force", `${fmt(p.impact.force_kn, 2)} kN, ${DIR_LABEL[p.impact.direction]} (${p.impact.case})`) : "",
      p.impact ? stat("Demand / own grip", `${fmt(p.impact.ratio, 2)}×${p.impact.ratio > 1 ? " — relies on blocking or lashing" : ""}`) : "",
      own.length ? h("ul", { class: "issue-list" }, ...own.map((i) => h("li", {}, describeIssue(i)))) : "",
      mode === "manual" ? selectedEditor(p) : "",
    );
  }

  if (r.unpacked.length) {
    const byReason = new Map<string, string[]>();
    for (const u of r.unpacked) byReason.set(u.reason, [...(byReason.get(u.reason) ?? []), u.instance_id]);
    panel.append(
      h("h3", {}, mode === "manual" ? "Not placed yet" : `Not packed (${r.unpacked.length})`),
      h("ul", { class: "list" }, ...[...byReason].map(([reason, ids]) => h("li", {}, `${reason.replace(/_/g, " ")}: ${ids.slice(0, 8).join(", ")}${ids.length > 8 ? ` +${ids.length - 8}` : ""}`))),
    );
  }

  panel.append(
    h(
      "div",
      { class: "actions" },
      h("button", { onclick: saveSolution, title: "Keep this plan in Solutions; mark it to train the learned placement" }, "Save solution…"),
      h("button", { onclick: exportPlanJson }, "Export plan (JSON)"),
      h("button", { onclick: exportLoadList }, "Export load list (CSV)"),
    ),
  );
}

function stat(label: string, value: string) {
  return h("div", { class: "stat" }, h("span", {}, label), h("span", {}, value));
}

// ---------- 3D view & timeline ----------

const meshKey = (p: Placement) => `${JSON.stringify(p.shape)}|${p.orientation}`;

async function ensureMeshes(plan: ContainerPlan) {
  const missing = new Map<string, Placement>();
  for (const p of plan.placements) if (!meshCache.has(meshKey(p))) missing.set(meshKey(p), p);
  await Promise.all(
    [...missing].map(async ([key, p]) => {
      meshCache.set(key, await invoke<RenderMesh>("shape_mesh", { shape: p.shape, orientation: p.orientation }));
    }),
  );
}

async function showPlan(keepStep = false) {
  const plan = currentPlan();
  $("empty").style.display = plan ? "none" : "grid";
  $("container-tabs").replaceChildren(
    ...(result?.containers ?? []).map((_, i) => h("button", { class: i === current ? "active" : "", onclick: () => selectContainer(i) }, `#${i + 1}`)),
  );
  if (!plan) {
    viewer.clear();
    renderLegend();
    updateStep();
    return;
  }
  const step = $<HTMLInputElement>("step");
  const prev = Number(step.value);
  await ensureMeshes(plan);
  viewer.show(plan, colorFor, (p) => ({ key: meshKey(p), data: meshCache.get(meshKey(p))! }), plan.placements.length ? plan.metrics.center_of_mass : null, guidesFor(plan));
  step.max = String(plan.placements.length);
  step.value = String(keepStep ? Math.min(prev, plan.placements.length) : plan.placements.length);
  applyFocus();
  updateStep();
  renderLegend();
}

/** CTU Code window for the cargo's centre of gravity, drawn on the floor. */
function guidesFor(plan: ContainerPlan): BalanceGuides | null {
  const o = req.options.balance;
  if (!o.ctu_checks || !plan.placements.length) return null;
  const [W, , D] = plan.size;
  const [ex, ez] = [o.max_eccentricity * W, o.max_eccentricity * D];
  const [cx, , cz] = plan.metrics.center_of_mass;
  return {
    window: [W / 2 - ex, D / 2 - ez, W / 2 + ex, D / 2 + ez],
    inside: Math.abs(cx - W / 2) <= ex + 1e-6 && Math.abs(cz - D / 2) <= ez + 1e-6,
    quarters: [0.25 * D, 0.75 * D],
  };
}

function selectContainer(i: number) {
  current = i;
  selected = null;
  focusKeys.clear();
  showPlan();
  renderResults();
}

function setStep(n: number) {
  const step = $<HTMLInputElement>("step");
  step.value = String(Math.max(0, Math.min(Number(step.max), n)));
  updateStep();
}

function updateStep() {
  const plan = currentPlan();
  const n = Number($<HTMLInputElement>("step").value);
  viewer.setVisibleCount(n);
  const total = plan?.placements.length ?? 0;
  const bySeq = (s: number) => plan?.placements.find((p) => p.seq === s);
  const last = bySeq(n - 1);
  const next = bySeq(n);
  $("step-label").textContent = plan ? `Step ${n} / ${total}${last ? ` · placed ${last.instance_id}` : ""}` : "";
  $("next-label").textContent = next
    ? `Next: ${next.instance_id} (${next.shape.kind.replace("_", "-")}, ${fmt(next.mass, 1)} kg) → x ${fmt(next.position[0])}, y ${fmt(next.position[1])}, z ${fmt(next.position[2])}${next.needs_chocks ? " · chock it" : ""}`
    : plan
      ? "All items loaded"
      : "";
  for (const id of ["first", "prev"]) $<HTMLButtonElement>(id).disabled = !plan || n <= 0;
  for (const id of ["next", "last"]) $<HTMLButtonElement>(id).disabled = !plan || n >= total;
}

function stopPlaying() {
  if (playing !== null) clearInterval(playing);
  playing = null;
  $("play").textContent = "▶";
}

function togglePlay() {
  const step = $<HTMLInputElement>("step");
  if (playing !== null) return stopPlaying();
  if (!currentPlan()) return;
  if (step.value === step.max) setStep(0);
  $("play").textContent = "⏸";
  playing = window.setInterval(() => {
    setStep(Number(step.value) + 1);
    if (Number(step.value) >= Number(step.max)) stopPlaying();
  }, 350);
}

// ---------- actions ----------

function setStatus(msg: string, kind: "" | "ok" | "bad" = "") {
  const s = $("status");
  s.textContent = msg;
  s.className = `status ${kind}`;
}

function objective(): Objective {
  // 0 = density only, 1 = strongly avoid lashing and dunnage.
  const t = search.securing;
  return { density: 1, securing: 2 * t, dunnage: 0.6 * t, stability: 1, balance: 0.1 + 0.6 * t };
}

async function runSearch() {
  const buttons = [$<HTMLButtonElement>("pack"), $<HTMLButtonElement>("pack-mobile")];
  const labels = buttons.map((b) => b.textContent);
  searching = true;
  for (const b of buttons) b.textContent = "Stop ■";
  stopPlaying();
  const t0 = performance.now();
  const phaseName: Record<SearchPhase, string> = { sweep: "trying every pattern", evolve: "evolving orders", polish: "polishing" };
  setStatus(`Searching (${search.budget} s)…`);
  const unlisten = await listen<SearchProgress>("optimize-progress", (e) => {
    const p = e.payload;
    const left = Math.max(0, search.budget - (performance.now() - t0) / 1000);
    setStatus(`${phaseName[p.phase]}: ${p.evaluated} plans, best ${fmt(p.best.volume_utilization * 100, 1)} % vol, ${p.best.tipping_units} may tip, ${p.best.lashing_units} to lash, ${p.best.balance_issues} balance warn. · ${left.toFixed(0)} s left`);
  });
  try {
    const options = { budget_ms: search.budget * 1000, objective: objective(), keep: 3 };
    const r = await invoke<OptimizeResult>("optimize_request", { request: withModel(), options });
    searchResult = r;
    searchPick = 0;
    result = r.solutions[0].result;
    stale = false;
    current = 0;
    selected = null;
    focusKeys.clear();
    const best = r.solutions[0].score;
    setStatus(
      `${r.cancelled ? "Stopped" : "Done"}: ${r.evaluated} plans in ${fmt(r.elapsed_ms / 1000, 1)} s · best ${fmt(best.volume_utilization * 100, 1)} % vol, ${best.tipping_units} may tip, ${best.lashing_units} to lash, ${best.balance_issues} balance warn.`,
      result.unpacked.length === 0 ? "ok" : "bad",
    );
    showTab("view");
    await showPlan();
    renderResults();
  } catch (e) {
    setStatus(String(e), "bad");
  } finally {
    unlisten();
    searching = false;
    buttons.forEach((b, i) => (b.textContent = labels[i]));
  }
}

async function runPack() {
  if (searching) {
    await invoke("cancel_optimize");
    return;
  }
  if (!req.items.length) {
    setStatus("Add some items first.", "bad");
    return;
  }
  if (mode === "manual") return manualAutoFill();
  if (search.enabled) return runSearch();
  searchResult = null;
  const btn = $<HTMLButtonElement>("pack");
  const btnMobile = $<HTMLButtonElement>("pack-mobile");
  btn.disabled = btnMobile.disabled = true;
  stopPlaying();
  setStatus("Packing…");
  try {
    result = await invoke<PackResult>("pack_request", { request: withModel() });
    stale = false;
    current = 0;
    selected = null;
    focusKeys.clear();
    const valid = result.containers.every((c) => c.violations.length === 0);
    setStatus(
      `${result.packed_units}/${result.requested_units} units in ${result.containers.length} container(s), ${result.elapsed_ms} ms`,
      valid && result.unpacked.length === 0 ? "ok" : "bad",
    );
    showTab("view");
    await showPlan();
    renderResults();
  } catch (e) {
    setStatus(String(e), "bad");
  } finally {
    btn.disabled = btnMobile.disabled = false;
  }
}

function loadRequest(r: PackRequest) {
  // Fill in anything older files may lack.
  const base = emptyRequest();
  req = {
    container: { ...base.container, ...r.container, cog_limits: { ...base.container.cog_limits, ...r.container.cog_limits } },
    items: r.items.map((i) => ({ ...newItem(0), ...i })),
    options: {
      ...defaultOptions(),
      ...r.options,
      physics: { ...defaultPhysics(), ...r.options?.physics, lashing: { ...defaultLashing(), ...r.options?.physics?.lashing } },
      balance: { ...defaultBalance(), ...r.options?.balance },
    },
  };
  result = null;
  autoResult = null;
  searchResult = autoSearch = null;
  stale = autoStale = false;
  selected = null;
  focusKeys.clear();
  stopPlaying();
  Object.assign(manual, { started: false, placements: [], history: [], remaining: [], item: null, poses: [], pose: 0, source: "manual" });
  renderEditor();
  renderResults();
  showPlan();
  if (mode === "manual") startManual();
}

async function loadSample(kind: string) {
  try {
    loadRequest(await invoke<PackRequest>("sample_request", { kind, seed: 1 }));
    setStatus(`Loaded sample "${kind}". Press Pack.`);
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

async function saveTextFile(defaultPath: string, ext: string, contents: string) {
  const path = await save({ defaultPath, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!path) return;
  try {
    await writeTextFile(path, contents);
    setStatus(`Saved ${path}`, "ok");
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

async function openRequest() {
  const path = await open({ multiple: false, filters: [{ name: "OmniPack setup", extensions: ["json"] }] });
  if (!path || Array.isArray(path)) return;
  try {
    const text = await readTextFile(path);
    const parsed = JSON.parse(text) as PackRequest;
    if (!parsed.container || !Array.isArray(parsed.items)) throw new Error("not an OmniPack setup file");
    loadRequest(parsed);
    setStatus(`Opened ${path}`);
  } catch (e) {
    await message(String(e), { title: "Cannot open file", kind: "error" });
  }
}

function exportPlanJson() {
  if (result) saveTextFile(`${req.container.id}-plan.json`, "json", JSON.stringify(result, null, 2));
}

function exportLoadList() {
  if (!result) return;
  const rows = [["container", "seq", "unit", "item", "shape", "x_mm", "y_mm", "z_mm", "w_mm", "h_mm", "d_mm", "orientation", "mass_kg", "load_on_top_kg", "floor_kg_m2", "stop", "chocks", "lashings", "securing"]];
  for (const c of result.containers) {
    const issues = issuesByItem(c);
    for (const p of c.placements)
      rows.push([
        c.id, String(p.seq + 1), p.instance_id, p.item_id, p.shape.kind,
        ...p.position.map((v) => v.toFixed(1)), ...p.size.map((v) => v.toFixed(1)),
        p.orientation, p.mass.toFixed(2), p.load_on_top.toFixed(2), (p.floor_pressure ?? 0).toFixed(0), String(p.stop),
        p.needs_chocks ? "yes" : "", String(Math.max(0, ...(issues.get(p.instance_id) ?? []).map((i) => i.lashings ?? 0))),
        (issues.get(p.instance_id) ?? []).map(describeIssue).join("; "),
      ]);
  }
  const csv = rows.map((r) => r.map((v) => (/[",;\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v)).join(",")).join("\n");
  saveTextFile(`${req.container.id}-load-list.csv`, "csv", csv);
}

async function refreshCatalog() {
  const sel = $<HTMLSelectElement>("catalog");
  try {
    const names = await invoke<string[]>("catalog_list");
    sel.replaceChildren(h("option", { value: "" }, "Catalog…"), ...names.map((n) => h("option", { value: n }, n)));
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

/** In-page text prompt (window.prompt is unavailable in some mobile WebViews). */
function promptText(label: string, value: string): Promise<string | null> {
  const modal = $("modal");
  const input = $<HTMLInputElement>("modal-input");
  $("modal-label").textContent = label;
  input.value = value;
  modal.hidden = false;
  input.focus();
  input.select();
  return new Promise((resolve) => {
    const done = (v: string | null) => {
      modal.hidden = true;
      $("modal-form").removeEventListener("submit", onSubmit);
      $("modal-cancel").removeEventListener("click", onCancel);
      resolve(v);
    };
    const onSubmit = (e: Event) => {
      e.preventDefault();
      done(input.value.trim() || null);
    };
    const onCancel = () => done(null);
    $("modal-form").addEventListener("submit", onSubmit);
    $("modal-cancel").addEventListener("click", onCancel);
  });
}

function showTab(tab: string) {
  document.body.dataset.tab = tab;
  for (const b of document.querySelectorAll<HTMLButtonElement>("#mobile-tabs button[data-tab]")) b.classList.toggle("active", b.dataset.tab === tab);
}

// ---------- manual placement ----------

/** The hand-made plan as a pack result, so the results panel and viewer show it. */
function manualResult(view: ManualView): PackResult {
  const plan = view.plan;
  return {
    schema: "omnipack.plan/1",
    containers: [plan],
    unpacked: view.remaining.filter(([, n]) => n > 0).map(([id, n]) => ({ instance_id: `${id} ×${n}`, item_id: id, reason: "to_place" })),
    requested_units: req.items.reduce((s, i) => s + i.quantity, 0),
    packed_units: plan.placements.length,
    volume_utilization: plan.metrics.volume_utilization,
    elapsed_ms: 0,
  };
}

async function applyManualView(view: ManualView) {
  manual.placements = view.plan.placements;
  manual.remaining = view.remaining;
  result = manualResult(view);
  stale = false;
  current = 0;
  if (selected) selected = manual.placements.find((p) => p.instance_id === selected!.instance_id) ?? null;
  viewer.highlight(selected?.instance_id ?? null);
  await showPlan();
  renderResults();
  if (ui.tab === "place") renderEditor();
  else renderManualBar();
}

/** Runs a manual-session command and shows the re-checked plan. */
async function manualCall(cmd: string, args: Record<string, unknown>, record = true) {
  try {
    const before = manual.placements;
    const view = await invoke<ManualView>(cmd, args);
    if (record) {
      manual.history.push(before);
      if (manual.history.length > 100) manual.history.shift();
    }
    await applyManualView(view);
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

async function startManual() {
  manual.started = true;
  await manualCall("manual_set", { request: withModel(), placements: manual.placements }, false);
}

async function setMode(m: Mode) {
  if (m === mode) return;
  stopPlaying();
  mode = m;
  document.body.dataset.mode = m;
  for (const b of document.querySelectorAll<HTMLButtonElement>("#mode-switch button")) b.classList.toggle("active", b.dataset.mode === m);
  for (const id of ["pack", "pack-mobile"]) $(id).textContent = m === "manual" ? "Auto-fill ▶" : "Pack ▶";
  selected = null;
  focusKeys.clear();
  if (m === "manual") {
    autoResult = result;
    autoSearch = searchResult;
    searchResult = null;
    ui.tab = "place";
    viewer.setInteraction(manualInteraction);
    renderEditor();
    await startManual();
    setStatus("Manual mode: pick a unit in the Place tab, then click in the 3D view.");
  } else {
    viewer.setInteraction(null);
    result = autoResult;
    searchResult = autoSearch;
    stale = stale || autoStale;
    autoStale = false;
    current = 0;
    if (ui.tab === "place") ui.tab = "cargo";
    renderEditor();
    await showPlan();
    renderResults();
    setStatus("");
  }
}

/** Holds a unit of `id` ready to place (or lets go of it). */
async function armItem(id: string | null) {
  manual.item = manual.item === id ? null : id;
  manual.poses = [];
  manual.pose = 0;
  viewer.hideGhost();
  const it = req.items.find((i) => i.id === manual.item);
  if (it) {
    try {
      manual.poses = await invoke<Pose[]>("item_poses", { item: it, allowRotation: req.options.allow_rotation });
    } catch (e) {
      setStatus(String(e), "bad");
    }
    if (!manual.poses.length) setStatus(`${it.id} has no allowed orientation.`, "bad");
  }
  renderEditor();
}

function setManualHint(text: string) {
  const el = document.getElementById("manual-hint");
  if (el) el.textContent = text;
}

/** Touch screen without a mouse: no hover, no keyboard shortcuts. */
const TOUCH = window.matchMedia("(pointer: coarse)").matches;

/** The units-to-place bar over the 3D view (handy on phones). */
function renderManualBar() {
  const bar = document.getElementById("manual-bar");
  if (!bar) return;
  if (mode !== "manual") return void bar.replaceChildren();
  const left = new Map(manual.remaining);
  const sel = h("select", { title: "Unit to place" }, h("option", { value: "" }, "Select / move"), ...req.items.map((it) => h("option", { value: it.id, disabled: (left.get(it.id) ?? 0) <= 0 }, `${it.id} (${left.get(it.id) ?? 0})`))) as HTMLSelectElement;
  sel.value = manual.item ?? "";
  sel.addEventListener("change", () => armItem(sel.value || null));
  bar.replaceChildren(
    sel,
    h("button", { title: "Rotate (R)", onclick: rotateManual }, "⟳"),
    h("button", { title: "Undo (Ctrl+Z)", onclick: undoManual, disabled: !manual.history.length }, "↶"),
    h("button", { title: "Remove the selected unit (Delete)", onclick: removeSelected, disabled: !selected }, "✕"),
    h("span", { id: "manual-hint", class: "hint" }, manual.item ? `${TOUCH ? "Tap" : "Click"} in the container to place it` : selected ? `${selected.instance_id}: drag it${TOUCH ? "" : ", or use the arrow keys"}` : TOUCH ? "Drag a unit to move it" : ""),
  );
}

/** Lines a unit up with the walls and nearby faces (within 30 mm) and keeps it inside. */
function snapped(x: number, z: number, size: [number, number, number], skip: string | null): [number, number] {
  const { width: W, depth: D } = req.container;
  let sx = Math.min(Math.max(0, x), Math.max(0, W - size[0]));
  let sz = Math.min(Math.max(0, z), Math.max(0, D - size[2]));
  if (manual.magnet) {
    const near = (v: number, cands: number[]) => cands.reduce((best, c) => (Math.abs(v - c) < Math.abs(v - best) && Math.abs(v - c) < 30 ? c : best), v);
    const xs = [0, W - size[0]];
    const zs = [0, D - size[2]];
    for (const p of manual.placements) {
      if (p.instance_id === skip) continue;
      xs.push(p.position[0] - size[0], p.position[0] + p.size[0], p.position[0]);
      zs.push(p.position[2] - size[2], p.position[2] + p.size[2], p.position[2]);
    }
    sx = near(sx, xs);
    sz = near(sz, zs);
  }
  return [Math.round(sx * 10) / 10, Math.round(sz * 10) / 10];
}

/** Probes run one at a time; while one runs, only the latest request waits. */
let probeBusy = false;
let probeNext: (() => Promise<void>) | null = null;
function queueProbe(job: () => Promise<void>) {
  if (probeBusy) {
    probeNext = job;
    return;
  }
  probeBusy = true;
  job().finally(() => {
    probeBusy = false;
    const next = probeNext;
    probeNext = null;
    if (next) queueProbe(next);
  });
}

/** Shows where a unit would land under the pointer, green or red. */
function ghostAt(itemId: string, pose: Pose, hit: Hit, replace: string | null) {
  const [x, z] = snapped(hit.point[0] - pose.extents[0] / 2, hit.point[2] - pose.extents[2] / 2, pose.extents, replace);
  const y = manual.gravity ? null : hit.point[1];
  queueProbe(async () => {
    try {
      const pr = await invoke<Probe>("manual_probe", { itemId, orientation: pose.orientation, x, z, y, replace });
      if (!manual.item && !replace) return;
      const p = pr.placement;
      const key = meshKey(p);
      if (!meshCache.has(key)) meshCache.set(key, await invoke<RenderMesh>("shape_mesh", { shape: p.shape, orientation: p.orientation }));
      viewer.showGhost(key, meshCache.get(key)!, p.position, p.size, pr.problems.length === 0);
      setManualHint(pr.problems.length ? `⚠ ${describeViolation(pr.problems[0])}` : `✓ x ${fmt(p.position[0])}, y ${fmt(p.position[1])}, z ${fmt(p.position[2])} mm`);
    } catch (e) {
      setManualHint(String(e));
    }
  });
}

let dragUnit: Placement | null = null;
/** The last point the dragged unit was over, used when it is let go off the floor. */
let dragHit: Hit | null = null;

const manualInteraction: Interaction = {
  hover(hit) {
    const pose = manual.poses[manual.pose];
    if (!manual.item || !pose || !hit) return void (manual.item ? undefined : viewer.hideGhost());
    ghostAt(manual.item, pose, hit, null);
  },
  tap(hit) {
    const pose = manual.poses[manual.pose];
    if (!manual.item || !pose) return false;
    if (!hit) return true;
    const item = manual.item;
    const [x, z] = snapped(hit.point[0] - pose.extents[0] / 2, hit.point[2] - pose.extents[2] / 2, pose.extents, null);
    manualCall("manual_place", { itemId: item, orientation: pose.orientation, x, z, y: manual.gravity ? null : hit.point[1], replace: null }).then(() => {
      // Let go of the item once all its units are in.
      if ((new Map(manual.remaining).get(item) ?? 0) <= 0 && manual.item === item) armItem(null);
    });
    return true;
  },
  dragStart(p) {
    if (manual.item) return false;
    dragUnit = p;
    dragHit = null;
    selected = p;
    viewer.highlight(p.instance_id);
    return true;
  },
  drag(hit) {
    if (hit) dragHit = hit;
    if (dragUnit && hit) ghostAt(dragUnit.item_id, { orientation: dragUnit.orientation, extents: dragUnit.size }, hit, dragUnit.instance_id);
  },
  drop(hit) {
    const p = dragUnit;
    hit ??= dragHit;
    dragUnit = null;
    dragHit = null;
    viewer.hideGhost();
    if (!p || !hit) return;
    const [x, z] = snapped(hit.point[0] - p.size[0] / 2, hit.point[2] - p.size[2] / 2, p.size, p.instance_id);
    manualCall("manual_place", { itemId: p.item_id, orientation: p.orientation, x, z, y: manual.gravity ? null : hit.point[1], replace: p.instance_id });
  },
};

/** Moves or turns the selected unit; a typed Y places it exactly there. */
function moveSelected(ch: { x?: number; y?: number | null; z?: number; orientation?: Orientation }) {
  const p = selected;
  if (!p) return;
  const y = ch.y !== undefined ? ch.y : manual.gravity ? null : p.position[1];
  manualCall("manual_place", { itemId: p.item_id, orientation: ch.orientation ?? p.orientation, x: ch.x ?? p.position[0], z: ch.z ?? p.position[2], y, replace: p.instance_id });
}

function selectedEditor(p: Placement): HTMLElement {
  return h(
    "div",
    { class: "selected-edit" },
    h(
      "div",
      { class: "grid" },
      num("X mm", () => Math.round(p.position[0] * 10) / 10, (v) => moveSelected({ x: v ?? 0 }), { quiet: true }),
      num("Y mm", () => Math.round(p.position[1] * 10) / 10, (v) => moveSelected({ y: v ?? 0 }), { quiet: true }),
      num("Z mm", () => Math.round(p.position[2] * 10) / 10, (v) => moveSelected({ z: v ?? 0 }), { quiet: true }),
    ),
    h("div", { class: "actions" }, h("button", { onclick: rotateManual }, "⟳ Rotate"), h("button", { onclick: removeSelected }, "Remove")),
  );
}

async function rotateManual() {
  if (manual.item && manual.poses.length) {
    manual.pose = (manual.pose + 1) % manual.poses.length;
    renderEditor();
    return;
  }
  const p = selected;
  const it = p && req.items.find((i) => i.id === p.item_id);
  if (!p || !it) return;
  const poses = await invoke<Pose[]>("item_poses", { item: it, allowRotation: true });
  if (poses.length < 2) return;
  const k = poses.findIndex((q) => q.orientation === p.orientation);
  moveSelected({ orientation: poses[(k + 1) % poses.length].orientation });
}

function removeSelected() {
  const p = selected;
  if (!p) return;
  selected = null;
  manualCall("manual_remove", { instanceId: p.instance_id });
}

function undoManual() {
  const prev = manual.history.pop();
  if (prev) manualCall("manual_set", { request: withModel(), placements: prev }, false);
}

function clearManual() {
  if (manual.placements.length) manualCall("manual_set", { request: withModel(), placements: [] });
}

async function manualAutoFill() {
  setStatus("Auto-filling around your units…");
  try {
    const r = await invoke<PackResult>("manual_auto_fill");
    const view = await invoke<ManualView>("manual_set", { request: withModel(), placements: r.containers[0]?.placements ?? [] });
    manual.history.push(manual.placements);
    manual.source = "manual+auto";
    await applyManualView(view);
    if (r.containers.length > 1) {
      result = { ...r, containers: [view.plan, ...r.containers.slice(1)] };
      renderResults();
      await showPlan();
    }
    setStatus(
      `Auto-filled: ${r.packed_units}/${r.requested_units} units${r.containers.length > 1 ? `, ${r.containers.length - 1} more container(s)` : ""}`,
      r.unpacked.length ? "bad" : "ok",
    );
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

/** Keys in manual mode; returns true if the key was used. */
function manualKey(e: KeyboardEvent): boolean {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") return (undoManual(), true);
  if (e.key === "Escape" && manual.item) return (armItem(null), true);
  if (e.key === "r" || e.key === "R") return (rotateManual(), true);
  const p = selected;
  if (!p) return false;
  if (e.key === "Delete" || e.key === "Backspace") return (removeSelected(), true);
  const step = e.shiftKey ? 100 : 10;
  const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
  const m = moves[e.key];
  if (!m) return false;
  moveSelected({ x: p.position[0] + m[0], z: p.position[2] + m[1] });
  return true;
}

// ---------- saved solutions and learning ----------

/** Name and "use for training" for a plan to save. */
function promptSave(name: string, valid: boolean): Promise<{ name: string; train: boolean } | null> {
  const row = $("modal-check-row");
  const cb = $<HTMLInputElement>("modal-check");
  row.hidden = false;
  cb.checked = valid;
  cb.disabled = !valid;
  $("modal-check-hint").textContent = valid ? "Teaches the learned placement what a good plan looks like." : "This plan has violations, so it cannot be used for training.";
  return promptText("Save this plan as:", name).then((n) => {
    row.hidden = true;
    return n ? { name: n, train: cb.checked && valid } : null;
  });
}

async function saveSolution() {
  if (!result) return setStatus("Pack or place something first.", "bad");
  const valid = result.containers.every((c) => c.violations.length === 0);
  const source = mode === "manual" ? manual.source : searchResult ? "best" : "auto";
  const answer = await promptSave(`${req.container.id} – ${new Date().toLocaleString()}`, valid);
  if (!answer) return;
  try {
    await invoke("solution_save", { name: answer.name, source, train: answer.train, request: req, result });
    setStatus(`Saved "${answer.name}"${answer.train ? " for training" : ""}. Open it from Solutions.`, "ok");
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

async function openSolution(id: string) {
  const saved = await invoke<SavedPlan>("solution_load", { id });
  loadRequest(saved.request);
  if (saved.source.startsWith("manual")) {
    manual.placements = saved.result.containers[0]?.placements ?? [];
    manual.source = saved.source;
    if (mode === "manual") await startManual();
    else await setMode("manual");
  } else {
    if (mode === "manual") await setMode("auto");
    result = saved.result;
    current = 0;
    await showPlan();
    renderResults();
  }
  setStatus(`Opened "${saved.name}".`);
}

/** The Solutions dialog: saved plans, training flags and the learned model. */
async function openSolutions() {
  const box = h("div", { class: "modal-box wide" });
  const overlay = h("div", { class: "modal" }, box);
  const close = () => overlay.remove();
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) close();
  });
  document.body.append(overlay);
  const render = async () => {
    let list: SolutionMeta[] = [];
    try {
      list = await invoke<SolutionMeta[]>("solution_list");
      model = await invoke<ModelFile | null>("model_info");
    } catch (e) {
      setStatus(String(e), "bad");
    }
    const flagged = list.filter((m) => m.train && m.valid).length;
    const row = (m: SolutionMeta) => {
      const cb = h("input", { type: "checkbox", disabled: !m.valid, title: m.valid ? "Use for training" : "Has violations: cannot be used for training" }) as HTMLInputElement;
      cb.checked = m.train;
      cb.addEventListener("change", async () => {
        try {
          await invoke("solution_set_train", { id: m.id, train: cb.checked });
        } catch (e) {
          setStatus(String(e), "bad");
        }
        render();
      });
      return h(
        "div",
        { class: "sol-row" },
        h("div", { class: "sol-main" }, h("b", {}, m.name), h("span", {}, `${new Date(m.created * 1000).toLocaleString()} · ${m.source} · ${m.summary}`)),
        h("span", { class: `badge ${m.valid ? "ok" : "bad"}`, title: m.valid ? "Valid plan" : "Has violations" }, m.valid ? "✓" : "✗"),
        h("label", { class: "train", title: "Use this plan to train the learned placement" }, cb, "train"),
        h("button", { class: "small", onclick: async () => {
          close();
          try {
            await openSolution(m.id);
          } catch (e) {
            setStatus(String(e), "bad");
          }
        } }, "Open"),
        h("button", { class: "small", title: "Delete", onclick: async () => {
          if (!(await ask(`Delete "${m.name}"?`, { title: "Delete solution", kind: "warning" }))) return;
          await invoke("solution_delete", { id: m.id });
          render();
        } }, "✕"),
      );
    };
    const r = model?.report;
    box.replaceChildren(
      h("h3", {}, "Saved solutions", h("span", { class: "spacer" }), h("button", { class: "small", onclick: close }, "✕")),
      list.length ? h("div", { class: "sol-list" }, ...[...list].reverse().map(row)) : h("p", { class: "hint" }, "Nothing saved yet. Use “Save solution…” under the results, in auto or manual mode."),
      h("h3", {}, "Learned placement"),
      h(
        "p",
        { class: "hint" },
        model && r
          ? `Trained ${new Date(model.trained * 1000).toLocaleString()} on ${r.steps} placement decisions from ${r.plans} plan(s). The position you chose ranks first in ${fmt(r.top1_before * 100)} % of them with the closest built-in pattern, ${fmt(r.top1_after * 100)} % with the learned weights. Use it with the fill pattern “Learned”; ★ Best tries it too.`
          : "No model yet. Tick “train” on valid plans you like (yours or automatic ones), then train.",
      ),
      h("p", { class: "hint" }, `${flagged} plan(s) marked for training.`),
      h(
        "div",
        { class: "actions" },
        h("button", { class: "primary", disabled: !flagged, onclick: async () => {
          setStatus("Training…");
          try {
            model = await invoke<ModelFile>("model_train");
            setStatus(`Trained on ${model.report.steps} decisions: ${fmt(model.report.top1_before * 100)} % → ${fmt(model.report.top1_after * 100)} % ranked first.`, "ok");
            renderEditor();
          } catch (e) {
            setStatus(String(e), "bad");
          }
          render();
        } }, "Train model"),
        h("button", { disabled: !flagged, onclick: async () => {
          try {
            await saveTextFile("omnipack-training.jsonl", "jsonl", await invoke<string>("training_data"));
          } catch (e) {
            setStatus(String(e), "bad");
          }
        } }, "Export training data"),
        model
          ? h("button", { onclick: async () => {
              if (!(await ask("Forget the learned model?", { title: "Reset model", kind: "warning" }))) return;
              await invoke("model_reset");
              model = null;
              if (req.options.bias === "learned") req.options.bias = "wall_building";
              renderEditor();
              render();
            } }, "Reset model")
          : null,
      ),
    );
  };
  await render();
}

// ---------- local API (desktop) ----------

function randomKey(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The API dialog: the desktop app serves the OmniPack API to other programs. */
async function openApiDialog() {
  let status: ApiStatus;
  try {
    status = await invoke<ApiStatus>("api_status");
  } catch (e) {
    return setStatus(String(e), "bad");
  }
  const s: LocalApiSettings = { ...status.settings };
  const box = h("div", { class: "modal-box wide" });
  const overlay = h("div", { class: "modal" }, box);
  const close = () => overlay.remove();
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) close();
  });
  document.body.append(overlay);
  const render = () => {
    const keyInput = h("input", { type: "text", value: s.api_key, placeholder: "none (this computer only)", spellcheck: "false" }) as HTMLInputElement;
    keyInput.addEventListener("change", () => (s.api_key = keyInput.value.trim()));
    const folder = (label: string, get: () => string | null, set: (v: string | null) => void) => {
      const input = h("input", { type: "text", value: get() ?? "", placeholder: "—" }) as HTMLInputElement;
      input.addEventListener("change", () => set(input.value.trim() || null));
      const pick = h("button", { type: "button", class: "small", onclick: async () => {
        const dir = await open({ directory: true, multiple: false });
        if (typeof dir === "string") {
          set(dir);
          render();
        }
      } }, "Browse…");
      return h("label", { class: "field" }, h("span", {}, label), h("div", { class: "row-input" }, input, pick));
    };
    const url = status.url ?? `http://127.0.0.1:${s.port}`;
    const keyHeader = s.api_key ? ` -H "X-API-Key: ${s.api_key}"` : "";
    const toggle = (label: string, get: () => boolean, set: (v: boolean) => void, title?: string) => {
      const cb = h("input", { type: "checkbox" }) as HTMLInputElement;
      cb.checked = get();
      cb.addEventListener("change", () => set(cb.checked));
      return h("label", { title }, cb, label);
    };
    box.replaceChildren(
      h("h3", {}, "Local API", h("span", { class: "spacer" }), h("button", { class: "small", onclick: close }, "✕")),
      h("p", { class: "hint" }, "While OmniPack runs, other programs (an ERP system such as SAP, scripts, other apps) can send a container and its cargo and get every placement back, as JSON or a CSV load list. It is the same API as the standalone omnipack-server. See docs/api.md; the full description is at /api/v1/openapi.json."),
      h(
        "div",
        { class: "checks" },
        toggle("Enable the API", () => s.enabled, (v) => (s.enabled = v)),
        toggle("Allow other computers", () => s.allow_network, (v) => {
          s.allow_network = v;
          if (v && !s.api_key) s.api_key = randomKey();
          render();
        }, "Listen on the network, not only on this computer. Needs an API key; check your firewall."),
      ),
      h(
        "div",
        { class: "grid two" },
        num("Port", () => s.port, (v) => (s.port = Math.min(65535, Math.max(1024, Math.round(v ?? 8765)))), { min: 1024, max: 65535, step: "1", quiet: true }),
        h("label", { class: "field" }, h("span", {}, "API key"), h("div", { class: "row-input" }, keyInput,
          h("button", { type: "button", class: "small", title: "New random key", onclick: () => {
            s.api_key = randomKey();
            render();
          } }, "New"),
          h("button", { type: "button", class: "small", title: "Copy the key", onclick: async () => {
            try {
              await navigator.clipboard.writeText(s.api_key);
              setStatus("API key copied.", "ok");
            } catch {
              keyInput.select();
            }
          } }, "Copy"),
        )),
        folder("Drop folder: inbox", () => s.inbox, (v) => (s.inbox = v)),
        folder("Drop folder: outbox", () => s.outbox, (v) => (s.outbox = v)),
      ),
      h("p", { class: "hint" }, "Drop folders: JSON requests or CSV item lists put in the inbox are planned (CSV lists in a 40 ft high cube); the result and a load list appear in the outbox, for file-based interfaces."),
      h(
        "p",
        { class: `hint api-state ${status.running ? "ok" : status.error ? "bad" : ""}` },
        status.running ? `● Running at ${status.url}` : status.error ? `✗ ${status.error}` : "○ Stopped",
      ),
      status.running ? h("pre", { class: "api-example" }, `curl -X POST ${url}/api/v1/erp/plan${keyHeader} -H "Content-Type: application/json" -d @delivery.json`) : "",
      h(
        "div",
        { class: "actions" },
        h("button", { class: "primary", onclick: async () => {
          s.api_key = keyInput.value.trim();
          try {
            status = await invoke<ApiStatus>("api_apply", { settings: s });
            setStatus(status.running ? `Local API running at ${status.url}` : status.error ?? "Local API stopped.", status.error ? "bad" : "ok");
          } catch (e) {
            setStatus(String(e), "bad");
          }
          render();
        } }, "Apply"),
        h("button", { onclick: close }, "Close"),
      ),
    );
  };
  render();
}

// ---------- panel width ----------

function applyEditorWidth() {
  document.documentElement.style.setProperty("--editor-w", `${ui.width}px`);
}

$("editor-resize").addEventListener("pointerdown", (e) => {
  const el = e.currentTarget as HTMLElement;
  el.setPointerCapture(e.pointerId);
  el.classList.add("resizing");
  const left = $("editor").getBoundingClientRect().left;
  const move = (ev: PointerEvent) => {
    ui.width = Math.round(Math.min(Math.max(300, ev.clientX - left), window.innerWidth * 0.5));
    applyEditorWidth();
  };
  const up = () => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", up);
    el.classList.remove("resizing");
    saveUi();
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", up);
});

// ---------- wiring ----------

for (const b of document.querySelectorAll<HTMLButtonElement>("#mode-switch button")) b.addEventListener("click", () => setMode(b.dataset.mode as Mode));
$("solutions").addEventListener("click", openSolutions);
$("api").addEventListener("click", openApiDialog);

for (const b of document.querySelectorAll<HTMLButtonElement>("#mobile-tabs button[data-tab]")) b.addEventListener("click", () => showTab(b.dataset.tab!));
$("pack-mobile").addEventListener("click", runPack);

$<HTMLSelectElement>("sample").addEventListener("change", (e) => {
  const sel = e.target as HTMLSelectElement;
  if (sel.value) loadSample(sel.value);
  sel.value = "";
});
$("new").addEventListener("click", async () => {
  if (req.items.length && !(await ask("Discard the current setup?", { title: "New setup" }))) return;
  loadRequest(emptyRequest());
  setStatus("");
});
$("open").addEventListener("click", openRequest);
$("save").addEventListener("click", () => saveTextFile(`${req.container.id}.json`, "json", JSON.stringify(req, null, 2)));
$("pack").addEventListener("click", runPack);

$<HTMLSelectElement>("catalog").addEventListener("change", async (e) => {
  const name = (e.target as HTMLSelectElement).value;
  if (!name) return;
  try {
    loadRequest(await invoke<PackRequest>("catalog_load", { name }));
    setStatus(`Loaded "${name}" from the catalog.`);
  } catch (err) {
    setStatus(String(err), "bad");
  }
});
$("cat-save").addEventListener("click", async () => {
  const name = await promptText("Save this setup to the catalog as:", req.container.id);
  if (!name) return;
  try {
    await invoke("catalog_save", { name, request: req });
    await refreshCatalog();
    $<HTMLSelectElement>("catalog").value = name;
    setStatus(`Saved "${name}" to the catalog.`, "ok");
  } catch (e) {
    setStatus(String(e), "bad");
  }
});
$("cat-delete").addEventListener("click", async () => {
  const name = $<HTMLSelectElement>("catalog").value;
  if (!name) return setStatus("Pick a catalog entry to delete.");
  if (!(await ask(`Delete "${name}" from the catalog?`, { title: "Delete", kind: "warning" }))) return;
  await invoke("catalog_delete", { name });
  await refreshCatalog();
  setStatus(`Deleted "${name}".`);
});

$("color-mode").addEventListener("change", () => {
  focusKeys.clear();
  showPlan(true);
});
$("reset-cam").addEventListener("click", () => viewer.resetCamera());
$("step").addEventListener("input", () => {
  stopPlaying();
  updateStep();
});
$("first").addEventListener("click", () => { stopPlaying(); setStep(0); });
$("prev").addEventListener("click", () => { stopPlaying(); setStep(Number($<HTMLInputElement>("step").value) - 1); });
$("next").addEventListener("click", () => { stopPlaying(); setStep(Number($<HTMLInputElement>("step").value) + 1); });
$("last").addEventListener("click", () => { stopPlaying(); setStep(Number($<HTMLInputElement>("step").max)); });
$("play").addEventListener("click", togglePlay);

viewer.onPick((p) => {
  selected = p;
  viewer.highlight(p?.instance_id ?? null);
  renderResults();
  renderManualBar();
});

window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key === "Enter") return void runPack();
  const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement;
  if (!typing && mode === "manual" && manualKey(e)) {
    e.preventDefault();
    return;
  }
  if (typing || !currentPlan()) return;
  const n = Number($<HTMLInputElement>("step").value);
  const keys: Record<string, () => void> = {
    ArrowRight: () => setStep(n + 1),
    ArrowLeft: () => setStep(n - 1),
    Home: () => setStep(0),
    End: () => setStep(Number($<HTMLInputElement>("step").max)),
    " ": togglePlay,
  };
  const action = keys[e.key];
  if (action) {
    e.preventDefault();
    if (e.key !== " ") stopPlaying();
    action();
  }
});

async function init() {
  try {
    presets = await invoke<TransportCase[]>("transport_presets");
  } catch {
    presets = [ROAD];
  }
  try {
    containerPresets = await invoke<ContainerSpec[]>("container_presets");
    vehiclePresets = await invoke<RoadVehicle[]>("vehicle_presets");
  } catch {
    // Presets are a convenience; the fields can still be typed in.
  }
  try {
    model = await invoke<ModelFile | null>("model_info");
  } catch {
    model = null;
  }
  // The local API is a desktop feature.
  try {
    const api = await invoke<ApiStatus>("api_status");
    $("api").hidden = !api.supported;
  } catch {
    $("api").hidden = true;
  }
  applyEditorWidth();
  renderEditor();
  renderResults();
  showPlan();
  refreshCatalog();
}
init();
