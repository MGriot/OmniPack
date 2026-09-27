// Small 3D previews of single items for the editor cards. One hidden WebGL
// engine renders every preview on demand (no render loop) and each card's 2D
// canvas receives a copy, so twenty cards still use a single GL context.

import { invoke } from "@tauri-apps/api/core";
import {
  ArcRotateCamera,
  Color3,
  Color4,
  DirectionalLight,
  Engine,
  HemisphericLight,
  Mesh,
  MeshBuilder,
  Scene,
  StandardMaterial,
  Vector3,
  VertexData,
} from "@babylonjs/core";
import type { ItemPreview, ItemSpec } from "./types";

/** Preview size in CSS pixels. */
const SIZE = 96;

let engine: Engine | null = null;

function sharedEngine(): Engine {
  if (!engine) {
    const canvas = document.createElement("canvas");
    const px = Math.round(SIZE * Math.min(2, window.devicePixelRatio || 1));
    canvas.width = canvas.height = px;
    engine = new Engine(canvas, true, { preserveDrawingBuffer: true, stencil: false });
  }
  return engine;
}

class Preview {
  readonly scene: Scene;
  private camera: ArcRotateCamera;
  private parts: Mesh[] = [];
  private material: StandardMaterial;
  private key = "";
  private loading: Promise<void> | null = null;
  info: ItemPreview | null = null;
  canvas: HTMLCanvasElement | null = null;
  private listeners: ((info: ItemPreview) => void)[] = [];

  constructor() {
    this.scene = new Scene(sharedEngine());
    this.scene.clearColor = new Color4(0.1, 0.11, 0.14, 1);
    this.scene.autoClear = true;
    this.camera = new ArcRotateCamera("cam", Math.PI / 2 - 0.7, 1.05, 10, Vector3.Zero(), this.scene);
    const hemi = new HemisphericLight("hemi", new Vector3(0.2, 1, -0.3), this.scene);
    hemi.intensity = 0.8;
    hemi.groundColor = new Color3(0.25, 0.25, 0.3);
    const sun = new DirectionalLight("sun", new Vector3(-0.4, -1, 0.6), this.scene);
    sun.intensity = 0.5;
    this.material = new StandardMaterial("item", this.scene);
    this.material.specularColor = new Color3(0.08, 0.08, 0.08);
    this.material.backFaceCulling = false;
    this.material.twoSidedLighting = true;
  }

  /** Fetches geometry and centre of mass when the item changed, then draws. */
  update(it: ItemSpec, color: string): Promise<void> {
    this.material.diffuseColor = Color3.FromHexString(color);
    const key = JSON.stringify([it.shape, it.com_offset]);
    if (key === this.key) {
      this.render();
      return this.loading ?? Promise.resolve();
    }
    this.key = key;
    this.loading = invoke<ItemPreview>("item_preview", { shape: it.shape, comOffset: it.com_offset })
      .then((info) => {
        if (key !== this.key) return;
        this.info = info;
        this.build(info);
        this.render();
        for (const cb of this.listeners) cb(info);
      })
      .catch(() => {});
    return this.loading;
  }

  onInfo(cb: (info: ItemPreview) => void) {
    this.listeners = [cb];
    if (this.info) cb(this.info);
  }

