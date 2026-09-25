// Babylon.js view of one container plan. Engine coordinates map 1:1 to
// Babylon's (Y up); positions are AABB minimum corners, and item meshes come
// from the engine (centred on the AABB), so what you see is what the physics
// checked.

import {
  ArcRotateCamera,
  Color3,
  Color4,
  DirectionalLight,
  Engine,
  HemisphericLight,
  LinesMesh,
  Mesh,
  MeshBuilder,
  PointerEventTypes,
  Scene,
  StandardMaterial,
  Vector3,
  VertexData,
} from "@babylonjs/core";
import type { ContainerPlan, Placement, RenderMesh } from "./types";

/** Opacity of items outside the legend focus. */
const DIMMED = 0.1;
/** Opacity of the "next item" preview. */
const GHOST = 0.4;

export class PlanViewer {
  private engine: Engine;
  private scene: Scene;
  private camera: ArcRotateCamera;
  private meshes: { placement: Placement; mesh: Mesh }[] = [];
  private staticMeshes: (Mesh | LinesMesh)[] = [];
  private templates = new Map<string, Mesh>();
  private materials = new Map<string, StandardMaterial>();
  private highlighted: string | null = null;
  private focus: ((p: Placement) => boolean) | null = null;
  private visibleCount = Infinity;
  private pickHandler: (p: Placement | null) => void = () => {};
  private size: [number, number, number] = [1, 1, 1];

  constructor(canvas: HTMLCanvasElement) {
    this.engine = new Engine(canvas, true, { preserveDrawingBuffer: false, stencil: true });
    this.scene = new Scene(this.engine);
    this.scene.clearColor = new Color4(0.07, 0.08, 0.1, 1);
    this.camera = new ArcRotateCamera("cam", -Math.PI / 3, Math.PI / 3, 10, Vector3.Zero(), this.scene);
    this.camera.attachControl(canvas, true);
    this.camera.wheelDeltaPercentage = 0.02;
    const hemi = new HemisphericLight("hemi", new Vector3(0.2, 1, -0.3), this.scene);
    hemi.intensity = 0.75;
    hemi.groundColor = new Color3(0.25, 0.25, 0.3);
    const sun = new DirectionalLight("sun", new Vector3(-0.4, -1, 0.6), this.scene);
    sun.intensity = 0.55;

    this.scene.onPointerObservable.add((info) => {
      if (info.type !== PointerEventTypes.POINTERTAP) return;
      const picked = info.pickInfo?.pickedMesh;
      const hit = this.meshes.find((m) => m.mesh === picked);
      this.pickHandler(hit ? hit.placement : null);
    });
    this.engine.runRenderLoop(() => this.scene.render());
    new ResizeObserver(() => this.engine.resize()).observe(canvas);
  }

  onPick(cb: (p: Placement | null) => void) {
    this.pickHandler = cb;
  }

  private material(hex: string): StandardMaterial {
    let m = this.materials.get(hex);
    if (!m) {
      m = new StandardMaterial(`m${hex}`, this.scene);
      m.diffuseColor = Color3.FromHexString(hex);
      m.specularColor = new Color3(0.08, 0.08, 0.08);
      m.backFaceCulling = false;
      m.twoSidedLighting = true;
      this.materials.set(hex, m);
    }
    return m;
  }

  /** One hidden template mesh per shape+orientation; items are clones of it. */
  private template(key: string, data: RenderMesh): Mesh {
    let t = this.templates.get(key);
    if (!t) {
      t = new Mesh(`tpl:${key}`, this.scene);
      const vd = new VertexData();
      vd.positions = data.positions;
      vd.indices = data.indices;
      const normals: number[] = [];
      VertexData.ComputeNormals(data.positions, data.indices, normals);
      vd.normals = normals;
      vd.applyToMesh(t);
      t.setEnabled(false);
      t.isPickable = false;
      this.templates.set(key, t);
    }
    return t;
  }

  clear() {
    for (const { mesh } of this.meshes) mesh.dispose();
    for (const m of this.staticMeshes) m.dispose();
    this.meshes = [];
    this.staticMeshes = [];
  }

  show(
    plan: ContainerPlan,
    colorOf: (p: Placement) => string,
    meshOf: (p: Placement) => { key: string; data: RenderMesh },
    cog: [number, number, number] | null,
  ) {
    this.clear();
    const [W, H, D] = plan.size;
    const sizeChanged = W !== this.size[0] || H !== this.size[1] || D !== this.size[2];
    this.size = plan.size;
    this.drawContainer(W, H, D);

    for (const p of plan.placements) {
      const { key, data } = meshOf(p);
      const mesh = this.template(key, data).clone(p.instance_id);
      mesh.setEnabled(true);
      mesh.isPickable = true;
      mesh.position = new Vector3(p.position[0] + p.size[0] / 2, p.position[1] + p.size[1] / 2, p.position[2] + p.size[2] / 2);
      mesh.material = this.material(colorOf(p));
      mesh.enableEdgesRendering(0.95, true); // match edges by position: meshes are unwelded
      mesh.edgesWidth = 3;
      mesh.edgesColor = new Color4(0, 0, 0, 0.5);
      this.meshes.push({ placement: p, mesh });
    }
    if (cog) this.drawCog(cog);
    if (sizeChanged) this.resetCamera();
    this.refresh();
  }

