//! Uniform 2D grid over the container floor (XZ) for footprint queries.

pub struct FloorGrid {
    cell: f64,
    nx: usize,
    nz: usize,
    cells: Vec<Vec<u32>>,
    stamp: Vec<u32>,
    epoch: u32,
}

impl FloorGrid {
    pub fn new(width: f64, depth: f64, cell: f64) -> Self {
        let cell = cell.max(1e-3);
        let nx = ((width / cell).ceil() as usize).max(1);
        let nz = ((depth / cell).ceil() as usize).max(1);
        FloorGrid { cell, nx, nz, cells: vec![Vec::new(); nx * nz], stamp: Vec::new(), epoch: 0 }
    }

    fn range(&self, x0: f64, z0: f64, x1: f64, z1: f64) -> (usize, usize, usize, usize) {
        let clamp_x = |v: f64| ((v / self.cell).floor().max(0.0) as usize).min(self.nx - 1);
        let clamp_z = |v: f64| ((v / self.cell).floor().max(0.0) as usize).min(self.nz - 1);
        (clamp_x(x0), clamp_z(z0), clamp_x(x1), clamp_z(z1))
    }

    pub fn insert(&mut self, id: u32, x0: f64, z0: f64, x1: f64, z1: f64) {
        let (i0, k0, i1, k1) = self.range(x0, z0, x1, z1);
        for k in k0..=k1 {
            for i in i0..=i1 {
                self.cells[k * self.nx + i].push(id);
            }
        }
        if self.stamp.len() <= id as usize {
            self.stamp.resize(id as usize + 1, 0);
        }
    }

    /// Ids whose inserted rectangle may touch `[x0,x1] × [z0,z1]` (superset).
    pub fn query(&mut self, x0: f64, z0: f64, x1: f64, z1: f64, out: &mut Vec<u32>) {
        out.clear();
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.epoch = 1;
        }
        let (i0, k0, i1, k1) = self.range(x0, z0, x1, z1);
        for k in k0..=k1 {
            for i in i0..=i1 {
                for &id in &self.cells[k * self.nx + i] {
                    let s = &mut self.stamp[id as usize];
                    if *s != self.epoch {
                        *s = self.epoch;
                        out.push(id);
                    }
                }
            }
        }
    }
}
