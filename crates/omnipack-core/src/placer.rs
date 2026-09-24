//! Constructive placer: fills one container with a sequence of units.
//!
//! For every unit it enumerates anchor positions on the floor plane and
//! orientations, lowers the item from above until it rests (so nothing
//! floats), ranks the candidates by the fill-bias score and accepts the best
//! one that passes the static checks: support area, centre of gravity inside
//! the support polygon with margin, and load limits of everything below after
//! propagating the new weight down to the floor.

use crate::grid::FloorGrid;
use crate::model::{ContainerSpec, PackOptions, Zone};
use crate::scene::{self, compute_supports};
use crate::statics::{add, effective_mass, load_at, resultant, sub, Load, Support, SupportSet};
use omnipack_geom::{drop_height, tol, Body, OrientedShape, Pt2};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// One physical unit to pack (an `ItemSpec` expanded by quantity).
#[derive(Debug, Clone)]
pub struct Instance {
    pub spec: usize,
    pub id: String,
    pub item_id: String,
    pub shapes: Vec<OrientedShape>,
    pub mass: f64,
    pub capacity: f64,
    pub floor_only: bool,
    pub stop: u32,
    pub zone: Zone,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct PlacedBody {
    pub inst: usize,
    pub orient: usize,
    pub min: [f64; 3],
    pub supports: SupportSet,
    pub chocked: bool,
    pub required_margin: f64,
    /// Load received from items above.
    pub incoming: Load,
    /// Load passed to each supporter (own weight + incoming).
    pub outgoing: Vec<(Support, Load)>,
    pub margin: f64,
}

pub struct ContainerState<'a> {
    pub spec: &'a ContainerSpec,
    opts: &'a PackOptions,
    instances: &'a [Instance],
    pub placed: Vec<PlacedBody>,
    grid: FloorGrid,
    anchors: Vec<[f64; 2]>,
    anchor_keys: HashSet<(i64, i64)>,
    mass: f64,
    moment_x: f64,
    scratch: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Candidate {
    key: f64,
    orient: usize,
    x: f64,
    z: f64,
    /// `None` until the drop height has been computed.
    y: Option<f64>,
}

impl PartialEq for Candidate {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Candidate {
    /// Min-heap on key; evaluated candidates first on ties so exact scores win.
    fn cmp(&self, o: &Self) -> Ordering {
        o.key
            .total_cmp(&self.key)
            .then_with(|| self.y.is_some().cmp(&o.y.is_some()))
            .then_with(|| o.orient.cmp(&self.orient))
            .then_with(|| o.x.total_cmp(&self.x))
            .then_with(|| o.z.total_cmp(&self.z))
    }
}

fn key(v: f64) -> i64 {
    (v * 1e4).round() as i64
}

impl<'a> ContainerState<'a> {
    pub fn new(spec: &'a ContainerSpec, opts: &'a PackOptions, instances: &'a [Instance]) -> Self {
        let min_dim = instances
            .iter()
            .flat_map(|i| i.shapes.iter())
            .map(|s| s.extents[0].min(s.extents[2]))
            .fold(f64::INFINITY, f64::min);
        let cell = if min_dim.is_finite() { min_dim.max(spec.width.max(spec.depth) / 256.0) } else { spec.width };
        let mut st = ContainerState {
            spec,
            opts,
            instances,
            placed: Vec::new(),
            grid: FloorGrid::new(spec.width, spec.depth, cell),
            anchors: Vec::new(),
            anchor_keys: HashSet::new(),
            mass: 0.0,
            moment_x: 0.0,
            scratch: Vec::new(),
        };
        for p in [[0.0, 0.0], [spec.width, 0.0], [0.0, spec.depth], [spec.width, spec.depth]] {
            st.add_anchor(p);
        }
        st
    }

    pub fn mass(&self) -> f64 {
        self.mass
    }

    pub fn body(&self, i: usize) -> Body<'a> {
        let p = &self.placed[i];
        Body::new(&self.instances[p.inst].shapes[p.orient], p.min)
    }

    fn add_anchor(&mut self, p: [f64; 2]) {
        if self.anchor_keys.insert((key(p[0]), key(p[1]))) {
            self.anchors.push(p);
        }
    }

    fn neighbours(&mut self, x0: f64, z0: f64, x1: f64, z1: f64) -> Vec<usize> {
        let mut ids = std::mem::take(&mut self.scratch);
        self.grid.query(x0, z0, x1, z1, &mut ids);
        let out = ids.iter().map(|&i| i as usize).collect();
        self.scratch = ids;
        out
    }

