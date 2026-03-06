use pyo3::prelude::*;
use rstar::{RTree, AABB, RTreeObject};

#[pyclass]
#[derive(Clone, Copy, Debug)]
pub struct Rect3D {
    pub x1: f64,
    pub y1: f64,
    pub z1: f64,
    pub x2: f64,
    pub y2: f64,
    pub z2: f64,
}

#[pymethods]
impl Rect3D {
    #[new]
    pub fn new(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> Self {
        Rect3D { x1, y1, z1, x2, y2, z2 }
    }
}

impl RTreeObject for Rect3D {
    type Envelope = AABB<[f64; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners([self.x1, self.y1, self.z1], [self.x2, self.y2, self.z2])
    }
}

#[pyclass]
pub struct RTreeManager {
    tree: RTree<Rect3D>,
}

#[pymethods]
impl RTreeManager {
    #[new]
    pub fn new() -> Self {
        RTreeManager {
            tree: RTree::new(),
        }
    }

    pub fn insert(&mut self, rect: Rect3D) {
        self.tree.insert(rect);
    }

    pub fn intersects(&self, rect: Rect3D) -> bool {
        let envelope = rect.envelope();
        self.tree.locate_in_envelope_intersecting(&envelope).next().is_some()
    }

    pub fn size(&self) -> usize {
        self.tree.size()
    }

    pub fn clear(&mut self) {
        self.tree = RTree::new();
    }
}

#[pyfunction]
fn hello_rust() -> PyResult<String> {
    Ok("OmniPack Rust Core Online with R-Tree Support".to_string())
}

#[pymodule]
fn omnipack_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_rust, m)?)?;
    m.add_class::<Rect3D>()?;
    m.add_class::<RTreeManager>()?;
    Ok(())
}