  private build(info: ItemPreview) {
    for (const m of this.parts) m.dispose();
    this.parts = [];
    const [w, h, d] = info.extents;
    const s = Math.max(w, h, d);
    const v = (x: number, y: number, z: number) => new Vector3(x, y, z);

    const mesh = new Mesh("item", this.scene);
    const vd = new VertexData();
    vd.positions = info.mesh.positions;
    vd.indices = info.mesh.indices;
    const normals: number[] = [];
    VertexData.ComputeNormals(info.mesh.positions, info.mesh.indices, normals);
    vd.normals = normals;
    vd.applyToMesh(mesh);
    mesh.position = v(w / 2, h / 2, d / 2); // mesh is centred on its box
    mesh.material = this.material;
    mesh.enableEdgesRendering(0.95, true);
    mesh.edgesWidth = 2;
    mesh.edgesColor = new Color4(0, 0, 0, 0.5);
    mesh.visibility = 0.85;

    // Bounding box, and X/Y/Z axes (red/green/blue) from the base corner.
    const box = MeshBuilder.CreateLineSystem("box", {
      lines: [
        [v(0, 0, 0), v(w, 0, 0), v(w, 0, d), v(0, 0, d), v(0, 0, 0)],
        [v(0, h, 0), v(w, h, 0), v(w, h, d), v(0, h, d), v(0, h, 0)],
        [v(w, 0, 0), v(w, h, 0)], [v(w, 0, d), v(w, h, d)], [v(0, 0, d), v(0, h, d)],
      ],
    }, this.scene);
    box.color = new Color3(0.55, 0.6, 0.7);
    const axis = (name: string, to: Vector3, c: Color3) => {
      const l = MeshBuilder.CreateLines(name, { points: [v(0, 0, 0), to] }, this.scene);
      l.color = c;
      return l;
    };
    const ax = s * 0.25;
    const axes = [
      axis("x", v(w + ax, 0, 0), new Color3(1, 0.3, 0.3)),
      axis("y", v(0, h + ax, 0), new Color3(0.3, 1, 0.4)),
      axis("z", v(0, 0, d + ax), new Color3(0.35, 0.55, 1)),
    ];

    // Centre of mass: a ball drawn on top of everything, dropped to the base.
    const [cx, cy, cz] = info.com;
    const cog = MeshBuilder.CreateSphere("cog", { diameter: s / 12 }, this.scene);
    cog.position = v(cx, cy, cz);
    const cm = new StandardMaterial("cogMat", this.scene);
    cm.emissiveColor = new Color3(1, 0.1, 0.4);
    cm.disableLighting = true;
    cog.material = cm;
    cog.renderingGroupId = 1;
    const drop = MeshBuilder.CreateLines("drop", { points: [v(cx, cy, cz), v(cx, 0, cz)] }, this.scene);
    drop.color = new Color3(1, 0.1, 0.4);
    drop.renderingGroupId = 1;
    this.parts.push(mesh, box, ...axes, cog, drop);

    this.camera.target = v(w / 2, h / 2, d / 2);
    this.camera.radius = Math.hypot(w, h, d) * 1.8;
    this.camera.minZ = this.camera.radius / 100;
    this.camera.maxZ = this.camera.radius * 10;
  }

  /** Draws the scene and copies it to the card's canvas. */
  render() {
    const c = this.canvas;
    if (!c || !this.info || !c.isConnected) return;
    const e = sharedEngine();
    const src = e.getRenderingCanvas()!;
    // Shaders compile asynchronously: draw again once everything is ready.
    if (!this.scene.isReady()) this.scene.executeWhenReady(() => this.render());
    this.scene.render();
    const ctx = c.getContext("2d");
    if (!ctx) return;
    if (c.width !== src.width) c.width = c.height = src.width;
    ctx.clearRect(0, 0, c.width, c.height);
    ctx.drawImage(src, 0, 0);
  }

  /** Drag to orbit (mouse or finger). */
  orbit(dx: number, dy: number) {
    this.camera.alpha -= dx * 0.012;
    this.camera.beta = Math.min(Math.PI - 0.05, Math.max(0.05, this.camera.beta - dy * 0.012));
    this.render();
  }

  dispose() {
    this.scene.dispose();
  }
}

const previews = new Map<ItemSpec, Preview>();

const seen = new IntersectionObserver((entries) => {
  for (const e of entries) {
    if (!e.isIntersecting) continue;
    seen.unobserve(e.target);
    (e.target as HTMLElement).dispatchEvent(new Event("preview-visible"));
  }
});

/**
 * A preview canvas for `it`. `onInfo` receives the item's bounding box and
 * centre of mass whenever they are (re)loaded.
 */
export function previewCanvas(it: ItemSpec, color: string, onInfo: (info: ItemPreview) => void): { canvas: HTMLCanvasElement; refresh: () => void } {
  let p = previews.get(it);
  if (!p) {
    p = new Preview();
    previews.set(it, p);
  }
  const preview = p;
  const canvas = document.createElement("canvas");
  canvas.className = "item-preview";
  canvas.title = "Drag to rotate · pink dot = centre of mass · red/green/blue = X/Y/Z";
  canvas.width = canvas.height = SIZE;
  preview.canvas = canvas;
  preview.onInfo(onInfo);

  let last: [number, number] | null = null;
  canvas.addEventListener("pointerdown", (e) => {
    last = [e.clientX, e.clientY];
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!last) return;
    preview.orbit(e.clientX - last[0], e.clientY - last[1]);
    last = [e.clientX, e.clientY];
  });
  const end = () => (last = null);
  canvas.addEventListener("pointerup", end);
  canvas.addEventListener("pointercancel", end);

  let colorNow = color;
  const refresh = () => void preview.update(it, colorNow);
  canvas.addEventListener("preview-visible", refresh);
  seen.observe(canvas);
  return {
    canvas,
    refresh: () => {
      colorNow = it.color ?? color;
      refresh();
    },
  };
}

/** Drops the previews of items that are gone. */
export function prunePreviews(items: ItemSpec[]) {
  const live = new Set(items);
  for (const [it, p] of previews) {
    if (!live.has(it)) {
      p.dispose();
      previews.delete(it);
    }
  }
}
