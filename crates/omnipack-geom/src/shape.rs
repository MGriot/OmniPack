use parry3d_f64::math::{Isometry, Vector};
use parry3d_f64::na::{Matrix3, Rotation3, UnitQuaternion};
use parry3d_f64::shape::{Cuboid, Cylinder, SharedShape};
use serde::{Deserialize, Serialize};

/// An item or container shape in its local frame. The local frame has extents
/// `(w, h, d)` along `(x, y, z)`, centred on the origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Box { w: f64, h: f64, d: f64 },
    /// Axis along local Y. `w = d = 2 * radius`, `h = length`.
    Cylinder { radius: f64, length: f64 },
}

impl Shape {
    pub fn local_extents(&self) -> [f64; 3] {
        match *self {
            Shape::Box { w, h, d } => [w, h, d],
            Shape::Cylinder { radius, length } => [2.0 * radius, length, 2.0 * radius],
        }
    }

    pub fn volume(&self) -> f64 {
        match *self {
            Shape::Box { w, h, d } => w * h * d,
            Shape::Cylinder { radius, length } => std::f64::consts::PI * radius * radius * length,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.local_extents().iter().all(|v| v.is_finite() && *v > 0.0)
    }

    fn parry_shape(&self) -> SharedShape {
        match *self {
            Shape::Box { w, h, d } => SharedShape::new(Cuboid::new(Vector::new(w / 2.0, h / 2.0, d / 2.0))),
            Shape::Cylinder { radius, length } => SharedShape::new(Cylinder::new(length / 2.0, radius)),
        }
    }

    /// Orientations that give geometrically distinct placements for this shape.
    pub fn distinct_orientations(&self) -> Vec<Orientation> {
        match self {
            Shape::Box { .. } => Orientation::ALL.to_vec(),
            // Symmetric about local Y: only the choice of up-axis matters.
            Shape::Cylinder { .. } => vec![Orientation::Whd, Orientation::Hwd, Orientation::Wdh],
        }
    }
}

/// Which local axis ends up along world X, Y, Z. Named after the local extent
/// that ends up along each world axis, e.g. `Dhw` = local d along X, h along Y,
/// w along Z (a 90° yaw).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Orientation {
    Whd,
    Dhw,
    Hwd,
    Wdh,
    Hdw,
    Dwh,
}

impl Orientation {
    pub const ALL: [Orientation; 6] = [
        Orientation::Whd,
        Orientation::Dhw,
        Orientation::Hwd,
        Orientation::Wdh,
        Orientation::Hdw,
        Orientation::Dwh,
    ];

    /// Index of the local axis (0=x/w, 1=y/h, 2=z/d) mapped to world X, Y, Z.
    pub fn axis_map(self) -> [usize; 3] {
        match self {
            Orientation::Whd => [0, 1, 2],
            Orientation::Dhw => [2, 1, 0],
            Orientation::Hwd => [1, 0, 2],
            Orientation::Wdh => [0, 2, 1],
            Orientation::Hdw => [1, 2, 0],
            Orientation::Dwh => [2, 0, 1],
        }
    }

    /// True when the local height axis stays vertical ("this side up" compatible).
    pub fn keeps_upright(self) -> bool {
        self.axis_map()[1] == 1
    }

    /// Proper rotation (det = +1) taking local coordinates to world coordinates.
    pub fn rotation(self) -> UnitQuaternion<f64> {
        let map = self.axis_map();
        let mut m = Matrix3::zeros();
        for (world, &local) in map.iter().enumerate() {
            m[(world, local)] = 1.0;
        }
        if m.determinant() < 0.0 {
            // Flip one world axis: the shapes are symmetric, so this is the same body.
            for c in 0..3 {
                m[(2, c)] = -m[(2, c)];
            }
        }
        UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix_unchecked(m))
    }

    pub fn world_extents(self, local: [f64; 3]) -> [f64; 3] {
        let m = self.axis_map();
        [local[m[0]], local[m[1]], local[m[2]]]
    }
}

