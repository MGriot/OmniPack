//! Load distribution: where the cargo's weight sits in the container and on
//! the road vehicle.
//!
//! - CTU Code (IMO/ILO/UNECE 2014, annex 7): the cargo's centre of gravity
//!   within a share of the length and width from the middle, enough mass in
//!   the middle half of the length, and the centre of gravity low.
//! - SOLAS VI/2: verified gross mass = tare + cargo (method 2).
//! - Directive 96/53/EC as amended by 2015/719: axle loads of a tractor and
//!   semi-trailer, by moments about the kingpin and the axles.
//! - ISO 1496-1: floor pressure against the floor rating.
//!
//! The results are warnings in [`BalanceReport`]; the hard limits stay in
//! [`crate::model::CogLimits`] and the container's axles.

use crate::model::{ContainerSpec, PackOptions};
use crate::plan::{BalanceIssue, BalanceReport, Metrics, Placement, VehicleLoads};

const AXLE_NAMES: [&str; 4] = ["steer axle", "drive axle", "trailer axles", "gross mass"];

/// How much each unit counts: its mass, or its volume for massless cargo.
fn weights(placements: &[Placement]) -> Vec<f64> {
    let massless = placements.iter().all(|p| p.mass <= 0.0);
    placements.iter().map(|p| if massless { p.shape.volume() } else { p.mass.max(0.0) }).collect()
}

/// Share of the weight between `z0` and `z1` with the load moved by `shift`,
/// each unit taken as uniform along its length.
fn share_between(placements: &[Placement], w: &[f64], z0: f64, z1: f64, shift: f64) -> f64 {
    let total: f64 = w.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let inside: f64 = placements
        .iter()
        .zip(w)
        .map(|(p, &m)| {
            let (a, b) = (p.position[2] + shift, p.position[2] + p.size[2] + shift);
            if b - a <= 0.0 {
                return if a >= z0 && a <= z1 { m } else { 0.0 };
            }
            m * (b.min(z1) - a.max(z0)).max(0.0) / (b - a)
        })
        .sum();
    inside / total
}

/// Weighted centre of gravity along Z (mm), unmoved.
fn cog_z(placements: &[Placement], w: &[f64]) -> f64 {
    let total: f64 = w.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    placements.iter().zip(w).map(|(p, m)| m * p.center_of_mass[2]).sum::<f64>() / total
}

/// Vehicle ground loads for the cargo plus the container tare (at mid-length).
fn vehicle_loads(c: &ContainerSpec, mass: f64, cog_z: f64) -> Option<VehicleLoads> {
    let v = c.vehicle.as_ref()?;
    let tare = c.tare_mass.unwrap_or(0.0);
    let total = mass + tare;
    let z = if total > 0.0 { (mass * cog_z + tare * c.depth / 2.0) / total } else { c.depth / 2.0 };
    let [steer, drive, trailer, gross] = v.axle_loads(total, z);
    Some(VehicleLoads { steer, drive, trailer, gross, limits: v.limits() })
}

/// Percentage points by which `value` exceeds `limit`, relative to `scale`.
fn over(value: f64, limit: f64, scale: f64) -> f64 {
    if value > limit + 1e-9 && scale > 0.0 {
        (value - limit) / scale * 100.0
    } else {
        0.0
    }
}

