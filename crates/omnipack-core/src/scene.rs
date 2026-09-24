//! Support and stability rules shared by the placer and the validator.

use crate::statics::{Support, SupportSet};
use omnipack_geom::hull2d::polygon_area;
use omnipack_geom::{convex_hull, floor_contacts, support_contacts, tol, Body, Pt2, Shape};

/// Supports of one body plus the true contact area of its flat bottom.
pub struct SupportInfo {
    pub set: SupportSet,
    pub contact_area: f64,
}

pub fn compute_supports<'a>(body: &Body, others: impl IntoIterator<Item = (usize, Body<'a>)>) -> SupportInfo {
    let mut points = Vec::new();
    let mut owners = Vec::new();
    let mut area = 0.0;
    let floor = floor_contacts(body);
    if !floor.is_empty() {
        let pts: Vec<Pt2> = floor.iter().map(|c| Pt2::new(c.x, c.z)).collect();
        area += polygon_area(&convex_hull(&pts));
        owners.extend(std::iter::repeat_n(Support::Floor, pts.len()));
        points.extend(pts);
    }
    for (j, other) in others {
        let c = support_contacts(body, &other);
        if c.is_empty() {
            continue;
        }
        let pts: Vec<Pt2> = c.iter().map(|c| Pt2::new(c.x, c.z)).collect();
        area += polygon_area(&convex_hull(&pts));
        owners.extend(std::iter::repeat_n(Support::Item(j), pts.len()));
        points.extend(pts);
    }
    SupportInfo { set: SupportSet::new(points, owners), contact_area: area }
}

/// Area of a flat bottom face, if the body has one.
pub fn flat_bottom_area(body: &Body) -> Option<f64> {
    match body.shape.shape {
        Shape::Box { .. } => Some(body.shape.extents[0] * body.shape.extents[2]),
        Shape::Cylinder { radius, .. } if body.shape.orientation.axis_map()[1] == 1 => {
            Some(std::f64::consts::PI * radius * radius)
        }
        Shape::Cylinder { .. } => None,
    }
}

/// A cylinder lying on its side.
pub fn is_lying_cylinder(body: &Body) -> bool {
    matches!(body.shape.shape, Shape::Cylinder { .. }) && body.shape.orientation.axis_map()[1] != 1
}

/// Horizontal world axis (0 = x, 2 = z) along which a lying cylinder would roll.
fn roll_axis(body: &Body) -> usize {
    if body.shape.orientation.axis_map()[0] == 1 {
        2 // cylinder axis along x, rolls along z
    } else {
        0
    }
}

/// True if a lying cylinder is chocked on both sides of its rolling direction
/// by a wall or by a touching neighbour.
pub fn roll_blocked<'a>(body: &Body, others: impl IntoIterator<Item = Body<'a>>, container: [f64; 3]) -> bool {
    let k = roll_axis(body);
    let (mn, mx) = (body.min, body.max());
    let mut neg = mn[k] <= tol::CONTACT;
    let mut pos = mx[k] >= container[k] - tol::CONTACT;
    let other_axes = [0usize, 1, 2].into_iter().filter(|&a| a != k).collect::<Vec<_>>();
    for o in others {
        let (omn, omx) = (o.min, o.max());
        let side_by_side = other_axes.iter().all(|&a| omn[a] < mx[a] - tol::CONTACT && mn[a] < omx[a] - tol::CONTACT);
        if !side_by_side {
            continue;
        }
        if (omx[k] - mn[k]).abs() <= tol::CONTACT {
            neg = true;
        }
        if (omn[k] - mx[k]).abs() <= tol::CONTACT {
            pos = true;
        }
    }
    neg && pos
}

/// Stability margin (mm) of a load resultant at `p` on `set`. A chocked lying
/// cylinder on a line support is judged along its axis only.
pub fn stability_margin(set: &SupportSet, p: Pt2, chocked_line: bool) -> f64 {
    if !(chocked_line && set.hull.len() == 2) {
        return set.margin(p);
    }
    let (a, b) = (set.hull[0], set.hull[1]);
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let len = (dx * dx + dz * dz).sqrt();
    if len <= 0.0 {
        return set.margin(p);
    }
    let t = ((p.x - a.x) * dx + (p.z - a.z) * dz) / len;
    let perp = ((p.x - a.x) * dz - (p.z - a.z) * dx).abs() / len;
    if perp > tol::CONTACT {
        return -perp;
    }
    t.min(len - t)
}

/// Margin an item needs: a fraction of its smaller half-footprint.
pub fn required_margin(body: &Body, fraction: f64) -> f64 {
    fraction.max(0.0) * body.shape.extents[0].min(body.shape.extents[2]) / 2.0
}
