import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import {
  defaultOptions,
  defaultPhysics,
  defaultShape,
  newItem,
  ROAD,
  SHAPE_KINDS,
  type ContainerPlan,
  type Direction,
  type FillBias,
  type ItemSpec,
  type LoadPriority,
  type Objective,
  type OptimizeResult,
  type PackRequest,
  type PackResult,
  type Placement,
  type RenderMesh,
  type Score,
  type Shape,
  type SearchPhase,
  type SearchProgress,
  type SecuringClass,
  type ShapeKind,
  type StopOrder,
  type TransportCase,
  type TransportIssue,
  type Zone,
} from "./types";
import { PlanViewer } from "./viewer";

// ---------- state ----------

let req: PackRequest = emptyRequest();
let result: PackResult | null = null;
let stale = false;
let current = 0;
let selected: Placement | null = null;
let playing: number | null = null;
let presets: TransportCase[] = [ROAD];
/** "Best" fill mode: search patterns, orders and orientations (omnipack-opt). */
const search = loadSearchSettings();
/** Plans returned by the last search, best first; `result` is one of them. */
let searchResult: OptimizeResult | null = null;
let searchPick = 0;
let searching = false;

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
  if (result) {
    stale = true;
    renderResults();
  }
}

/** Numeric input bound to a getter/setter. `nullable`: empty = null. */
function num(label: string, get: () => number | null, set: (v: number | null) => void, opts: { nullable?: boolean; min?: number; max?: number; step?: string; placeholder?: string } = {}) {
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
    changed();
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
    default:
      return "";
  }
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
  } else if (mode === "load") {
    legend.append(row(heat(0), "unloaded / no limit"), row(heat(0.5), "50% of limit"), row(heat(1), "at limit"), row("#6b7280", "fragile (nothing on top)"));
  } else {
    legend.append(row(heat(0), "large margin"), row(heat(0.5), "moderate"), row(heat(1), "near tipping edge"));
  }
  if (focusKeys.size) {
    legend.append(h("button", { class: "small", onclick: () => { focusKeys.clear(); applyFocus(); renderLegend(); } }, "Show all"));
  } else if (["item", "stop", "securing", "impact"].includes(mode)) {
    legend.append(h("p", { class: "hint" }, "Click an entry to isolate it"));
  }
}

// ---------- editor ----------

