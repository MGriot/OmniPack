//! Exact geometric queries between placed bodies.

use crate::hull2d::{clip_convex, disk, polygon_area, rect, Pt2};
use crate::shape::{OrientedShape, Shape};
use crate::tol;
use parry3d_f64::math::Vector;
use parry3d_f64::query::{self, ContactManifold, DefaultQueryDispatcher, PersistentQueryDispatcher, ShapeCastOptions};

/// Segments used to approximate a circular face. Inscribed, so contact areas
/// are slightly under-estimated (the conservative direction for stability).
const DISK_SEGMENTS: usize = 32;

/// An oriented shape placed with its AABB minimum corner at `min`.
#[derive(Debug, Clone, Copy)]
pub struct Body<'a> {
    pub shape: &'a OrientedShape,
    pub min: [f64; 3],
}

impl<'a> Body<'a> {
    pub fn new(shape: &'a OrientedShape, min: [f64; 3]) -> Self {
        Body { shape, min }
    }

    pub fn max(&self) -> [f64; 3] {
        let e = self.shape.extents;
        [self.min[0] + e[0], self.min[1] + e[1], self.min[2] + e[2]]
    }

    pub fn com(&self) -> [f64; 3] {
        let c = self.shape.com_from_min;
        [self.min[0] + c[0], self.min[1] + c[1], self.min[2] + c[2]]
    }

    fn is_box(&self) -> bool {
        matches!(self.shape.shape, Shape::Box { .. })
    }

    fn vertical_axis_is_local_y(&self) -> bool {
        self.shape.orientation.axis_map()[1] == 1
    }

    /// The flat horizontal face at the bottom or top, if the body has one.
    fn flat_face(&self, top: bool) -> Option<(f64, Vec<Pt2>)> {
        let (mn, mx) = (self.min, self.max());
        let y = if top { mx[1] } else { mn[1] };
        match self.shape.shape {
            Shape::Box { .. } => Some((y, rect(mn[0], mn[2], mx[0], mx[2]))),
            Shape::Cylinder { radius, .. } if self.vertical_axis_is_local_y() => {
                let (cx, cz) = ((mn[0] + mx[0]) / 2.0, (mn[2] + mx[2]) / 2.0);
                Some((y, disk(cx, cz, radius, DISK_SEGMENTS)))
            }
            Shape::Cylinder { .. } => None,
        }
    }

    /// Footprint polygon on the floor when resting at the body's lowest point.
    /// For a lying cylinder this is the contact line (2 points).
    pub fn bottom_contact_region(&self) -> Vec<Pt2> {
        if let Some((_, poly)) = self.flat_face(false) {
            return poly;
        }
        let (mn, mx) = (self.min, self.max());
        let (cx, cz) = ((mn[0] + mx[0]) / 2.0, (mn[2] + mx[2]) / 2.0);
        // Lying cylinder: axis along whichever horizontal extent is the length.
        if self.shape.orientation.axis_map()[0] == 1 {
            vec![Pt2::new(mn[0], cz), Pt2::new(mx[0], cz)]
        } else {
            vec![Pt2::new(cx, mn[2]), Pt2::new(cx, mx[2])]
        }
    }
}

fn aabbs_overlap(a: &Body, b: &Body, margin: f64) -> bool {
    let (amin, amax, bmin, bmax) = (a.min, a.max(), b.min, b.max());
    (0..3).all(|k| amin[k] < bmax[k] - margin && bmin[k] < amax[k] - margin)
}

fn footprints_overlap(a: &Body, b: &Body, margin: f64) -> bool {
    let (amin, amax, bmin, bmax) = (a.min, a.max(), b.min, b.max());
    [0, 2].iter().all(|&k| amin[k] < bmax[k] - margin && bmin[k] < amax[k] - margin)
}

/// True if the two bodies interpenetrate by more than [`tol::PENETRATION`].
/// Touching is not a collision.
pub fn overlaps(a: &Body, b: &Body) -> bool {
    if !aabbs_overlap(a, b, tol::PENETRATION) {
        return false;
    }
    if a.is_box() && b.is_box() {
        return true; // axis-aligned boxes are their own AABB
    }
    match query::contact(
        &a.shape.isometry_at(a.min),
        a.shape.parry().as_ref(),
        &b.shape.isometry_at(b.min),
        b.shape.parry().as_ref(),
        0.0,
    ) {
        Ok(Some(c)) => c.dist < -tol::PENETRATION,
        Ok(None) => false,
        Err(_) => true, // unsupported pair: be conservative
    }
}

