//! Static equilibrium: how weight flows from each item through its contact
//! points down to the floor.
//!
//! Contacts are modelled as equal, tension-free springs (a Winkler
//! foundation): the contact forces are the minimum-norm solution of vertical
//! force and moment balance, and any contact that would have to pull is
//! released and the system re-solved. This is statically admissible, gives the
//! familiar linear pressure distribution under eccentric loads, and fails
//! exactly when the load resultant leaves the support polygon (tipping).
//!
//! Loads are in kg-force; positions in mm on the XZ plane.

use omnipack_geom::na::{DMatrix, DVector};
use omnipack_geom::{convex_hull, signed_distance_to_polygon, Pt2};

/// What carries a contact point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Support {
    Floor,
    Item(usize),
}

/// A load summary: total force and its first moments (Σf, Σf·x, Σf·z).
pub type Load = [f64; 3];

/// Massless items (e.g. volume-only benchmarks) still need a load resultant
/// for the tipping check; they get a negligible weight.
pub fn effective_mass(mass: f64) -> f64 {
    mass.max(1e-6)
}

pub fn load_at(f: f64, p: Pt2) -> Load {
    [f, f * p.x, f * p.z]
}

pub fn add(a: Load, b: Load) -> Load {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Load, b: Load) -> Load {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Where a load acts (its resultant point). `None` for zero load.
pub fn resultant(l: Load) -> Option<Pt2> {
    (l[0] > 0.0).then(|| Pt2::new(l[1] / l[0], l[2] / l[0]))
}

/// The contact points holding one body up.
#[derive(Debug, Clone, Default)]
pub struct SupportSet {
    pub points: Vec<Pt2>,
    pub owners: Vec<Support>,
    pub hull: Vec<Pt2>,
}

impl SupportSet {
    pub fn new(points: Vec<Pt2>, owners: Vec<Support>) -> Self {
        let hull = convex_hull(&points);
        SupportSet { points, owners, hull }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Signed distance from `p` to the support polygon edge (positive = inside).
    pub fn margin(&self, p: Pt2) -> f64 {
        signed_distance_to_polygon(p, &self.hull)
    }

    /// True when the supports form a line or point (e.g. a lying cylinder).
    pub fn is_degenerate(&self) -> bool {
        self.hull.len() < 3
    }

    /// Splits `load` (acting at its resultant) over the contacts and sums the
    /// share of each supporter. `None` if equilibrium is impossible.
    pub fn distribute(&self, load: Load) -> Option<Vec<(Support, Load)>> {
        let at = resultant(load)?;
        let forces = distribute(&self.points, at, load[0])?;
        let mut out: Vec<(Support, Load)> = Vec::new();
        for ((&owner, &p), f) in self.owners.iter().zip(&self.points).zip(forces) {
            if f <= 0.0 {
                continue;
            }
            match out.iter_mut().find(|(o, _)| *o == owner) {
                Some((_, l)) => *l = add(*l, load_at(f, p)),
                None => out.push((owner, load_at(f, p))),
            }
        }
        Some(out)
    }
}

/// Non-negative point forces summing to `total` whose moment about `at` is
/// zero. `None` if no such forces exist (the resultant is outside the support).
pub fn distribute(points: &[Pt2], at: Pt2, total: f64) -> Option<Vec<f64>> {
    let n = points.len();
    if n == 0 || total <= 0.0 {
        return if total <= 0.0 { Some(vec![0.0; n]) } else { None };
    }
    // Normalise lever arms so the pseudo-inverse tolerance is scale-free.
    let scale = points
        .iter()
        .map(|p| (p.x - at.x).abs().max((p.z - at.z).abs()))
        .fold(1e-9, f64::max);
    let mut active = vec![true; n];
    for _ in 0..n {
        let idx: Vec<usize> = (0..n).filter(|&i| active[i]).collect();
        if idx.is_empty() {
            return None;
        }
        let k = idx.len();
        let a = DMatrix::from_fn(3, k, |r, c| {
            let p = points[idx[c]];
            match r {
                0 => 1.0,
                1 => (p.x - at.x) / scale,
                _ => (p.z - at.z) / scale,
            }
        });
        let b = DVector::from_vec(vec![1.0, 0.0, 0.0]);
        let pinv = (&a * a.transpose()).pseudo_inverse(1e-10).ok()?;
        let f = a.transpose() * (pinv * &b);
        let (worst, worst_f) = (0..k).map(|c| (c, f[c])).fold((0, f64::INFINITY), |acc, x| if x.1 < acc.1 { x } else { acc });
        if worst_f < -1e-9 {
            active[idx[worst]] = false;
            continue;
        }
        if (&a * &f - &b).norm() > 1e-6 {
            return None;
        }
        let mut out = vec![0.0; n];
        for (c, &i) in idx.iter().enumerate() {
            out[i] = f[c].max(0.0) * total;
        }
        return Some(out);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Pt2> {
        vec![Pt2::new(0.0, 0.0), Pt2::new(10.0, 0.0), Pt2::new(10.0, 10.0), Pt2::new(0.0, 10.0)]
    }

    #[test]
    fn centred_load_splits_evenly() {
        let f = distribute(&square(), Pt2::new(5.0, 5.0), 100.0).unwrap();
        for v in f {
            assert!((v - 25.0).abs() < 1e-9);
        }
    }

    #[test]
    fn eccentric_load_balances_moments() {
        let pts = square();
        let at = Pt2::new(8.0, 3.0);
        let f = distribute(&pts, at, 100.0).unwrap();
        let s: f64 = f.iter().sum();
        let mx: f64 = f.iter().zip(&pts).map(|(f, p)| f * (p.x - at.x)).sum();
        let mz: f64 = f.iter().zip(&pts).map(|(f, p)| f * (p.z - at.z)).sum();
        assert!((s - 100.0).abs() < 1e-6 && mx.abs() < 1e-6 && mz.abs() < 1e-6);
        assert!(f.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn load_outside_support_is_impossible() {
        assert!(distribute(&square(), Pt2::new(11.0, 5.0), 100.0).is_none());
    }

    #[test]
    fn line_support_holds_load_on_the_line() {
        let pts = [Pt2::new(0.0, 5.0), Pt2::new(10.0, 5.0)];
        let f = distribute(&pts, Pt2::new(2.5, 5.0), 100.0).unwrap();
        assert!((f[0] - 75.0).abs() < 1e-6 && (f[1] - 25.0).abs() < 1e-6);
        assert!(distribute(&pts, Pt2::new(2.5, 6.0), 100.0).is_none());
    }

    #[test]
    fn two_supporters_share_by_lever_rule() {
        // A beam on two blocks at x in [0,2] and [8,10], load at x = 3.
        let pts = vec![
            Pt2::new(0.0, 0.0), Pt2::new(2.0, 0.0), Pt2::new(2.0, 1.0), Pt2::new(0.0, 1.0),
            Pt2::new(8.0, 0.0), Pt2::new(10.0, 0.0), Pt2::new(10.0, 1.0), Pt2::new(8.0, 1.0),
        ];
        let owners = [0, 0, 0, 0, 1, 1, 1, 1].map(Support::Item).to_vec();
        let set = SupportSet::new(pts, owners);
        let shares = set.distribute(load_at(100.0, Pt2::new(3.0, 0.5))).unwrap();
        let left = shares.iter().find(|(o, _)| *o == Support::Item(0)).unwrap().1[0];
        let right = shares.iter().find(|(o, _)| *o == Support::Item(1)).unwrap().1[0];
        assert!((left + right - 100.0).abs() < 1e-6);
        assert!(left > right, "load nearer the left block must load it more: {left} vs {right}");
    }
}
