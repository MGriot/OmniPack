use crate::hull2d::{convex_hull, polygon_area, Pt2};
use parry3d_f64::math::{Isometry, Point, Vector};
use parry3d_f64::na::{Matrix3, Rotation3, UnitQuaternion};
use parry3d_f64::shape::{Cuboid, Cylinder, SharedShape};
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

/// Segments used to tessellate round shapes (rendering and flat-face detection).
/// Inscribed polygons, so contact areas are slightly under-estimated, which is
/// the conservative direction for stability.
const ROUND_SEGMENTS: u32 = 32;

/// Vertices and triangles of one convex part.
type PartMesh = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// An item shape in its local frame. The local axis-aligned bounding box has
/// extents `(w, h, d)` along `(x, y, z)` and is centred on the origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Box {
        w: f64,
        h: f64,
        d: f64,
    },
    /// Axis along local Y.
    Cylinder {
        radius: f64,
        length: f64,
    },
    Sphere {
        radius: f64,
    },
    /// Base down, apex up (local Y).
    Cone {
        radius: f64,
        height: f64,
    },
    /// Rectangular base `w × d`, apex up.
    Pyramid {
        w: f64,
        d: f64,
        height: f64,
    },
    /// Regular `sides`-gon cross-section (circumradius `radius`), axis along
    /// local Y, with one flat side facing local −Z so it can lie on a face.
    Prism {
        sides: u32,
        radius: f64,
        length: f64,
    },
    /// Angle profile: horizontal leg `a` (local X) and vertical leg `b`
    /// (local Y), both `thickness` thick, extruded `length` along local Z.
    LProfile {
        a: f64,
        b: f64,
        thickness: f64,
        length: f64,
    },
}

fn prism_polygon(sides: u32, radius: f64) -> Vec<(f64, f64)> {
    let n = sides.max(3) as f64;
    (0..sides.max(3))
        .map(|k| {
            let t = -PI / 2.0 + PI / n + TAU * k as f64 / n;
            (radius * t.cos(), radius * t.sin())
        })
        .collect()
}

fn bounds_2d(pts: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    pts.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(a, b, c, d), &(x, z)| (a.min(x), b.min(z), c.max(x), d.max(z)),
    )
}

impl Shape {
    pub fn local_extents(&self) -> [f64; 3] {
        match *self {
            Shape::Box { w, h, d } => [w, h, d],
            Shape::Cylinder { radius, length } => [2.0 * radius, length, 2.0 * radius],
            Shape::Sphere { radius } => [2.0 * radius; 3],
            Shape::Cone { radius, height } => [2.0 * radius, height, 2.0 * radius],
            Shape::Pyramid { w, d, height } => [w, height, d],
            Shape::Prism {
                sides,
                radius,
                length,
            } => {
                let (x0, z0, x1, z1) = bounds_2d(&prism_polygon(sides, radius));
                [x1 - x0, length, z1 - z0]
            }
            Shape::LProfile { a, b, length, .. } => [a, b, length],
        }
    }

    pub fn is_valid(&self) -> bool {
        let dims_ok = self
            .local_extents()
            .iter()
            .all(|v| v.is_finite() && *v > 0.0);
        dims_ok
            && match *self {
                Shape::Prism { sides, .. } => (3..=64).contains(&sides),
                Shape::LProfile {
                    a, b, thickness, ..
                } => thickness > 0.0 && thickness < a && thickness < b,
                _ => true,
            }
    }

    /// Exact volume, mm³.
    pub fn volume(&self) -> f64 {
        match *self {
            Shape::Box { w, h, d } => w * h * d,
            Shape::Cylinder { radius, length } => PI * radius * radius * length,
            Shape::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
            Shape::Cone { radius, height } => PI * radius * radius * height / 3.0,
            Shape::Pyramid { w, d, height } => w * d * height / 3.0,
            Shape::Prism {
                sides,
                radius,
                length,
            } => {
                let n = sides.max(3) as f64;
                0.5 * n * radius * radius * (TAU / n).sin() * length
            }
            Shape::LProfile {
                a,
                b,
                thickness: t,
                length,
            } => (a * t + (b - t) * t) * length,
        }
    }

