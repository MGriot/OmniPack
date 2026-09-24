// Babylon.js view of one container plan. Engine coordinates map 1:1 to
// Babylon's (Y up); positions are AABB minimum corners, so meshes are
// centred at position + size / 2.

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
} from "@babylonjs/core";
import { AXIS_MAP, type ContainerPlan, type Placement } from "./types";

export class PlanViewer {
  private engine: Engine;
  private scene: Scene;
  private camera: ArcRotateCamera;
  private meshes: { placement: Placement; mesh: Mesh }[] = [];
  private staticMeshes: (Mesh | LinesMesh)[] = [];
  private materials = new Map<string, StandardMaterial>();
  private highlighted: string | null = null;
  private pickHandler: (p: Placement | null) => void = () => {};
  private size: [number, number, number] = [1, 1, 1];

  constructor(canvas: HTMLCanvasElement) {
    this.engine = new Engine(canvas, true, { preserveDrawingBuffer: false, stencil: true });
    this.scene = new Scene(this.engine);
    this.scene.clearColor = new Color4(0.07, 0.08, 0.1, 1);
    this.camera = new ArcRotateCamera("cam", -Math.PI / 3, Math.PI / 3, 10, Vector3.Zero(), this.scene);
    this.camera.attachControl(canvas, true);
    this.camera.wheelDeltaPercentage = 0.02;
    this.camera.useFramingBehavior = false;
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
      this.materials.set(hex, m);
    }
    return m;
  }

  clear() {
    for (const { mesh } of this.meshes) mesh.dispose();
    for (const m of this.staticMeshes) m.dispose();
    this.meshes = [];
    this.staticMeshes = [];
  }

  show(plan: ContainerPlan, colorOf: (p: Placement) => string, cog: [number, number, number] | null) {
    this.clear();
    const [W, H, D] = plan.size;
    const sizeChanged = W !== this.size[0] || H !== this.size[1] || D !== this.size[2];
    this.size = plan.size;
    this.drawContainer(W, H, D);

    for (const p of plan.placements) {
      const mesh = this.buildMesh(p);
      mesh.material = this.material(colorOf(p));
      mesh.enableEdgesRendering();
      mesh.edgesWidth = 3;
      mesh.edgesColor = new Color4(0, 0, 0, 0.55);
      this.meshes.push({ placement: p, mesh });
    }
    if (cog) this.drawCog(cog);
    if (sizeChanged) this.resetCamera();
    this.applyHighlight();
  }

  private buildMesh(p: Placement): Mesh {
    const [x, y, z] = p.position;
    const [w, h, d] = p.size;
    let mesh: Mesh;
    if (p.shape.kind === "box") {
      mesh = MeshBuilder.CreateBox(p.instance_id, { width: w, height: h, depth: d }, this.scene);
    } else {
      mesh = MeshBuilder.CreateCylinder(
        p.instance_id,
        { diameter: 2 * p.shape.radius, height: p.shape.length, tessellation: 32 },
        this.scene,
      );
      // Local cylinder axis is Y; find which world axis it lies on.
      const axis = AXIS_MAP[p.orientation].indexOf(1);
      if (axis === 0) mesh.rotation.z = Math.PI / 2;
      if (axis === 2) mesh.rotation.x = Math.PI / 2;
    }
    mesh.position = new Vector3(x + w / 2, y + h / 2, z + d / 2);
    return mesh;
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
    const door = MeshBuilder.CreateLines(
      "door",
      { points: [c(0, 0, D), c(W, 0, D), c(W, H, D), c(0, H, D), c(0, 0, D)] },
      this.scene,
    );
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

  /** Shows only placements with `seq < count` (step-by-step loading). */
  setVisibleCount(count: number) {
    for (const { placement, mesh } of this.meshes) mesh.setEnabled(placement.seq < count);
  }

  highlight(instanceId: string | null) {
    this.highlighted = instanceId;
    this.applyHighlight();
  }

  private applyHighlight() {
    for (const { placement, mesh } of this.meshes) {
      mesh.renderOverlay = placement.instance_id === this.highlighted;
      mesh.overlayColor = new Color3(1, 1, 1);
      mesh.overlayAlpha = 0.45;
    }
  }

  resetCamera() {
    const [W, H, D] = this.size;
    // Look in through the door (z = depth), from above and slightly to the side.
    this.camera.target = new Vector3(W / 2, H / 3, D / 2);
    this.camera.radius = Math.hypot(W, H, D) * 1.6;
    this.camera.alpha = Math.PI / 2 - 0.65;
    this.camera.beta = Math.PI / 3.4;
    this.camera.minZ = this.camera.radius / 1000;
    this.camera.maxZ = this.camera.radius * 20;
    this.camera.lowerRadiusLimit = this.camera.radius / 20;
    this.camera.upperRadiusLimit = this.camera.radius * 5;
    this.camera.panningSensibility = 1000 / this.camera.radius; // right-drag pans ~1 px per mm-scale unit
  }
}