function renderEditor() {
  const c = req.container;
  const o = req.options;
  const ph = o.physics;
  const ed = $("editor");
  ed.replaceChildren(
    h("h3", {}, "Container"),
    h(
      "div",
      { class: "grid" },
      text("Name", () => c.id, (v) => (c.id = v || "container")),
      num("Max payload kg", () => c.max_payload, (v) => (c.max_payload = v), { nullable: true, placeholder: "∞" }),
      num("Max CoG offset mm", () => c.cog_limits.max_lateral_offset, (v) => (c.cog_limits.max_lateral_offset = v), { nullable: true, placeholder: "—" }),
      num("Width mm (X)", () => c.width, (v) => (c.width = Math.max(1, v ?? 1)), { min: 1 }),
      num("Height mm (Y)", () => c.height, (v) => (c.height = Math.max(1, v ?? 1)), { min: 1 }),
      num("Depth mm (Z)", () => c.depth, (v) => (c.depth = Math.max(1, v ?? 1)), { min: 1 }),
    ),
    h("p", { class: "hint" }, "The door is at the far end of the depth axis (orange outline); the front wall is at depth 0."),

    h("h3", {}, "Loading strategy"),
    h(
      "div",
      { class: "grid two" },
      select<StopOrder>(
        "Unloading order",
        [
          ["lifo", "LIFO – last stop loaded first"],
          ["fifo", "FIFO – first stop loaded first"],
        ],
        () => o.stop_order,
        (v) => (o.stop_order = v),
      ),
      select<LoadPriority>(
        "Within a stop, load",
        [
          ["volume", "Largest first"],
          ["mass", "Heaviest first"],
          ["base_area", "Largest base first"],
          ["height", "Tallest first"],
          ["as_listed", "As listed (grouped)"],
        ],
        () => o.priority,
        (v) => (o.priority = v),
      ),
      select<FillBias | "best">(
        "Fill pattern",
        [
          ["best", "★ Best: search all patterns & orders"],
          ["wall_building", "Walls across width"],
          ["floor_first", "Floor layers first"],
          ["longitudinal", "Walls along length"],
          ["lateral", "Floor rows along length"],
          ["corner_first", "From a corner"],
        ],
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
          h("p", { class: "hint" }, "Tries every fill pattern and load priority, then evolves loading orders and orientations (genetic search + local search). Every plan gets the full physics check; stops, zones and floor-only rules are kept."),
          slider("Search time", 5, 120, 5, () => search.budget, (v) => ((search.budget = v), saveSearchSettings()), (v) => `${v.toFixed(0)} s`),
          slider("Prefer", 0, 1, 0.05, () => search.securing, (v) => ((search.securing = v), saveSearchSettings()), (v) => (v < 0.2 ? "max. density" : v > 0.8 ? "least securing" : "balanced")),
        )
      : "",
    h("p", { class: "hint" }, o.stop_order === "fifo"
      ? "FIFO fills from the door towards the back, so the units loaded first are unloaded first (side loading / drive-through)."
      : "LIFO fills from the back wall towards the door; the first stop ends up at the door (rear-door vehicles)."),
    slider("Stability margin (share of half-footprint)", 0, 0.5, 0.01, () => o.stability_margin, (v) => (o.stability_margin = v)),
    slider("Minimum support area", 0, 1, 0.05, () => o.min_support_ratio, (v) => (o.min_support_ratio = v), (v) => `${Math.round(v * 100)}%`),
    slider("Balance (keep CoG centred)", 0, 1, 0.05, () => o.balance_weight, (v) => (o.balance_weight = v)),
    h("div", { class: "checks" }, check("Allow rotation", () => o.allow_rotation, (v) => (o.allow_rotation = v))),

    h("h3", {}, "Physics & transport"),
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
              changed();
            });
            return cb;
          })(),
          pc.name,
          h("small", {}, ` ${pc.forward}/${pc.backward}/${pc.sideways} g`),
        );
      }),
    ),
    h(
      "div",
      { class: "checks" },
      check("Sliding", () => ph.check_sliding, (v) => (ph.check_sliding = v), "Friction vs acceleration, unless blocked by walls or neighbours"),
      check("Tipping", () => ph.check_tipping, (v) => (ph.check_tipping = v), "Tipping moment vs restoring moment, unless blocked above the CoG"),
      check("Dynamic stacking", () => ph.dynamic_stacking, (v) => (ph.dynamic_stacking = v), "Multiply loads on top by the vertical factor"),
      check("Chocks for round items", () => ph.use_chocks, (v) => (ph.use_chocks = v), "Lying drums and balls are held by wedges; off = they must be wedged in by neighbours"),
      check("Load end secured", () => ph.secure_load_end, (v) => (ph.secure_load_end = v), "A locking bar / gate / dunnage closes the open end of the load"),
      check("Anti-slip mats", () => ph.anti_slip_mats, (v) => (ph.anti_slip_mats = v), `Rubber mats under every item and between layers: μ ≥ ${ANTI_SLIP_FRICTION}`),
    ),
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

    h(
      "h3",
      {},
      `Items (${req.items.reduce((s, i) => s + i.quantity, 0)} units)`,
      h("span", { class: "spacer" }),
      (() => {
        const sel = h("select", { title: "Add an item of this shape" }, h("option", { value: "" }, "+ Add…"), ...SHAPE_KINDS.map(([k, t]) => h("option", { value: k }, t))) as HTMLSelectElement;
        sel.addEventListener("change", () => {
          if (sel.value) addItem(sel.value as ShapeKind);
          sel.value = "";
        });
        return sel;
      })(),
    ),
    ...req.items.map(itemCard),
  );
}