    /// Convex parts as `(offset from AABB centre, shape)`.
    fn parts(&self) -> Vec<(Vector<f64>, SharedShape)> {
        let one = |s: SharedShape| vec![(Vector::zeros(), s)];
        match *self {
            Shape::Box { w, h, d } => one(SharedShape::new(Cuboid::new(Vector::new(
                w / 2.0,
                h / 2.0,
                d / 2.0,
            )))),
            Shape::Cylinder { radius, length } => {
                one(SharedShape::new(Cylinder::new(length / 2.0, radius)))
            }
            Shape::Sphere { radius } => one(SharedShape::ball(radius)),
            Shape::Cone { radius, height } => one(SharedShape::cone(height / 2.0, radius)),
            Shape::Pyramid { w, d, height } => {
                let (hw, hh, hd) = (w / 2.0, height / 2.0, d / 2.0);
                let pts = [
                    Point::new(-hw, -hh, -hd),
                    Point::new(hw, -hh, -hd),
                    Point::new(hw, -hh, hd),
                    Point::new(-hw, -hh, hd),
                    Point::new(0.0, hh, 0.0),
                ];
                one(SharedShape::convex_hull(&pts).expect("pyramid hull"))
            }
            Shape::Prism {
                sides,
                radius,
                length,
            } => {
                let poly = prism_polygon(sides, radius);
                let (x0, z0, x1, z1) = bounds_2d(&poly);
                let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
                let pts: Vec<Point<f64>> = poly
                    .iter()
                    .flat_map(|&(x, z)| {
                        [
                            Point::new(x - cx, -length / 2.0, z - cz),
                            Point::new(x - cx, length / 2.0, z - cz),
                        ]
                    })
                    .collect();
                one(SharedShape::convex_hull(&pts).expect("prism hull"))
            }
            Shape::LProfile {
                a,
                b,
                thickness: t,
                length,
            } => {
                let leg_a =
                    SharedShape::new(Cuboid::new(Vector::new(a / 2.0, t / 2.0, length / 2.0)));
                let leg_b = SharedShape::new(Cuboid::new(Vector::new(
                    t / 2.0,
                    (b - t) / 2.0,
                    length / 2.0,
                )));
                vec![
                    (Vector::new(0.0, -b / 2.0 + t / 2.0, 0.0), leg_a),
                    (Vector::new(-a / 2.0 + t / 2.0, t / 2.0, 0.0), leg_b),
                ]
            }
        }
    }

    fn parry_shape(&self) -> SharedShape {
        let mut parts = self.parts();
        if parts.len() == 1 {
            return parts.pop().unwrap().1;
        }
        SharedShape::compound(
            parts
                .into_iter()
                .map(|(o, s)| (Isometry::translation(o.x, o.y, o.z), s))
                .collect(),
        )
    }

    /// Centre of mass in the local frame (uniform density).
    pub fn local_com(&self) -> [f64; 3] {
        let c = self.parry_shape().mass_properties(1.0).local_com;
        [c.x, c.y, c.z]
    }

    /// Triangle mesh per convex part, local frame (for rendering and face detection).
    fn part_meshes(&self) -> Vec<PartMesh> {
        self.parts()
            .into_iter()
            .map(|(off, s)| {
                let (pts, idx) = if let Some(c) = s.as_cuboid() {
                    c.to_trimesh()
                } else if let Some(c) = s.as_cylinder() {
                    c.to_trimesh(ROUND_SEGMENTS)
                } else if let Some(b) = s.as_ball() {
                    b.to_trimesh(ROUND_SEGMENTS / 2, ROUND_SEGMENTS)
                } else if let Some(c) = s.as_cone() {
                    c.to_trimesh(ROUND_SEGMENTS)
                } else if let Some(p) = s.as_convex_polyhedron() {
                    p.to_trimesh()
                } else {
                    (Vec::new(), Vec::new())
                };
                (
                    pts.iter()
                        .map(|p| [p.x + off.x, p.y + off.y, p.z + off.z])
                        .collect(),
                    idx,
                )
            })
            .collect()
    }

    /// Orientations that give distinct, physically meaningful resting poses.
    pub fn distinct_orientations(&self) -> Vec<Orientation> {
        use Orientation::*;
        match self {
            Shape::Box { .. } | Shape::LProfile { .. } => Orientation::ALL.to_vec(),
            // Symmetric about local Y: only the choice of up-axis matters.
            Shape::Cylinder { .. } => vec![Whd, Hwd, Wdh],
            Shape::Sphere { .. } | Shape::Cone { .. } => vec![Whd],
            Shape::Pyramid { .. } => vec![Whd, Dhw],
            // Upright, or lying on the flat side (local −Z turned downwards).
            Shape::Prism { .. } => vec![Whd, Dhw, Wdh, Hdw],
        }
    }

    /// Items that roll when resting on a point or a line.
    pub fn can_roll(&self) -> bool {
        matches!(self, Shape::Cylinder { .. } | Shape::Sphere { .. })
    }

