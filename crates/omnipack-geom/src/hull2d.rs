//! 2D convex hull and point-in-polygon distance, used for support polygons on
//! the horizontal (XZ) plane.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pt2 {
    pub x: f64,
    pub z: f64,
}

impl Pt2 {
    pub fn new(x: f64, z: f64) -> Self {
        Pt2 { x, z }
    }
}

fn cross(o: Pt2, a: Pt2, b: Pt2) -> f64 {
    (a.x - o.x) * (b.z - o.z) - (a.z - o.z) * (b.x - o.x)
}

/// Andrew's monotone chain. Returns the hull counter-clockwise with no
/// collinear points; degenerate inputs give 1 or 2 points.
pub fn convex_hull(points: &[Pt2]) -> Vec<Pt2> {
    let mut pts: Vec<Pt2> = points.to_vec();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.z.total_cmp(&b.z)));
    pts.dedup_by(|a, b| (a.x - b.x).abs() < 1e-9 && (a.z - b.z).abs() < 1e-9);
    if pts.len() < 3 {
        return pts;
    }
    let mut hull: Vec<Pt2> = Vec::with_capacity(pts.len() * 2);
    for &p in &pts {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 1e-12 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower = hull.len() + 1;
    for &p in pts.iter().rev().skip(1) {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 1e-12 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop();
    hull
}

fn dist_point_segment(p: Pt2, a: Pt2, b: Pt2) -> f64 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let len2 = dx * dx + dz * dz;
    let t = if len2 > 0.0 { (((p.x - a.x) * dx + (p.z - a.z) * dz) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cz) = (a.x + t * dx, a.z + t * dz);
    ((p.x - cx).powi(2) + (p.z - cz).powi(2)).sqrt()
}

/// Signed distance from `p` to the boundary of a convex CCW polygon:
/// positive inside, negative outside. Degenerate polygons (point, segment)
/// have no interior, so the result is `-distance` (≤ 0), which lets callers
/// accept "exactly on the line" only when they allow zero margin.
pub fn signed_distance_to_polygon(p: Pt2, hull: &[Pt2]) -> f64 {
    match hull.len() {
        0 => f64::NEG_INFINITY,
        1 => -((p.x - hull[0].x).powi(2) + (p.z - hull[0].z).powi(2)).sqrt(),
        2 => -dist_point_segment(p, hull[0], hull[1]),
        n => {
            let mut inside = true;
            let mut min_d = f64::INFINITY;
            for i in 0..n {
                let a = hull[i];
                let b = hull[(i + 1) % n];
                if cross(a, b, p) < 0.0 {
                    inside = false;
                }
                min_d = min_d.min(dist_point_segment(p, a, b));
            }
            if inside {
                min_d
            } else {
                -min_d
            }
        }
    }
}

/// Distance from `p` along the unit direction `d` (x, z) to the boundary of a
/// convex polygon (0 if `p` is outside or the polygon is degenerate).
pub fn ray_exit_distance(hull: &[Pt2], p: Pt2, d: [f64; 2]) -> f64 {
    let n = hull.len();
    if n < 3 {
        return 0.0;
    }
    let mut best = f64::INFINITY;
    for i in 0..n {
        let (a, b) = (hull[i], hull[(i + 1) % n]);
        let (ex, ez) = (b.x - a.x, b.z - a.z);
        let denom = d[0] * ez - d[1] * ex;
        if denom.abs() < 1e-12 {
            continue;
        }
        let (wx, wz) = (a.x - p.x, a.z - p.z);
        let t = (wx * ez - wz * ex) / denom;
        let u = (wx * d[1] - wz * d[0]) / denom;
        if t >= -1e-9 && (-1e-9..=1.0 + 1e-9).contains(&u) {
            best = best.min(t.max(0.0));
        }
    }
    if best.is_finite() {
        best
    } else {
        0.0
    }
}

/// Intersection of two convex CCW polygons (Sutherland–Hodgman).
pub fn clip_convex(subject: &[Pt2], clip: &[Pt2]) -> Vec<Pt2> {
    let mut out: Vec<Pt2> = subject.to_vec();
    let n = clip.len();
    for i in 0..n {
        if out.is_empty() {
            break;
        }
        let (a, b) = (clip[i], clip[(i + 1) % n]);
        let input = std::mem::take(&mut out);
        let m = input.len();
        for j in 0..m {
            let p = input[j];
            let q = input[(j + 1) % m];
            let (dp, dq) = (cross(a, b, p), cross(a, b, q));
            if dp >= 0.0 {
                out.push(p);
            }
            if (dp >= 0.0) != (dq >= 0.0) {
                let t = dp / (dp - dq);
                out.push(Pt2::new(p.x + t * (q.x - p.x), p.z + t * (q.z - p.z)));
            }
        }
    }
    out
}

pub fn polygon_area(poly: &[Pt2]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        a += p.x * q.z - q.x * p.z;
    }
    a.abs() / 2.0
}