    fn score(&self, inst: &Instance, shape: &OrientedShape, x: f64, y: f64, z: f64) -> f64 {
        let c = self.spec;
        let [w, _, d] = shape.extents;
        let nz = match inst.zone {
            Zone::Front => (c.depth - (z + d)) / c.depth,
            _ => z / c.depth,
        };
        let [wx, wy, wz] = self.opts.bias.weights();
        let mut s = wx * x / c.width + wy * y / c.height + wz * nz;
        if self.opts.balance_weight > 0.0 {
            let cx = x + shape.com_from_min[0];
            let m = self.mass + inst.mass;
            let com_x = if m > 0.0 { (self.moment_x + inst.mass * cx) / m } else { x + w / 2.0 };
            s += self.opts.balance_weight * 1e-2 * (com_x - c.width / 2.0).abs() / c.width;
        }
        s
    }

    /// Lower bound of `score` for any `y ≥ 0`.
    fn score_lower_bound(&self, inst: &Instance, shape: &OrientedShape, x: f64, z: f64) -> f64 {
        let c = self.spec;
        let d = shape.extents[2];
        let nz = match inst.zone {
            Zone::Front => (c.depth - (z + d)) / c.depth,
            _ => z / c.depth,
        };
        let [wx, _, wz] = self.opts.bias.weights();
        wx * x / c.width + wz * nz
    }

    fn drop_y(&mut self, shape: &OrientedShape, x: f64, z: f64) -> f64 {
        let [w, _, d] = shape.extents;
        let ids = self.neighbours(x, z, x + w, z + d);
        let bodies: Vec<Body> = ids.into_iter().map(|i| self.body(i)).collect();
        drop_height(shape, x, z, bodies)
    }