    /// Short human-readable name.
    pub fn label(&self) -> &'static str {
        match self {
            Shape::Box { .. } => "box",
            Shape::Cylinder { .. } => "cylinder",
            Shape::Sphere { .. } => "sphere",
            Shape::Cone { .. } => "cone",
            Shape::Pyramid { .. } => "pyramid",
            Shape::Prism { .. } => "prism",
            Shape::LProfile { .. } => "L-profile",
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
    /// The local axis mapped to world Y keeps its sign, so "down" stays down.
    pub fn rotation(self) -> UnitQuaternion<f64> {
        let map = self.axis_map();
        let mut m = Matrix3::zeros();
        for (world, &local) in map.iter().enumerate() {
            m[(world, local)] = 1.0;
        }
        if m.determinant() < 0.0 {
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

/// Render mesh in world axes, centred on the AABB centre.
#[derive(Debug, Clone, Serialize)]
pub struct RenderMesh {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
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
    /// Flat bottom / top face as XZ offsets from the AABB centre (convex, CCW).
    pub bottom_face: Option<Vec<Pt2>>,
    pub top_face: Option<Vec<Pt2>>,
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

/// The flat face at the bottom (or top) of the rotated shape, if one convex
/// part alone forms it with at least three vertices.
fn flat_face(parts: &[Vec<[f64; 3]>], top: bool) -> Option<Vec<Pt2>> {
    let key = |p: &[f64; 3]| if top { p[1] } else { -p[1] };
    let extreme = parts.iter().flatten().map(key).fold(f64::MIN, f64::max);
    let scale = parts
        .iter()
        .flatten()
        .map(|p| p[0].abs().max(p[1].abs()).max(p[2].abs()))
        .fold(1.0, f64::max);
    let eps = 1e-7 * scale;
    let touching: Vec<&Vec<[f64; 3]>> = parts
        .iter()
        .filter(|part| part.iter().any(|p| key(p) >= extreme - eps))
        .collect();
    if touching.len() != 1 {
        return None;
    }
    let pts: Vec<Pt2> = touching[0]
        .iter()
        .filter(|p| key(p) >= extreme - eps)
        .map(|p| Pt2::new(p[0], p[2]))
        .collect();
    let hull = convex_hull(&pts);
    (hull.len() >= 3 && polygon_area(&hull) > eps * eps).then_some(hull)
}

impl OrientedShape {
    /// `com_offset` is the centre-of-mass offset from the shape's uniform-density
    /// centre of mass, in the shape's local frame.
    pub fn new(shape: &Shape, orientation: Orientation, com_offset: [f64; 3]) -> Self {
        let extents = orientation.world_extents(shape.local_extents());
        let rotation = orientation.rotation();
        let lc = shape.local_com();
        let c = rotation
            * Vector::new(
                lc[0] + com_offset[0],
                lc[1] + com_offset[1],
                lc[2] + com_offset[2],
            );
        let com_from_min = [
            extents[0] / 2.0 + c.x,
            extents[1] / 2.0 + c.y,
            extents[2] / 2.0 + c.z,
        ];
        let world_parts: Vec<Vec<[f64; 3]>> = shape
            .part_meshes()
            .into_iter()
            .map(|(pts, _)| {
                pts.iter()
                    .map(|p| {
                        let v = rotation * Vector::new(p[0], p[1], p[2]);
                        [v.x, v.y, v.z]
                    })
                    .collect()
            })
            .collect();
        OrientedShape {
            shape: shape.clone(),
            orientation,
            extents,
            com_from_min,
            bottom_face: flat_face(&world_parts, false),
            top_face: flat_face(&world_parts, true),
            rotation,
            parry: shape.parry_shape(),
        }
    }

    pub fn parry(&self) -> &SharedShape {
        &self.parry
    }

    pub fn rotation(&self) -> UnitQuaternion<f64> {
        self.rotation
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

    /// Area of the flat bottom face, if any.
    pub fn bottom_area(&self) -> Option<f64> {
        self.bottom_face.as_ref().map(|f| polygon_area(f))
    }

    /// Triangle mesh in world axes around the AABB centre, unwelded (one
    /// vertex per triangle corner) except for spheres, so flat faces shade flat.
    pub fn render_mesh(&self) -> RenderMesh {
        let smooth = matches!(self.shape, Shape::Sphere { .. });
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for (pts, tris) in self.shape.part_meshes() {
            let world: Vec<[f32; 3]> = pts
                .iter()
                .map(|p| {
                    let v = self.rotation * Vector::new(p[0], p[1], p[2]);
                    [v.x as f32, v.y as f32, v.z as f32]
                })
                .collect();
            let base = (positions.len() / 3) as u32;
            if smooth {
                positions.extend(world.iter().flatten());
                indices.extend(tris.iter().flat_map(|t| t.map(|i| base + i)));
            } else {
                for t in &tris {
                    for &i in t {
                        indices.push((positions.len() / 3) as u32);
                        positions.extend(world[i as usize]);
                    }
                }
            }
        }
        RenderMesh { positions, indices }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_shapes() -> Vec<Shape> {
        vec![
            Shape::Box {
                w: 10.0,
                h: 20.0,
                d: 30.0,
            },
            Shape::Cylinder {
                radius: 5.0,
                length: 40.0,
            },
            Shape::Sphere { radius: 7.0 },
            Shape::Cone {
                radius: 6.0,
                height: 20.0,
            },
            Shape::Pyramid {
                w: 10.0,
                d: 14.0,
                height: 12.0,
            },
            Shape::Prism {
                sides: 3,
                radius: 10.0,
                length: 30.0,
            },
            Shape::Prism {
                sides: 6,
                radius: 10.0,
                length: 30.0,
            },
            Shape::LProfile {
                a: 20.0,
                b: 15.0,
                thickness: 3.0,
                length: 50.0,
            },
        ]
    }

    #[test]
    fn aabb_matches_extents_in_every_orientation() {
        for s in all_shapes() {
            for o in Orientation::ALL {
                let os = OrientedShape::new(&s, o, [0.0; 3]);
                let aabb = os.parry().compute_aabb(&os.isometry_at([0.0; 3]));
                let e = aabb.extents();
                for k in 0..3 {
                    assert!(
                        (e[k] - os.extents[k]).abs() < 1e-6,
                        "{s:?} {o:?} axis {k}: {} vs {}",
                        e[k],
                        os.extents[k]
                    );
                }
                assert!(
                    aabb.mins.coords.norm() < 1e-6,
                    "{s:?} {o:?}: AABB not centred ({:?})",
                    aabb.mins
                );
            }
        }
    }

    #[test]
    fn volumes_match_parry_mass_properties() {
        for s in all_shapes() {
            let v = s.parry_shape().mass_properties(1.0).mass();
            assert!(
                (v - s.volume()).abs() / s.volume() < 1e-6,
                "{s:?}: {v} vs {}",
                s.volume()
            );
        }
    }

    #[test]
    fn stable_orientations_have_flat_bottoms() {
        for s in all_shapes() {
            for o in s.distinct_orientations() {
                let os = OrientedShape::new(&s, o, [0.0; 3]);
                let rolls = s.can_roll() && os.bottom_face.is_none();
                let l_profile = matches!(s, Shape::LProfile { .. });
                assert!(
                    rolls || l_profile || os.bottom_face.is_some(),
                    "{s:?} {o:?} has no flat bottom"
                );
            }
        }
    }

    #[test]
    fn cone_centre_of_mass_is_a_quarter_up() {
        let os = OrientedShape::new(
            &Shape::Cone {
                radius: 5.0,
                height: 20.0,
            },
            Orientation::Whd,
            [0.0; 3],
        );
        assert!(
            (os.com_from_min[1] - 5.0).abs() < 1e-6,
            "{:?}",
            os.com_from_min
        );
    }

    #[test]
    fn cylinder_orientations() {
        let s = Shape::Cylinder {
            radius: 5.0,
            length: 40.0,
        };
        assert_eq!(
            OrientedShape::new(&s, Orientation::Whd, [0.0; 3]).extents,
            [10.0, 40.0, 10.0]
        );
        assert_eq!(
            OrientedShape::new(&s, Orientation::Hwd, [0.0; 3]).extents,
            [40.0, 10.0, 10.0]
        );
    }

    #[test]
    fn com_offset_rotates_with_item() {
        let s = Shape::Box {
            w: 10.0,
            h: 20.0,
            d: 30.0,
        };
        let os = OrientedShape::new(&s, Orientation::Hwd, [0.0, 5.0, 0.0]);
        assert!(
            (os.com_from_min[0] - 15.0).abs() < 1e-9 || (os.com_from_min[0] - 5.0).abs() < 1e-9
        );
        assert!((os.com_from_min[1] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn render_mesh_is_well_formed() {
        for s in all_shapes() {
            let m = OrientedShape::new(&s, Orientation::Whd, [0.0; 3]).render_mesh();
            assert!(
                !m.indices.is_empty() && m.indices.len().is_multiple_of(3),
                "{s:?}"
            );
            assert!(
                m.indices
                    .iter()
                    .all(|&i| (i as usize) < m.positions.len() / 3),
                "{s:?}"
            );
        }
    }
}