/// Exceedance (percentage points) of the checks that depend on where the load
/// sits lengthwise, with the load moved by `shift`: CTU centre-of-gravity
/// window and central share, the container's axles, the vehicle axles and
/// the hard longitudinal window.
fn lengthwise_excess(c: &ContainerSpec, opts: &PackOptions, placements: &[Placement], w: &[f64], cz0: f64, shift: f64) -> f64 {
    let b = &opts.balance;
    let d = c.depth;
    let cz = cz0 + shift;
    let mass: f64 = placements.iter().map(|p| p.mass).sum();
    let mut e = 0.0;
    if b.ctu_checks {
        e += over((cz - d / 2.0).abs(), b.max_eccentricity * d, d);
        e += over(b.min_central_share, share_between(placements, w, 0.25 * d, 0.75 * d, shift), 1.0);
    }
    if let Some([a0, a1]) = c.axles {
        let span = a1.z - a0.z;
        let rb = if span.abs() > 0.0 { mass * (cz - a0.z) / span } else { mass / 2.0 };
        e += over(mass - rb, a0.max_load, a0.max_load) + over(rb, a1.max_load, a1.max_load);
    }
    if let Some(v) = vehicle_loads(c, mass, cz) {
        for (load, max) in [v.steer, v.drive, v.trailer].into_iter().zip(v.limits) {
            e += over(load, max, max);
        }
    }
    let lim = &c.cog_limits;
    if let Some(z) = lim.z_min {
        e += over(z, cz, d);
    }
    if let Some(z) = lim.z_max {
        e += over(cz, z, d);
    }
    e
}

/// Slides the whole load along Z by the shortest distance that best meets the
/// lengthwise checks (see [`lengthwise_excess`]), within the free length at
/// both ends. Returns the shift applied (mm, + = towards the door; 0 = none).
/// A rigid translation keeps every support, margin and loading step valid.
pub fn centre_lengthwise(c: &ContainerSpec, opts: &PackOptions, placements: &mut [Placement]) -> f64 {
    if placements.is_empty() {
        return 0.0;
    }
    let lo = -placements.iter().map(|p| p.position[2]).fold(f64::INFINITY, f64::min).max(0.0);
    let hi = (c.depth - placements.iter().map(|p| p.position[2] + p.size[2]).fold(f64::NEG_INFINITY, f64::max)).max(0.0);
    if hi - lo < 1.0 {
        return 0.0;
    }
    let w = weights(placements);
    let cz0 = cog_z(placements, &w);
    let excess = |s: f64| lengthwise_excess(c, opts, placements, &w, cz0, s);
    let at_zero = excess(0.0);
    if at_zero <= 1e-9 {
        return 0.0;
    }
    let steps = ((hi - lo) / 1.0).ceil().clamp(1.0, 2000.0) as usize;
    let step = (hi - lo) / steps as f64;
    let (mut best_e, mut best_s) = (at_zero, 0.0f64);
    for k in 0..=steps {
        let s = lo + k as f64 * step;
        let e = excess(s);
        if e < best_e - 1e-9 || (e <= best_e + 1e-9 && s.abs() < best_s.abs()) {
            (best_e, best_s) = (e, s);
        }
    }
    if best_e >= at_zero - 1e-9 {
        return 0.0;
    }
    // Shortest shift with the same result: bisect back towards zero.
    let (mut inner, mut outer) = ((best_s.abs() - step).max(0.0).copysign(best_s), best_s);
    for _ in 0..40 {
        let mid = (inner + outer) / 2.0;
        if excess(mid) <= best_e + 1e-9 {
            outer = mid;
        } else {
            inner = mid;
        }
    }
    let shift = ((outer * 10.0).round() / 10.0).clamp(lo, hi);
    let shift = if excess(shift) <= best_e + 1e-9 { shift } else { outer.clamp(lo, hi) };
    for p in placements.iter_mut() {
        p.position[2] += shift;
        p.center_of_mass[2] += shift;
    }
    shift
}