/// Lowest `y` at which `moving` (min corner at `x`, `z`) rests when lowered from
/// above onto `obstacles` or the floor. Never places the item under an overhang.
pub fn drop_height<'a>(moving: &OrientedShape, x: f64, z: f64, obstacles: impl IntoIterator<Item = Body<'a>>) -> f64 {
    let probe = Body::new(moving, [x, 0.0, z]);
    let below: Vec<Body> = obstacles
        .into_iter()
        .filter(|o| footprints_overlap(&probe, o, tol::PENETRATION))
        .collect();
    // Start clearly above everything: a cast that begins exactly in contact
    // may report no impact.
    const CLEARANCE: f64 = 1.0;
    let y_start = below.iter().map(|o| o.max()[1]).fold(0.0, f64::max) + CLEARANCE;
    let mut rest = 0.0f64;
    let moving_is_box = matches!(moving.shape, Shape::Box { .. });
    let start_iso = moving.isometry_at([x, y_start, z]);
    for o in &below {
        let top = o.max()[1];
        if top <= rest {
            continue;
        }
        if moving_is_box && o.is_box() {
            rest = top;
            continue;
        }
        let opts = ShapeCastOptions {
            max_time_of_impact: y_start - rest,
            target_distance: 0.0,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false,
        };
        let hit = query::cast_shapes(
            &start_iso,
            &Vector::new(0.0, -1.0, 0.0),
            moving.parry().as_ref(),
            &o.shape.isometry_at(o.min),
            &Vector::zeros(),
            o.shape.parry().as_ref(),
            opts,
        );
        match hit {
            Ok(Some(h)) => rest = rest.max(y_start - h.time_of_impact),
            Ok(None) => {}
            Err(_) => rest = rest.max(top),
        }
    }
    // Safety net against numerical misses: never return an interpenetrating pose.
    let at_rest = Body::new(moving, [x, rest, z]);
    if below.iter().any(|o| overlaps(&at_rest, o)) {
        return y_start - CLEARANCE;
    }
    rest
}

/// A point where `upper` pushes down on its supporter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportContact {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Contact points through which `upper` rests on `lower` (empty if none).
pub fn support_contacts(upper: &Body, lower: &Body) -> Vec<SupportContact> {
    if !aabbs_overlap(upper, lower, -tol::CONTACT) {
        return Vec::new();
    }
    // Flat-on-flat: the exact contact polygon.
    if let (Some((yb, bottom)), Some((yt, top))) = (upper.flat_face(false), lower.flat_face(true)) {
        if (yb - yt).abs() <= tol::CONTACT {
            let poly = clip_convex(&bottom, &top);
            if polygon_area(&poly) <= tol::CONTACT * tol::CONTACT {
                return Vec::new();
            }
            return poly.into_iter().map(|p| SupportContact { x: p.x, y: yb, z: p.z }).collect();
        }
        if yb > yt {
            return Vec::new();
        }
    }
    // Curved contacts: parry contact manifolds.
    let pos1 = upper.shape.isometry_at(upper.min);
    let pos2 = lower.shape.isometry_at(lower.min);
    let pos12 = pos1.inv_mul(&pos2);
    let mut manifolds: Vec<ContactManifold<(), ()>> = Vec::new();
    if DefaultQueryDispatcher
        .contact_manifolds(
            &pos12,
            upper.shape.parry().as_ref(),
            lower.shape.parry().as_ref(),
            tol::CONTACT,
            &mut manifolds,
            &mut None,
        )
        .is_err()
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for m in &manifolds {
        let n1 = pos1.rotation * m.local_n1;
        if n1.y > -tol::MIN_SUPPORT_NORMAL_Y {
            continue; // not pushing downwards
        }
        for c in &m.points {
            if c.dist <= tol::CONTACT {
                let p = pos1 * c.local_p1;
                out.push(SupportContact { x: p.x, y: p.y, z: p.z });
            }
        }
    }
    out
}

