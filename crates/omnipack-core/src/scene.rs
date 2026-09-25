//! Support and stability rules shared by the placer and the validator.

use crate::statics::{load_at, resultant, Load, Support, SupportSet};
use omnipack_geom::hull2d::polygon_area;
use omnipack_geom::{convex_hull, floor_contacts, support_contacts, tol, Body, Pt2};

/// Supports of one body plus the true contact area of its flat bottom.
pub struct SupportInfo {
    pub set: SupportSet,
    pub contact_area: f64,
}

pub fn compute_supports<'a>(
    body: &Body,
    others: impl IntoIterator<Item = (usize, Body<'a>)>,
) -> SupportInfo {
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
    SupportInfo {
        set: SupportSet::new(points, owners),
        contact_area: area,
    }
}

/// Area of a flat bottom face, if the body has one.
pub fn flat_bottom_area(body: &Body) -> Option<f64> {
    body.shape.bottom_area()
}

/// How a round item resting on a line or a point is kept from rolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roll {
    /// Not a round item on a line/point support: the normal tipping rules apply.
    NotApplicable,
    /// Wedged in by walls or neighbours on every side it could roll to.
    Blocked,
    /// Needs wooden wedges (chocks) to be applied when loading.
    NeedsChocks,
    /// Free to roll: not allowed.
    Free,
}

impl Roll {
    pub fn is_held(self) -> bool {
        matches!(self, Roll::Blocked | Roll::NeedsChocks)
    }
}

/// Horizontal axes (0 = x, 2 = z) along which a body on this support could roll.
fn roll_axes(set: &SupportSet) -> Vec<usize> {
    match set.hull.len() {
        2 => {
            let (a, b) = (set.hull[0], set.hull[1]);
            // Rolls across the contact line.
            if (b.x - a.x).abs() >= (b.z - a.z).abs() {
                vec![2]
            } else {
                vec![0]
            }
        }
        _ => vec![0, 2],
    }
}

fn blocked_along<'a>(body: &Body, k: usize, others: &[Body<'a>], container: [f64; 3]) -> bool {
    let (mn, mx) = (body.min, body.max());
    let mut neg = mn[k] <= tol::CONTACT;
    let mut pos = mx[k] >= container[k] - tol::CONTACT;
    let other_axes: Vec<usize> = [0usize, 1, 2].into_iter().filter(|&a| a != k).collect();
    for o in others {
        let (omn, omx) = (o.min, o.max());
        let side_by_side = other_axes
            .iter()
            .all(|&a| omn[a] < mx[a] - tol::CONTACT && mn[a] < omx[a] - tol::CONTACT);
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

/// Rolling state of `body` on `set`, given its neighbours and the container.
pub fn roll_state<'a>(
    body: &Body,
    set: &SupportSet,
    others: impl IntoIterator<Item = Body<'a>>,
    container: [f64; 3],
    use_chocks: bool,
) -> Roll {
    if !body.shape.shape.can_roll() || set.is_empty() || !set.is_degenerate() {
        return Roll::NotApplicable;
    }
    let others: Vec<Body> = others.into_iter().collect();
    if roll_axes(set)
        .into_iter()
        .all(|k| blocked_along(body, k, &others, container))
    {
        Roll::Blocked
    } else if use_chocks {
        Roll::NeedsChocks
    } else {
        Roll::Free
    }
}

/// Stability margin (mm) of a load resultant at `p` on `set`. A held round
/// item on a line is judged along the line only; on a point it cannot tip.
pub fn stability_margin(set: &SupportSet, p: Pt2, held: bool) -> f64 {
    if !held || !set.is_degenerate() {
        return set.margin(p);
    }
    if set.hull.len() < 2 {
        return f64::INFINITY;
    }
    let (a, b) = (set.hull[0], set.hull[1]);
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let len = (dx * dx + dz * dz).sqrt();
    if len <= 0.0 {
        return f64::INFINITY;
    }
    let t = ((p.x - a.x) * dx + (p.z - a.z) * dz) / len;
    t.min(len - t)
}

/// Splits `load` over the supports. For a held round item the wedges take the
/// horizontal moment, so the load is applied at the nearest support point.
pub fn distribute(set: &SupportSet, load: Load, held: bool) -> Option<Vec<(Support, Load)>> {
    if held && set.is_degenerate() {
        let r = resultant(load)?;
        let p = nearest_on_hull(&set.hull, r);
        return set.distribute(load_at(load[0], p));
    }
    set.distribute(load)
}

fn nearest_on_hull(hull: &[Pt2], p: Pt2) -> Pt2 {
    match hull.len() {
        0 => p,
        1 => hull[0],
        _ => {
            let (a, b) = (hull[0], hull[1]);
            let (dx, dz) = (b.x - a.x, b.z - a.z);
            let len2 = dx * dx + dz * dz;
            let t = if len2 > 0.0 {
                (((p.x - a.x) * dx + (p.z - a.z) * dz) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            Pt2::new(a.x + t * dx, a.z + t * dz)
        }
    }
}

/// Margin an item needs: a fraction of its smaller half-footprint.
pub fn required_margin(body: &Body, fraction: f64) -> f64 {
    fraction.max(0.0) * body.shape.extents[0].min(body.shape.extents[2]) / 2.0
}