function addItem(kind: ShapeKind) {
  const it = newItem(req.items.length + 1);
  while (req.items.some((i) => i.id === it.id)) it.id += "'";
  it.shape = defaultShape(kind);
  req.items.push(it);
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
  return h(
    "div",
    { class: "card", style: `border-left-color:${it.color ?? paletteColor(index)}` },
    h("div", { class: "card-head" }, color, idInput, shapeSel, remove),
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

function describeViolation(v: Record<string, unknown>): string {
  const { kind, ...rest } = v;
  return `${String(kind).replace(/_/g, " ")}: ${Object.values(rest).map((x) => (typeof x === "number" ? fmt(x, 1) : String(x))).join(", ")}`;
}

const DIR_LABEL: Record<Direction, string> = { forward: "forward (braking)", backward: "backward", left: "left", right: "right" };

function describeIssue(i: TransportIssue): string {
  if (i.kind === "stack_overload") return `${i.item}: stack overloaded by ${fmt(i.required, 1)} kg at ${i.acceleration} g`;
  const force = i.required < 0.01 ? "< 0.01" : fmt(i.required, 2);
  return `${i.item}: ${i.kind} ${DIR_LABEL[i.direction!]} at ${i.acceleration} g → secure with ≥ ${force} kN`;
}

function scoreLine(s: Score): string {
  const lash = s.lashing_units ? `${s.lashing_units} to lash (${fmt(s.lashing_kn, 1)} kN)` : "nothing to lash";
  return `${fmt(s.volume_utilization * 100, 1)} % vol · ${s.containers} cont. · ${lash} · dunnage ${fmt(s.dunnage_mm / 1000, 2)} m`;
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
  panel.append(
    h("h3", {}, "Plan", h("span", { class: "spacer" }), stale ? h("span", { class: "badge warn" }, "out of date") : null),
    h(
      "div",
      { class: "actions" },
      h("span", { class: `badge ${valid ? "ok" : "bad"}` }, valid ? "✓ Stable at rest" : "✗ Violations found"),
      req.options.physics.transport.length
        ? h("span", { class: `badge ${unsecured ? "warn" : "ok"}` }, unsecured ? `⚠ ${unsecured} units need lashing` : "✓ Secured for transport")
        : null,
      dunnage ? h("span", { class: "badge" }, `${dunnage} held once gaps are filled`) : null,
      chocks ? h("span", { class: "badge warn" }, `${chocks} need chocks`) : null,
    ),
    searchResult ? searchPicker(searchResult) : "",
    stat("Units packed", `${r.packed_units} / ${r.requested_units}`),
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
              h("ul", { class: "issue-list" }, ...gaps.map((g) => h("li", {}, `${g.item} ${DIR_LABEL[g.direction]} → ${g.other ?? "wall / load end"}: ${fmt(g.gap_mm)} mm`))),
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
      p.needs_chocks ? stat("Chocks", "required") : "",
      p.securing ? stat("Securing", SECURING.find(([k]) => k === p.securing)?.[2] ?? p.securing) : "",
      p.impact ? stat("Transport force", `${fmt(p.impact.force_kn, 2)} kN, ${DIR_LABEL[p.impact.direction]} (${p.impact.case})`) : "",
      p.impact ? stat("Demand / own grip", `${fmt(p.impact.ratio, 2)}×${p.impact.ratio > 1 ? " — relies on blocking or lashing" : ""}`) : "",
      own.length ? h("ul", { class: "issue-list" }, ...own.map((i) => h("li", {}, describeIssue(i)))) : "",
    );
  }

  if (r.unpacked.length) {
    const byReason = new Map<string, string[]>();
    for (const u of r.unpacked) byReason.set(u.reason, [...(byReason.get(u.reason) ?? []), u.instance_id]);
    panel.append(
      h("h3", {}, `Not packed (${r.unpacked.length})`),
      h("ul", { class: "list" }, ...[...byReason].map(([reason, ids]) => h("li", {}, `${reason.replace(/_/g, " ")}: ${ids.slice(0, 8).join(", ")}${ids.length > 8 ? ` +${ids.length - 8}` : ""}`))),
    );
  }

  panel.append(
    h(
      "div",
      { class: "actions" },
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
  viewer.show(plan, colorFor, (p) => ({ key: meshKey(p), data: meshCache.get(meshKey(p))! }), plan.placements.length ? plan.metrics.center_of_mass : null);
  step.max = String(plan.placements.length);
  step.value = String(keepStep ? Math.min(prev, plan.placements.length) : plan.placements.length);
  applyFocus();
  updateStep();
  renderLegend();
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
  return { density: 1, securing: 2 * t, dunnage: 0.6 * t, stability: 1 };
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
    setStatus(`${phaseName[p.phase]}: ${p.evaluated} plans, best ${fmt(p.best.volume_utilization * 100, 1)} % vol, ${p.best.lashing_units} to lash · ${left.toFixed(0)} s left`);
  });
  try {
    const options = { budget_ms: search.budget * 1000, objective: objective(), keep: 3 };
    const r = await invoke<OptimizeResult>("optimize_request", { request: req, options });
    searchResult = r;
    searchPick = 0;
    result = r.solutions[0].result;
    stale = false;
    current = 0;
    selected = null;
    focusKeys.clear();
    const best = r.solutions[0].score;
    setStatus(
      `${r.cancelled ? "Stopped" : "Done"}: ${r.evaluated} plans in ${fmt(r.elapsed_ms / 1000, 1)} s · best ${fmt(best.volume_utilization * 100, 1)} % vol, ${best.lashing_units} to lash`,
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
  if (search.enabled) return runSearch();
  searchResult = null;
  const btn = $<HTMLButtonElement>("pack");
  const btnMobile = $<HTMLButtonElement>("pack-mobile");
  btn.disabled = btnMobile.disabled = true;
  stopPlaying();
  setStatus("Packing…");
  try {
    result = await invoke<PackResult>("pack_request", { request: req });
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
    options: { ...defaultOptions(), ...r.options, physics: { ...defaultPhysics(), ...r.options?.physics } },
  };
  result = null;
  stale = false;
  selected = null;
  focusKeys.clear();
  stopPlaying();
  renderEditor();
  renderResults();
  showPlan();
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
  const rows = [["container", "seq", "unit", "item", "shape", "x_mm", "y_mm", "z_mm", "w_mm", "h_mm", "d_mm", "orientation", "mass_kg", "load_on_top_kg", "stop", "chocks", "securing"]];
  for (const c of result.containers) {
    const issues = issuesByItem(c);
    for (const p of c.placements)
      rows.push([
        c.id, String(p.seq + 1), p.instance_id, p.item_id, p.shape.kind,
        ...p.position.map((v) => v.toFixed(1)), ...p.size.map((v) => v.toFixed(1)),
        p.orientation, p.mass.toFixed(2), p.load_on_top.toFixed(2), String(p.stop),
        p.needs_chocks ? "yes" : "", (issues.get(p.instance_id) ?? []).map(describeIssue).join("; "),
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

// ---------- wiring ----------

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
});

window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key === "Enter") return void runPack();
  const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement;
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
  renderEditor();
  renderResults();
  showPlan();
  refreshCatalog();
}
init();