/// A shape fixed in one orientation, ready to be positioned in the container.
#[derive(Clone)]
pub struct OrientedShape {
    pub shape: Shape,
    pub orientation: Orientation,
    /// World-space AABB extents (w, h, d) in this orientation.
    pub extents: [f64; 3],
    /// Centre of mass relative to the AABB minimum corner, in world axes.
    pub com_from_min: [f64; 3],
    rotation: UnitQuaternion<f64>,
    parry: SharedShape,
}

impl std::fmt::Debug for OrientedShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrientedShape")
            .field("shape", &self.shape)
            .field("orientation", &self.orientation)
            .field("extents", &self.extents)
            .finish()
    }
}

impl OrientedShape {
    /// `com_offset` is the centre-of-mass offset from the geometric centre, in the
    /// shape's local frame (zero for uniform density).
    pub fn new(shape: &Shape, orientation: Orientation, com_offset: [f64; 3]) -> Self {
        let extents = orientation.world_extents(shape.local_extents());
        let rotation = orientation.rotation();
        let off = rotation * Vector::new(com_offset[0], com_offset[1], com_offset[2]);
        let com_from_min = [
            extents[0] / 2.0 + off.x,
            extents[1] / 2.0 + off.y,
            extents[2] / 2.0 + off.z,
        ];
        OrientedShape {
            shape: shape.clone(),
            orientation,
            extents,
            com_from_min,
            rotation,
            parry: shape.parry_shape(),
        }
    }

    pub fn parry(&self) -> &SharedShape {
        &self.parry
    }

    /// Isometry placing the shape so its AABB minimum corner is at `min`.
    pub fn isometry_at(&self, min: [f64; 3]) -> Isometry<f64> {
        let c = Vector::new(
            min[0] + self.extents[0] / 2.0,
            min[1] + self.extents[1] / 2.0,
            min[2] + self.extents[2] / 2.0,
        );
        Isometry::from_parts(c.into(), self.rotation)
    }

    /// Area of the footprint on the XZ plane (exact for the supported primitives).
    pub fn footprint_area(&self) -> f64 {
        match self.shape {
            Shape::Cylinder { radius, .. } if self.orientation.axis_map()[1] == 1 => {
                std::f64::consts::PI * radius * radius
            }
            _ => self.extents[0] * self.extents[2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_map_extents_consistently() {
        let s = Shape::Box { w: 10.0, h: 20.0, d: 30.0 };
        for o in Orientation::ALL {
            let os = OrientedShape::new(&s, o, [0.0; 3]);
            let aabb = os.parry().compute_aabb(&os.isometry_at([0.0; 3]));
            let e = aabb.extents();
            for k in 0..3 {
                assert!((e[k] - os.extents[k]).abs() < 1e-9, "{o:?} axis {k}: {} vs {}", e[k], os.extents[k]);
            }
            assert!((aabb.mins.coords.norm()) < 1e-9);
        }
    }

    #[test]
    fn cylinder_orientations() {
        let s = Shape::Cylinder { radius: 5.0, length: 40.0 };
        let up = OrientedShape::new(&s, Orientation::Whd, [0.0; 3]);
        assert_eq!(up.extents, [10.0, 40.0, 10.0]);
        let lying = OrientedShape::new(&s, Orientation::Hwd, [0.0; 3]);
        assert_eq!(lying.extents, [40.0, 10.0, 10.0]);
        let aabb = lying.parry().compute_aabb(&lying.isometry_at([0.0; 3]));
        assert!((aabb.extents().x - 40.0).abs() < 1e-9);
    }

    #[test]
    fn com_offset_rotates_with_item() {
        let s = Shape::Box { w: 10.0, h: 20.0, d: 30.0 };
        // Heavy end towards local +y (top).
        let os = OrientedShape::new(&s, Orientation::Hwd, [0.0, 5.0, 0.0]);
        // Local y now lies along world x.
        assert!((os.com_from_min[0].abs() - 15.0).abs() < 1e-9 || (os.com_from_min[0] - 5.0).abs() < 1e-9);
        assert!((os.com_from_min[1] - 5.0).abs() < 1e-9);
    }
}
