import { invoke } from "@tauri-apps/api/core";
import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import {
  defaultOptions,
  newItem,
  type ContainerPlan,
  type FillBias,
  type ItemSpec,
  type PackRequest,
  type PackResult,
  type Placement,
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
function num(label: string, get: () => number | null, set: (v: number | null) => void, opts: { nullable?: boolean; min?: number; step?: string; placeholder?: string } = {}) {
  const input = h("input", {
    type: "number",
    step: opts.step ?? "any",
    min: opts.min,
    placeholder: opts.placeholder ?? "",
    value: get() ?? "",
  }) as HTMLInputElement;
  input.addEventListener("change", () => {
    const raw = input.value.trim();
    if (raw === "" && opts.nullable) set(null);
    else {
      const v = Number(raw);
      if (Number.isFinite(v)) set(v);
      else input.value = String(get() ?? "");
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

function check(label: string, get: () => boolean, set: (v: boolean) => void) {
  const input = h("input", { type: "checkbox" }) as HTMLInputElement;
  input.checked = get();
  input.addEventListener("change", () => {
    set(input.checked);
    changed();
  });
  return h("label", {}, input, label);
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

function colorFor(p: Placement): string {
  const mode = $<HTMLSelectElement>("color-mode").value;
  if (mode === "stop") return paletteColor(p.stop);
  if (mode === "load") {
    const cap = capacityOf(p.item_id);
    if (!Number.isFinite(cap)) return heat(0);
    return cap <= 0 ? (p.load_on_top > 0 ? heat(1) : "#6b7280") : heat(p.load_on_top / cap);
  }
  if (mode === "margin") {
    const half = Math.min(p.size[0], p.size[2]) / 2;
    return heat(1 - Math.min(1, p.support_margin / (0.5 * half)));
  }
  return p.color ?? itemColor(p.item_id);
}

function renderLegend() {
  const legend = $("legend");
  legend.replaceChildren();
  const plan = currentPlan();
  if (!plan) return;
  const row = (color: string, label: string) => h("div", { class: "row" }, h("span", { class: "swatch", style: `background:${color}` }), label);
  const mode = $<HTMLSelectElement>("color-mode").value;
  if (mode === "item") {
    const ids = [...new Set(plan.placements.map((p) => p.item_id))];
    ids.sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
    for (const id of ids) legend.append(row(itemColor(id), id));
  } else if (mode === "stop") {
    for (const s of [...new Set(plan.placements.map((p) => p.stop))].sort((a, b) => a - b))
      legend.append(row(paletteColor(s), s === 0 ? "no stop" : `stop ${s}`));
  } else if (mode === "load") {
    legend.append(row(heat(0), "unloaded / no limit"), row(heat(0.5), "50% of limit"), row(heat(1), "at limit"), row("#6b7280", "fragile (nothing on top)"));
  } else {
    legend.append(row(heat(0), "large margin"), row(heat(0.5), "moderate"), row(heat(1), "near tipping edge"));
  }
}

// ---------- editor ----------

function renderEditor() {
  const c = req.container;
  const o = req.options;
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
    h("p", { class: "hint" }, "The door is at the far end of the depth axis (orange outline)."),

    h("h3", {}, "Strategy"),
    h(
      "div",
      { class: "grid two" },
      select<FillBias>(
        "Fill order",
        [
          ["wall_building", "Walls, back → door"],
          ["floor_first", "Floor layers first"],
          ["longitudinal", "Walls, left → right"],
          ["lateral", "Floor rows along length"],
          ["corner_first", "From back corner"],
        ],
        () => o.bias,
        (v) => (o.bias = v),
      ),
      num("Max containers", () => o.max_containers, (v) => (o.max_containers = Math.max(1, Math.round(v ?? 1))), { min: 1, step: "1" }),
    ),
    slider("Stability margin (share of half-footprint)", 0, 0.5, 0.01, () => o.stability_margin, (v) => (o.stability_margin = v)),
    slider("Minimum support area", 0, 1, 0.05, () => o.min_support_ratio, (v) => (o.min_support_ratio = v), (v) => `${Math.round(v * 100)}%`),
    slider("Balance (keep CoG centred)", 0, 1, 0.05, () => o.balance_weight, (v) => (o.balance_weight = v)),
    h("div", { class: "checks" }, check("Allow rotation", () => o.allow_rotation, (v) => (o.allow_rotation = v))),

    h(
      "h3",
      {},
      `Items (${req.items.reduce((s, i) => s + i.quantity, 0)} units)`,
      h("span", { class: "spacer" }),
      h("button", { class: "small", onclick: () => addItem("box") }, "+ Box"),
      h("button", { class: "small", onclick: () => addItem("cylinder") }, "+ Cylinder"),
    ),
    ...req.items.map(itemCard),
  );
}

function addItem(kind: "box" | "cylinder") {
  const it = newItem(req.items.length + 1);
  while (req.items.some((i) => i.id === it.id)) it.id += "'";
  if (kind === "cylinder") it.shape = { kind: "cylinder", radius: 150, length: 800 };
  req.items.push(it);
  renderEditor();
  changed();
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
  const shapeSel = h("select", {}, h("option", { value: "box" }, "Box"), h("option", { value: "cylinder" }, "Cylinder")) as HTMLSelectElement;
  shapeSel.value = it.shape.kind;
  shapeSel.addEventListener("change", () => {
    it.shape = shapeSel.value === "box" ? { kind: "box", w: 400, h: 300, d: 300 } : { kind: "cylinder", radius: 150, length: 800 };
    renderEditor();
    changed();
  });
  const remove = h("button", { class: "small", title: "Remove", onclick: () => {
    req.items.splice(index, 1);
    renderEditor();
    changed();
  } }, "✕");

  const s = it.shape;
  const dims =
    s.kind === "box"
      ? [
          num("W mm", () => s.w, (v) => (s.w = Math.max(1, v ?? 1)), { min: 1 }),
          num("H mm", () => s.h, (v) => (s.h = Math.max(1, v ?? 1)), { min: 1 }),
          num("D mm", () => s.d, (v) => (s.d = Math.max(1, v ?? 1)), { min: 1 }),
        ]
      : [
          num("Radius mm", () => s.radius, (v) => (s.radius = Math.max(1, v ?? 1)), { min: 1 }),
          num("Length mm", () => s.length, (v) => (s.length = Math.max(1, v ?? 1)), { min: 1 }),
          h("span"),
        ];

  return h(
    "div",
    { class: "card", style: `border-left-color:${it.color ?? paletteColor(index)}` },
    h("div", { class: "card-head" }, color, idInput, shapeSel, remove),
    h(
      "div",
      { class: "grid" },
      ...dims,
      num("Mass kg", () => it.mass, (v) => (it.mass = Math.max(0, v ?? 0)), { min: 0 }),
      num("Quantity", () => it.quantity, (v) => (it.quantity = Math.max(0, Math.round(v ?? 0))), { min: 0, step: "1" }),
      num("Max load on top kg", () => it.max_load_on_top, (v) => (it.max_load_on_top = v), { nullable: true, placeholder: "∞" }),
      num("Stop (1 = first off)", () => it.stop, (v) => (it.stop = Math.max(0, Math.round(v ?? 0))), { min: 0, step: "1" }),
      select<Zone>("Zone", [["any", "Anywhere"], ["back", "Back"], ["front", "Near door"]], () => it.zone, (v) => (it.zone = v)),
    ),
    h(
      "div",
      { class: "checks" },
      check("Fragile", () => it.fragile, (v) => (it.fragile = v)),
      check(s.kind === "box" ? "This side up" : "Upright only", () => it.upright_only, (v) => (it.upright_only = v)),
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

function renderResults() {
  const panel = $("results");
  panel.replaceChildren();
  if (!result) {
    panel.append(h("h3", {}, "Results"), h("p", { class: "hint" }, "No plan yet."));
    return;
  }
  const r = result;
  const valid = r.containers.every((c) => c.violations.length === 0);
  panel.append(
    h("h3", {}, "Plan", h("span", { class: "spacer" }), stale ? h("span", { class: "badge warn" }, "out of date") : null),
    h("div", {}, h("span", { class: `badge ${valid ? "ok" : "bad"}` }, valid ? "✓ Physically valid" : "✗ Violations found")),
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
  }

  if (selected) {
    const p = selected;
    const cap = capacityOf(p.item_id);
    panel.append(
      h("h3", {}, `Selected: ${p.instance_id}`),
      stat("Load order", `#${p.seq + 1}`),
      stat("Position", p.position.map((v) => fmt(v)).join(", ") + " mm"),
      stat("Size", p.size.map((v) => fmt(v)).join(" × ") + " mm"),
      stat("Orientation", p.orientation),
      stat("Mass", `${fmt(p.mass, 1)} kg`),
      stat("Load on top", `${fmt(p.load_on_top, 1)} kg${Number.isFinite(cap) ? ` of ${fmt(cap)} kg` : ""}`),
      stat("Stability margin", `${fmt(p.support_margin, 1)} mm`),
      stat("Stop", p.stop === 0 ? "—" : String(p.stop)),
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

function showPlan() {
  const plan = currentPlan();
  $("empty").style.display = plan ? "none" : "grid";
  const tabs = $("container-tabs");
  tabs.replaceChildren(
    ...(result?.containers ?? []).map((_, i) =>
      h("button", { class: i === current ? "active" : "", onclick: () => selectContainer(i) }, `#${i + 1}`),
    ),
  );
  if (!plan) {
    viewer.clear();
    renderLegend();
    return;
  }
  viewer.show(plan, colorFor, plan.placements.length ? plan.metrics.center_of_mass : null);
  const step = $<HTMLInputElement>("step");
  step.max = String(plan.placements.length);
  step.value = String(plan.placements.length);
  updateStep();
  renderLegend();
}

function selectContainer(i: number) {
  current = i;
  selected = null;
  showPlan();
  renderResults();
}

function updateStep() {
  const plan = currentPlan();
  const n = Number($<HTMLInputElement>("step").value);
  viewer.setVisibleCount(n);
  const last = plan?.placements[n - 1];
  $("step-label").textContent = plan ? `${n} / ${plan.placements.length}${last ? ` · ${last.instance_id}` : ""}` : "";
}

// ---------- actions ----------

function setStatus(msg: string, kind: "" | "ok" | "bad" = "") {
  const s = $("status");
  s.textContent = msg;
  s.className = `status ${kind}`;
}

async function runPack() {
  if (!req.items.length) {
    setStatus("Add some items first.", "bad");
    return;
  }
  const btn = $<HTMLButtonElement>("pack");
  btn.disabled = true;
  setStatus("Packing…");
  try {
    result = await invoke<PackResult>("pack_request", { request: req });
    stale = false;
    current = 0;
    selected = null;
    const valid = result.containers.every((c) => c.violations.length === 0);
    setStatus(
      `${result.packed_units}/${result.requested_units} units in ${result.containers.length} container(s), ${result.elapsed_ms} ms`,
      valid && result.unpacked.length === 0 ? "ok" : "bad",
    );
    showPlan();
    renderResults();
  } catch (e) {
    setStatus(String(e), "bad");
  } finally {
    btn.disabled = false;
  }
}

function loadRequest(r: PackRequest) {
  // Fill in anything older files may lack.
  req = {
    container: { ...emptyRequest().container, ...r.container, cog_limits: { ...emptyRequest().container.cog_limits, ...r.container.cog_limits } },
    items: r.items.map((i) => ({ ...newItem(0), ...i })),
    options: { ...defaultOptions(), ...r.options },
  };
  result = null;
  stale = false;
  selected = null;
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
    await invoke("write_text_file", { path, contents });
    setStatus(`Saved ${path}`, "ok");
  } catch (e) {
    setStatus(String(e), "bad");
  }
}

async function openRequest() {
  const path = await open({ multiple: false, filters: [{ name: "OmniPack setup", extensions: ["json"] }] });
  if (!path || Array.isArray(path)) return;
  try {
    const text = await invoke<string>("read_text_file", { path });
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
  const rows = [["container", "seq", "unit", "item", "x_mm", "y_mm", "z_mm", "w_mm", "h_mm", "d_mm", "orientation", "mass_kg", "load_on_top_kg", "stop"]];
  for (const c of result.containers)
    for (const p of c.placements)
      rows.push([c.id, String(p.seq + 1), p.instance_id, p.item_id, ...p.position.map((v) => v.toFixed(1)), ...p.size.map((v) => v.toFixed(1)), p.orientation, p.mass.toFixed(2), p.load_on_top.toFixed(2), String(p.stop)]);
  const csv = rows.map((r) => r.map((v) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v)).join(",")).join("\n");
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

// ---------- wiring ----------

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
  const name = window.prompt("Save this setup to the catalog as:", req.container.id);
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
  const n = $<HTMLInputElement>("step").value;
  showPlan();
  $<HTMLInputElement>("step").value = n;
  updateStep();
});
$("reset-cam").addEventListener("click", () => viewer.resetCamera());
$("step").addEventListener("input", updateStep);
$("play").addEventListener("click", () => {
  const step = $<HTMLInputElement>("step");
  const btn = $("play");
  if (playing !== null) {
    clearInterval(playing);
    playing = null;
    btn.textContent = "▶";
    return;
  }
  if (step.value === step.max) step.value = "0";
  btn.textContent = "⏸";
  playing = window.setInterval(() => {
    const n = Number(step.value) + 1;
    step.value = String(n);
    updateStep();
    if (n >= Number(step.max)) {
      clearInterval(playing!);
      playing = null;
      btn.textContent = "▶";
    }
  }, 250);
});

viewer.onPick((p) => {
  selected = p;
  viewer.highlight(p?.instance_id ?? null);
  renderResults();
});

window.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key === "Enter") runPack();
});

renderEditor();
renderResults();
showPlan();
refreshCatalog();