/// The load distribution report of one finished container. `floor_load` is
/// the mass (kg) each placement puts on the floor; `shift` what
/// [`centre_lengthwise`] moved.
pub fn report(c: &ContainerSpec, opts: &PackOptions, placements: &[Placement], metrics: &Metrics, floor_load: &[f64], shift: f64) -> BalanceReport {
    let b = &opts.balance;
    let (w, h, d) = (c.width, c.height, c.depth);
    let mass = metrics.total_mass;
    let mut r = BalanceReport { shift, vgm: c.tare_mass.map(|t| t + mass), ..Default::default() };
    if placements.is_empty() {
        r.end_gaps = [d, d];
        return r;
    }
    let weight = weights(placements);
    let cog = metrics.center_of_mass;
    r.lengthwise_offset = cog[2] - d / 2.0;
    r.lateral_offset = cog[0] - w / 2.0;
    r.central_share = share_between(placements, &weight, 0.25 * d, 0.75 * d, 0.0);
    r.half_shares = [share_between(placements, &weight, f64::NEG_INFINITY, d / 2.0, 0.0), share_between(placements, &weight, d / 2.0, f64::INFINITY, 0.0)];
    r.cog_height_ratio = cog[1] / h;
    let front = placements.iter().map(|p| p.position[2]).fold(f64::INFINITY, f64::min);
    let back = placements.iter().map(|p| p.position[2] + p.size[2]).fold(f64::NEG_INFINITY, f64::max);
    r.end_gaps = [front.max(0.0), (d - back).max(0.0)];

    if b.ctu_checks {
        let (lim_l, lim_w, lim_h) = (b.max_eccentricity * d, b.max_eccentricity * w, b.max_cog_height * h);
        if r.lengthwise_offset.abs() > lim_l + 1e-6 {
            r.excess += over(r.lengthwise_offset.abs(), lim_l, d);
            r.issues.push(BalanceIssue::CogLengthwise { offset: r.lengthwise_offset, limit: lim_l });
        }
        if r.lateral_offset.abs() > lim_w + 1e-6 {
            r.excess += over(r.lateral_offset.abs(), lim_w, w);
            r.issues.push(BalanceIssue::CogLateral { offset: r.lateral_offset, limit: lim_w });
        }
        if r.central_share < b.min_central_share - 1e-9 {
            r.excess += over(b.min_central_share, r.central_share, 1.0);
            r.issues.push(BalanceIssue::CentralShare { share: r.central_share, min: b.min_central_share });
        }
        if cog[1] > lim_h + 1e-6 {
            r.excess += over(cog[1], lim_h, h);
            r.issues.push(BalanceIssue::CogHeight { height: cog[1], limit: lim_h });
        }
    }
    if let Some(v) = vehicle_loads(c, mass, cog[2]) {
        for ((name, load), max) in AXLE_NAMES.iter().zip([v.steer, v.drive, v.trailer, v.gross]).zip(v.limits) {
            if load > max + 1e-6 {
                r.excess += over(load, max, max);
                r.issues.push(BalanceIssue::AxleOverload { axle: (*name).into(), load, max });
            }
        }
        r.vehicle = Some(v);
    }
    if let Some(rating) = c.floor_rating.filter(|r| *r > 0.0) {
        for (p, &load) in placements.iter().zip(floor_load) {
            if p.floor_pressure > rating * (1.0 + 1e-9) {
                r.issues.push(BalanceIssue::FloorPressure { item: p.instance_id.clone(), pressure: p.floor_pressure, limit: rating, spread_area: load / rating });
            }
        }
    }
    r
}