/// CCW rectangle `[x0,x1] × [z0,z1]`.
pub fn rect(x0: f64, z0: f64, x1: f64, z1: f64) -> Vec<Pt2> {
    vec![Pt2::new(x0, z0), Pt2::new(x1, z0), Pt2::new(x1, z1), Pt2::new(x0, z1)].pipe_ccw()
}

/// Regular polygon inscribed in a circle (conservative stand-in for a disk).
pub fn disk(cx: f64, cz: f64, r: f64, segments: usize) -> Vec<Pt2> {
    (0..segments)
        .map(|i| {
            let t = i as f64 / segments as f64 * std::f64::consts::TAU;
            Pt2::new(cx + r * t.cos(), cz + r * t.sin())
        })
        .collect::<Vec<_>>()
        .pipe_ccw()
}

trait PipeCcw {
    fn pipe_ccw(self) -> Vec<Pt2>;
}

impl PipeCcw for Vec<Pt2> {
    /// Orders a convex polygon counter-clockwise in the (x, z) plane.
    fn pipe_ccw(mut self) -> Vec<Pt2> {
        let n = self.len();
        let mut s = 0.0;
        for i in 0..n {
            let (p, q) = (self[i], self[(i + 1) % n]);
            s += p.x * q.z - q.x * p.z;
        }
        if s < 0.0 {
            self.reverse();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_distance_in_square() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        assert!((ray_exit_distance(&sq, Pt2::new(3.0, 5.0), [1.0, 0.0]) - 7.0).abs() < 1e-9);
        assert!((ray_exit_distance(&sq, Pt2::new(3.0, 5.0), [0.0, -1.0]) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn clip_rects_and_disk() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 5.0, 20.0, 20.0);
        assert!((polygon_area(&clip_convex(&a, &b)) - 25.0).abs() < 1e-9);
        let d = disk(0.0, 0.0, 10.0, 64);
        let half = clip_convex(&d, &rect(0.0, -20.0, 20.0, 20.0));
        let expect = std::f64::consts::PI * 100.0 / 2.0;
        assert!((polygon_area(&half) - expect).abs() / expect < 0.01);
        assert!(clip_convex(&a, &rect(10.0, 0.0, 20.0, 10.0)).len() <= 4);
        assert!(polygon_area(&clip_convex(&a, &rect(10.0, 0.0, 20.0, 10.0))) < 1e-9);
    }

    #[test]
    fn square_hull_and_distance() {
        let pts = [
            Pt2::new(0.0, 0.0),
            Pt2::new(10.0, 0.0),
            Pt2::new(10.0, 10.0),
            Pt2::new(0.0, 10.0),
            Pt2::new(5.0, 5.0),
            Pt2::new(5.0, 0.0),
        ];
        let h = convex_hull(&pts);
        assert_eq!(h.len(), 4);
        assert!((signed_distance_to_polygon(Pt2::new(5.0, 5.0), &h) - 5.0).abs() < 1e-9);
        assert!((signed_distance_to_polygon(Pt2::new(1.0, 5.0), &h) - 1.0).abs() < 1e-9);
        assert!((signed_distance_to_polygon(Pt2::new(12.0, 5.0), &h) + 2.0).abs() < 1e-9);
    }

    #[test]
    fn degenerate_hulls_are_never_strictly_inside() {
        let seg = convex_hull(&[Pt2::new(0.0, 0.0), Pt2::new(10.0, 0.0)]);
        assert_eq!(seg.len(), 2);
        assert_eq!(signed_distance_to_polygon(Pt2::new(5.0, 0.0), &seg), 0.0);
        assert!(signed_distance_to_polygon(Pt2::new(5.0, 1.0), &seg) < 0.0);
    }
}