    /// Tries to place instance `inst`. Returns `true` on success.
    pub fn try_place(&mut self, inst_idx: usize) -> bool {
        let inst = &self.instances[inst_idx];
        if let Some(max) = self.spec.max_payload {
            if self.mass + inst.mass > max + 1e-9 {
                return false;
            }
        }
        let (cw, ch, cd) = (self.spec.width, self.spec.height, self.spec.depth);
        let mut heap = BinaryHeap::new();
        let mut seen = HashSet::new();
        for (oi, shape) in inst.shapes.iter().enumerate() {
            let [w, h, d] = shape.extents;
            if w > cw + tol::BOUNDS || h > ch + tol::BOUNDS || d > cd + tol::BOUNDS {
                continue;
            }
            for &[px, pz] in &self.anchors {
                for (x, z) in [(px, pz), (px - w, pz), (px, pz - d), (px - w, pz - d)] {
                    if x < -tol::BOUNDS || z < -tol::BOUNDS || x + w > cw + tol::BOUNDS || z + d > cd + tol::BOUNDS {
                        continue;
                    }
                    let (x, z) = (x.clamp(0.0, (cw - w).max(0.0)), z.clamp(0.0, (cd - d).max(0.0)));
                    if !seen.insert((oi, key(x), key(z))) {
                        continue;
                    }
                    let lb = self.score_lower_bound(inst, shape, x, z);
                    heap.push(Candidate { key: lb, orient: oi, x, z, y: None });
                }
            }
        }
        let mut checks = 0;
        while let Some(c) = heap.pop() {
            let shape = &self.instances[inst_idx].shapes[c.orient];
            match c.y {
                None => {
                    let y = self.drop_y(shape, c.x, c.z);
                    if y + shape.extents[1] > ch + tol::BOUNDS {
                        continue;
                    }
                    if inst.floor_only && y > tol::CONTACT {
                        continue;
                    }
                    let s = self.score(inst, shape, c.x, y, c.z);
                    heap.push(Candidate { key: s, y: Some(y), ..c });
                }
                Some(y) => {
                    checks += 1;
                    if checks > self.opts.max_stability_checks {
                        return false;
                    }
                    if let Some(commit) = self.check(inst_idx, c.orient, [c.x, y, c.z]) {
                        self.commit(commit);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Full static check of a tentative placement. Returns the state changes
    /// to apply if it is acceptable.
    fn check(&mut self, inst_idx: usize, orient: usize, min: [f64; 3]) -> Option<Commit> {
        let inst = &self.instances[inst_idx];
        let shape = &inst.shapes[orient];
        let body = Body::new(shape, min);
        let mx = body.max();
        let ids = self.neighbours(min[0] - tol::CONTACT, min[2] - tol::CONTACT, mx[0] + tol::CONTACT, mx[2] + tol::CONTACT);
        let info = compute_supports(&body, ids.iter().map(|&j| (j, self.body(j))));
        if info.set.is_empty() {
            return None;
        }
        // Nothing may stand on a zero-capacity (fragile) item.
        for owner in &info.set.owners {
            if let Support::Item(j) = owner {
                if self.instances[self.placed[*j].inst].capacity <= 0.0 {
                    return None;
                }
            }
        }
        if let Some(bottom) = scene::flat_bottom_area(&body) {
            if info.contact_area < self.opts.min_support_ratio * bottom - 1e-9 {
                return None;
            }
        }
        let chocked = scene::is_lying_cylinder(&body)
            && info.set.is_degenerate()
            && scene::roll_blocked(&body, ids.iter().map(|&j| self.body(j)), [self.spec.width, self.spec.height, self.spec.depth]);
        let required = scene::required_margin(&body, self.opts.stability_margin);
        let com = body.com();
        let own = load_at(effective_mass(inst.mass), Pt2::new(com[0], com[2]));
        let margin = scene::stability_margin(&info.set, Pt2::new(com[0], com[2]), chocked);
        if margin < required - 1e-9 {
            return None;
        }
        let outgoing = info.set.distribute(own)?;

        // Propagate the new weight downwards, top to bottom.
        let mut delta: HashMap<usize, Load> = HashMap::new();
        let mut queue: BinaryHeap<(OrdF64, usize)> = BinaryHeap::new();
        for (s, l) in &outgoing {
            if let Support::Item(j) = *s {
                let e = delta.entry(j).or_insert([0.0; 3]);
                *e = add(*e, *l);
                queue.push((OrdF64(self.placed[j].min[1]), j));
            }
        }
        let mut updates: Vec<Update> = Vec::new();
        let mut done: HashSet<usize> = HashSet::new();
        while let Some((_, j)) = queue.pop() {
            if !done.insert(j) {
                continue;
            }
            let pj = &self.placed[j];
            let ij = &self.instances[pj.inst];
            let incoming = add(pj.incoming, delta[&j]);
            if incoming[0] > ij.capacity + 1e-6 {
                return None;
            }
            let bj = self.body(j);
            let cj = bj.com();
            let total = add(load_at(effective_mass(ij.mass), Pt2::new(cj[0], cj[2])), incoming);
            let r = resultant(total)?;
            let mj = scene::stability_margin(&pj.supports, r, pj.chocked);
            if mj < pj.required_margin - 1e-9 {
                return None;
            }
            let new_out = pj.supports.distribute(total)?;
            for (s, l) in &new_out {
                if let Support::Item(k) = *s {
                    let old = pj.outgoing.iter().find(|(o, _)| o == s).map(|x| x.1).unwrap_or([0.0; 3]);
                    let e = delta.entry(k).or_insert([0.0; 3]);
                    *e = add(*e, sub(*l, old));
                    queue.push((OrdF64(self.placed[k].min[1]), k));
                }
            }
            for (s, old) in &pj.outgoing {
                if let Support::Item(k) = *s {
                    if !new_out.iter().any(|(o, _)| o == s) {
                        let e = delta.entry(k).or_insert([0.0; 3]);
                        *e = sub(*e, *old);
                        queue.push((OrdF64(self.placed[k].min[1]), k));
                    }
                }
            }
            updates.push(Update { idx: j, incoming, outgoing: new_out, margin: mj });
        }
        Some(Commit {
            body: PlacedBody {
                inst: inst_idx,
                orient,
                min,
                supports: info.set,
                chocked,
                required_margin: required,
                incoming: [0.0; 3],
                outgoing,
                margin,
            },
            updates,
        })
    }

    fn commit(&mut self, c: Commit) {
        for u in c.updates {
            let p = &mut self.placed[u.idx];
            p.incoming = u.incoming;
            p.outgoing = u.outgoing;
            p.margin = u.margin;
        }
        let id = self.placed.len();
        let b = c.body;
        let inst = &self.instances[b.inst];
        let shape = &inst.shapes[b.orient];
        let [w, _, d] = shape.extents;
        let (x0, z0, x1, z1) = (b.min[0], b.min[2], b.min[0] + w, b.min[2] + d);
        self.grid.insert(id as u32, x0, z0, x1, z1);
        self.mass += inst.mass;
        self.moment_x += inst.mass * (b.min[0] + shape.com_from_min[0]);
        self.placed.push(b);
        let (cw, cd) = (self.spec.width, self.spec.depth);
        for p in [
            [x1, z0], [x0, z1], [x1, z1], [x0, z0],
            [0.0, z0], [0.0, z1], [x0, 0.0], [x1, 0.0],
            [cw, z0], [cw, z1], [x0, cd], [x1, cd],
        ] {
            self.add_anchor(p);
        }
    }
}

struct Update {
    idx: usize,
    incoming: Load,
    outgoing: Vec<(Support, Load)>,
    margin: f64,
}

struct Commit {
    body: PlacedBody,
    updates: Vec<Update>,
}

struct OrdF64(f64);
impl PartialEq for OrdF64 {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for OrdF64 {}
impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for OrdF64 {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}