/// Number of container-level balance warnings (everything but floor pressure).
pub fn container_issues(r: &BalanceReport) -> usize {
    r.issues.iter().filter(|i| !matches!(i, BalanceIssue::FloorPressure { .. })).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LashingOptions, RoadVehicle, ShipMotion, TransportCase};
    use omnipack_geom::{Orientation, Shape};

    fn unit(z: f64, d: f64, mass: f64) -> Placement {
        Placement {
            instance_id: format!("u{z}"),
            item_id: "u".into(),
            seq: 0,
            shape: Shape::Box { w: 100.0, h: 100.0, d },
            orientation: Orientation::Whd,
            position: [0.0, 0.0, z],
            size: [100.0, 100.0, d],
            center_of_mass: [50.0, 50.0, z + d / 2.0],
            mass,
            load_on_top: 0.0,
            support_margin: 0.0,
            stop: 0,
            needs_chocks: false,
            securing: Default::default(),
            impact: None,
            color: None,
            floor_pressure: 0.0,
        }
    }

    #[test]
    fn central_share_splits_units_across_the_quarter_line() {
        // 1000 mm container: the middle half is 250..750. A unit 200..400 has half inside.
        let p = [unit(200.0, 200.0, 10.0), unit(800.0, 100.0, 10.0)];
        let w = weights(&p);
        assert!((share_between(&p, &w, 250.0, 750.0, 0.0) - 0.375).abs() < 1e-9);
        // Moved 100 mm towards the door: 300..500 fully inside, 900..1000 outside.
        assert!((share_between(&p, &w, 250.0, 750.0, 100.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn vehicle_axle_loads_follow_the_lever_rule() {
        let v = &RoadVehicle::presets()[0];
        let [steer, drive, trailer, gross] = v.axle_loads(20_000.0, 6016.0);
        // x = 5016 behind the kingpin; trailer = (20000·5016 + 4800·5000) / 7700.
        assert!((trailer - 16_145.454_545).abs() < 1e-3, "{trailer}");
        let kingpin = 24_800.0 - trailer;
        assert!((drive - (kingpin * 3200.0 + 7500.0 * 1100.0) / 3800.0).abs() < 1e-6);
        assert!((steer - 6_695.454_545).abs() < 1e-3, "{steer}");
        assert!((gross - 32_300.0).abs() < 1e-9);
        assert!((steer + drive + trailer - gross).abs() < 1e-6, "loads must add up");
    }

    #[test]
    fn ship_motion_gives_roll_period_and_accelerations() {
        let s = TransportCase::from_ship(&ShipMotion::default());
        assert!((s.roll_period - 21.033).abs() < 1e-2, "{}", s.roll_period);
        assert_eq!(s.case.sideways, 0.43);
        assert_eq!(s.case.forward, 0.27);
        assert_eq!(s.case.vertical_min, 0.37);
        assert_eq!(s.case.vertical_max, 1.63);
        assert!(s.notes.is_empty());
        let tender = TransportCase::from_ship(&ShipMotion { gm: 0.1, ..Default::default() });
        assert!(tender.roll_period > 60.0 && tender.notes.len() == 1 && tender.notes[0].starts_with("tender"), "{:?}", tender.notes);
        // A large GM: short, violent roll and higher accelerations on deck.
        let stiff = TransportCase::from_ship(&ShipMotion { gm: 12.0, ..Default::default() });
        assert!(stiff.roll_period < 10.0 && stiff.notes[0].starts_with("stiff"), "{:?}", stiff.notes);
        assert!(stiff.case.sideways > s.case.sideways);
    }

    #[test]
    fn direct_lashings_follow_en_12195_1() {
        let l = LashingOptions::default();
        // 1000 kg, 0.8 g braking, μ 0.4: 4.905 kN over 10 kN · (0.3·sin 45° + cos 45°·cos 30°) = 8.245 kN.
        assert_eq!(l.against_sliding(1000.0, 0.8, 0.4, 1.0), 1);
        assert_eq!(l.against_sliding(3000.0, 0.8, 0.4, 1.0), 2);
        assert_eq!(l.against_sliding(1000.0, 0.3, 0.4, 1.0), 0, "friction holds");
        // 5 kN at a 600 mm lever, lashed at 1200 mm: 2.5 kN over 10·cos 45°·cos 30° = 6.12 kN.
        assert_eq!(l.against_tipping(5.0, 600.0, 1200.0), 1);
        assert_eq!(LashingOptions { anchor_dan: 250.0, ..l }.against_tipping(5.0, 600.0, 1200.0), 2);
    }

    #[test]
    fn partial_load_is_moved_to_meet_the_window() {
        let c = ContainerSpec::new("c", 100.0, 100.0, 1000.0);
        let opts = PackOptions::default();
        let mut p = vec![unit(0.0, 200.0, 10.0), unit(200.0, 200.0, 10.0)];
        let shift = centre_lengthwise(&c, &opts, &mut p);
        let w = weights(&p);
        // Centre of gravity within 5% of 1000 mm of the middle, by the shortest move.
        assert!((cog_z(&p, &w) - 450.0).abs() < 0.2, "{}", cog_z(&p, &w));
        assert!((shift - 250.0).abs() < 0.2, "{shift}");
        assert!(share_between(&p, &w, 250.0, 750.0, 0.0) >= 0.6);
    }
}