  private drawContainer(W: number, H: number, D: number) {
    const c = (x: number, y: number, z: number) => new Vector3(x, y, z);
    const edges = [
      [c(0, 0, 0), c(W, 0, 0)], [c(W, 0, 0), c(W, 0, D)], [c(W, 0, D), c(0, 0, D)], [c(0, 0, D), c(0, 0, 0)],
      [c(0, H, 0), c(W, H, 0)], [c(W, H, 0), c(W, H, D)], [c(W, H, D), c(0, H, D)], [c(0, H, D), c(0, H, 0)],
      [c(0, 0, 0), c(0, H, 0)], [c(W, 0, 0), c(W, H, 0)], [c(W, 0, D), c(W, H, D)], [c(0, 0, D), c(0, H, D)],
    ];
    const frame = MeshBuilder.CreateLineSystem("frame", { lines: edges }, this.scene);
    frame.color = new Color3(0.6, 0.65, 0.75);
    frame.isPickable = false;

    // Door (z = depth) outlined in orange.
    const door = MeshBuilder.CreateLines("door", { points: [c(0, 0, D), c(W, 0, D), c(W, H, D), c(0, H, D), c(0, 0, D)] }, this.scene);
    door.color = new Color3(1, 0.55, 0.15);
    door.isPickable = false;

    const floor = MeshBuilder.CreateGround("floor", { width: W, height: D }, this.scene);
    floor.position = new Vector3(W / 2, -0.5, D / 2);
    const fm = new StandardMaterial("floorMat", this.scene);
    fm.diffuseColor = new Color3(0.18, 0.2, 0.24);
    fm.specularColor = Color3.Black();
    floor.material = fm;
    floor.isPickable = false;
    this.staticMeshes.push(frame, door, floor);
  }

  private drawCog([x, y, z]: [number, number, number]) {
    const r = Math.max(...this.size) / 120;
    const ball = MeshBuilder.CreateSphere("cog", { diameter: 2 * r }, this.scene);
    ball.position = new Vector3(x, y, z);
    const m = new StandardMaterial("cogMat", this.scene);
    m.emissiveColor = new Color3(1, 0.1, 0.4);
    m.disableLighting = true;
    ball.material = m;
    ball.renderingGroupId = 1; // always visible
    ball.isPickable = false;
    const drop = MeshBuilder.CreateLines("cogDrop", { points: [new Vector3(x, y, z), new Vector3(x, 0, z)] }, this.scene);
    drop.color = new Color3(1, 0.1, 0.4);
    drop.renderingGroupId = 1;
    drop.isPickable = false;
    this.staticMeshes.push(ball, drop);
  }

  /** Shows placements with `seq < count`; the one with `seq == count` is drawn as a preview. */
  setVisibleCount(count: number) {
    this.visibleCount = count;
    this.refresh();
  }

  /** Items matching `f` stay solid, the rest are dimmed. `null` = all solid. */
  setFocus(f: ((p: Placement) => boolean) | null) {
    this.focus = f;
    this.refresh();
  }

  highlight(instanceId: string | null) {
    this.highlighted = instanceId;
    this.refresh();
  }

  private refresh() {
    for (const { placement: p, mesh } of this.meshes) {
      const placed = p.seq < this.visibleCount;
      const next = p.seq === this.visibleCount;
      mesh.setEnabled(placed || next);
      const focused = !this.focus || this.focus(p);
      mesh.visibility = next ? GHOST : focused ? 1 : DIMMED;
      mesh.isPickable = placed && focused;
      mesh.edgesColor = new Color4(0, 0, 0, focused || next ? 0.5 : 0.08);
      const hl = p.instance_id === this.highlighted;
      mesh.renderOverlay = hl || next;
      mesh.overlayColor = next ? new Color3(1, 0.6, 0.1) : new Color3(1, 1, 1);
      mesh.overlayAlpha = next ? 0.5 : 0.45;
    }
  }

  resetCamera() {
    const [W, H, D] = this.size;
    // Look in through the door (z = depth), from above and slightly to the side.
    this.camera.target = new Vector3(W / 2, H / 3, D / 2);
    // Portrait screens need more distance to fit the container's width.
    const aspect = this.engine.getAspectRatio(this.camera) || 1;
    this.camera.radius = Math.hypot(W, H, D) * 1.6 * Math.max(1, 1 / aspect);
    this.camera.alpha = Math.PI / 2 - 0.65;
    this.camera.beta = Math.PI / 3.4;
    this.camera.minZ = this.camera.radius / 1000;
    this.camera.maxZ = this.camera.radius * 20;
    this.camera.lowerRadiusLimit = this.camera.radius / 20;
    this.camera.upperRadiusLimit = this.camera.radius * 5;
    this.camera.panningSensibility = 1000 / this.camera.radius; // right-drag pans
  }
}