/// Contacts between `body` and the container floor (`y = 0`).
pub fn floor_contacts(body: &Body) -> Vec<SupportContact> {
    if body.min[1] > tol::CONTACT {
        return Vec::new();
    }
    body.bottom_contact_region()
        .into_iter()
        .map(|p| SupportContact { x: p.x, y: 0.0, z: p.z })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{Orientation, Shape};

    fn boxy(w: f64, h: f64, d: f64) -> OrientedShape {
        OrientedShape::new(&Shape::Box { w, h, d }, Orientation::Whd, [0.0; 3])
    }

    #[test]
    fn box_drops_onto_box() {
        let a = boxy(10.0, 10.0, 10.0);
        let placed = [Body::new(&a, [0.0, 0.0, 0.0])];
        assert_eq!(drop_height(&a, 5.0, 5.0, placed), 10.0);
        assert_eq!(drop_height(&a, 10.0, 0.0, placed), 0.0); // side by side, touching
    }

    #[test]
    fn box_drops_onto_lying_cylinder_ridge() {
        let cyl = OrientedShape::new(&Shape::Cylinder { radius: 10.0, length: 50.0 }, Orientation::Hwd, [0.0; 3]);
        let b = boxy(10.0, 10.0, 10.0);
        let placed = [Body::new(&cyl, [0.0, 0.0, 0.0])];
        // Centred on the ridge: rests on top (y = 20).
        assert!((drop_height(&b, 20.0, 5.0, placed) - 20.0).abs() < 1e-3);
        // Box spanning z in [15,25]: nearest point of circle (centre z=10) is dz = 5.
        let y = drop_height(&b, 20.0, 15.0, placed);
        let expect = 10.0 + (100.0f64 - 25.0).sqrt();
        assert!((y - expect).abs() < 1e-3, "{y} vs {expect}");
    }

    #[test]
    fn cylinder_lands_on_box_top() {
        let big = boxy(1400.0, 1300.0, 900.0);
        let drum = OrientedShape::new(&Shape::Cylinder { radius: 290.0, length: 880.0 }, Orientation::Whd, [0.0; 3]);
        let placed = [Body::new(&big, [0.0, 0.0, 0.0])];
        assert!((drop_height(&drum, 0.0, 0.0, placed) - 1300.0).abs() < 1e-3);
        let pipe = OrientedShape::new(&Shape::Cylinder { radius: 150.0, length: 1800.0 }, Orientation::Whd, [0.0; 3]);
        assert!((drop_height(&pipe, 0.0, 580.0, placed) - 1300.0).abs() < 1e-3);
    }

    #[test]
    fn cylinders_nest_in_groove() {
        let s = Shape::Cylinder { radius: 10.0, length: 50.0 };
        let c = OrientedShape::new(&s, Orientation::Hwd, [0.0; 3]);
        let placed = [Body::new(&c, [0.0, 0.0, 0.0]), Body::new(&c, [0.0, 0.0, 20.0])];
        let y = drop_height(&c, 0.0, 10.0, placed);
        // Centres form an equilateral triangle with side 20: height sqrt(300).
        let expect = 300f64.sqrt();
        assert!((y - expect).abs() < 1e-3, "{y} vs {expect}");
        let top = Body::new(&c, [0.0, y, 10.0]);
        assert!(!overlaps(&top, &placed[0]));
        let pts: usize = placed.iter().map(|p| support_contacts(&top, p).len()).sum();
        assert!(pts >= 2, "nested cylinder must touch both below, got {pts}");
    }

    #[test]
    fn overlap_is_exact_for_cylinders() {
        let s = Shape::Cylinder { radius: 10.0, length: 10.0 };
        let c = OrientedShape::new(&s, Orientation::Whd, [0.0; 3]);
        // AABBs overlap at the corners but the disks do not.
        let a = Body::new(&c, [0.0, 0.0, 0.0]);
        let b = Body::new(&c, [15.0, 0.0, 15.0]);
        assert!(!overlaps(&a, &b));
        let b2 = Body::new(&c, [10.0, 0.0, 5.0]);
        assert!(overlaps(&a, &b2));
    }

    #[test]
    fn flat_contact_polygon_is_intersection() {
        let a = boxy(10.0, 10.0, 10.0);
        let lower = Body::new(&a, [0.0, 0.0, 0.0]);
        let upper = Body::new(&a, [5.0, 10.0, 0.0]);
        let pts = support_contacts(&upper, &lower);
        let xs: Vec<f64> = pts.iter().map(|p| p.x).collect();
        assert!(xs.iter().all(|&x| (5.0 - 1e-9..=10.0 + 1e-9).contains(&x)));
        assert_eq!(pts.len(), 4);
    }
}
