//! Hand-made plans: gravity drop, loading order, flagged (not refused)
//! problems, and auto-fill around fixed units.

use omnipack_core::manual::{evaluate, place, probe, remaining};
use omnipack_core::{learn, placer};
use omnipack_core::*;
use omnipack_geom::{Orientation, Shape};

fn cube(id: &str, quantity: u32) -> ItemSpec {
    ItemSpec {
        id: id.into(),
        shape: Shape::Box { w: 200.0, h: 200.0, d: 200.0 },
        mass: 20.0,
        quantity,
        max_load_on_top: None,
        fragile: false,
        floor_only: false,
        upright_only: false,
        allowed_orientations: None,
        stop: 0,
        zone: Zone::Any,
        com_offset: [0.0; 3],
        color: None,
        friction: None,
    }
}

fn request(items: Vec<ItemSpec>) -> PackRequest {
    PackRequest { container: ContainerSpec::new("c", 1000.0, 1000.0, 1000.0), items, options: PackOptions::default() }
}

#[test]
fn units_drop_onto_what_is_below() {
    let req = request(vec![cube("a", 2)]);
    let first = place(&req, &[], "a", Orientation::Whd, 100.0, 100.0, None, None).unwrap();
    assert_eq!(first.position, [100.0, 0.0, 100.0]);
    assert_eq!(first.instance_id, "a#1");
    let second = place(&req, std::slice::from_ref(&first), "a", Orientation::Whd, 100.0, 100.0, None, None).unwrap();
    assert!((second.position[1] - 200.0).abs() < 1e-6, "{:?}", second.position);
    assert_eq!(second.instance_id, "a#2");
    let plan = evaluate(&req, vec![first.clone(), second]);
    assert!(plan.violations.is_empty(), "{:?}", plan.violations);
    assert!((plan.placements[0].load_on_top - 20.0).abs() < 1e-6, "the lower cube carries the upper one");
    assert_eq!(remaining(&req, &plan.placements), vec![("a".to_string(), 0)]);
}

#[test]
fn loading_order_follows_the_heights() {
    // The upper cube is placed first, in the air; the lower one is slid under it later.
    let req = request(vec![cube("a", 2)]);
    let upper = place(&req, &[], "a", Orientation::Whd, 0.0, 0.0, Some(200.0), None).unwrap();
    let lower = place(&req, std::slice::from_ref(&upper), "a", Orientation::Whd, 0.0, 0.0, Some(0.0), None).unwrap();
    let plan = evaluate(&req, vec![upper, lower]);
    assert!(plan.violations.is_empty(), "{:?}", plan.violations);
    assert!(plan.placements[0].position[1] < 1.0 && plan.placements[0].seq == 0);
}

#[test]
fn problems_are_flagged_not_refused() {
    let req = request(vec![cube("a", 2)]);
    let a = place(&req, &[], "a", Orientation::Whd, 0.0, 0.0, None, None).unwrap();
    // Forced into the same spot: overlapping.
    let p = probe(&req, std::slice::from_ref(&a), "a", Orientation::Whd, 50.0, 0.0, Some(0.0), None).unwrap();
    assert!(p.problems.iter().any(|v| matches!(v, Violation::Overlap { .. })), "{:?}", p.problems);
    // Floating in the middle of the container.
    let f = probe(&req, std::slice::from_ref(&a), "a", Orientation::Whd, 600.0, 600.0, Some(300.0), None).unwrap();
    assert!(f.problems.iter().any(|v| matches!(v, Violation::Unsupported { .. })), "{:?}", f.problems);
    let plan = evaluate(&req, vec![a, f.placement]);
    assert!(plan.violations.iter().any(|v| matches!(v, Violation::Unsupported { .. })));
    assert_eq!(plan.placements.len(), 2, "kept in the plan");
}

#[test]
fn moving_a_unit_keeps_its_id() {
    let req = request(vec![cube("a", 2)]);
    let a = place(&req, &[], "a", Orientation::Whd, 0.0, 0.0, None, None).unwrap();
    let b = place(&req, std::slice::from_ref(&a), "a", Orientation::Whd, 400.0, 0.0, None, None).unwrap();
    let moved = place(&req, &[a.clone(), b.clone()], "a", Orientation::Whd, 0.0, 0.0, None, Some("a#2")).unwrap();
    assert_eq!(moved.instance_id, "a#2");
    assert!((moved.position[1] - 200.0).abs() < 1e-6, "lands on a#1: {:?}", moved.position);
}

#[test]
fn auto_fill_keeps_the_fixed_units() {
    let req = request(vec![cube("a", 10)]);
    let a = place(&req, &[], "a", Orientation::Whd, 400.0, 0.0, None, None).unwrap();
    let b = place(&req, std::slice::from_ref(&a), "a", Orientation::Whd, 400.0, 400.0, None, None).unwrap();
    let fixed = evaluate(&req, vec![a, b]).placements;
    let r = pack_with_fixed(&req, &fixed).unwrap();
    assert!(r.is_valid(), "{:?}", r.containers[0].violations);
    assert_eq!(r.packed_units, 10);
    for f in &fixed {
        let p = r.containers[0].placements.iter().find(|p| p.instance_id == f.instance_id).unwrap();
        assert_eq!(p.position, f.position, "{} moved", f.instance_id);
    }
}

#[test]
fn nothing_is_stacked_on_a_broken_fixed_unit() {
    let req = request(vec![cube("a", 30)]);
    // Floating in mid-air: unsupported.
    let broken = place(&req, &[], "a", Orientation::Whd, 0.0, 0.0, Some(300.0), None).unwrap();
    let r = pack_with_fixed(&req, std::slice::from_ref(&broken)).unwrap();
    let plan = &r.containers[0];
    assert!(plan.violations.iter().all(|v| matches!(v, Violation::Unsupported { item } if item == "a#1")), "{:?}", plan.violations);
    assert!(plan.violations.iter().any(|v| matches!(v, Violation::Unsupported { .. })));
    let top = broken.position[1] + broken.size[1];
    let on_it = plan.placements.iter().filter(|p| p.instance_id != "a#1" && (p.position[1] - top).abs() < 0.1 && p.position[0] < 200.0 && p.position[2] < 200.0);
    assert_eq!(on_it.count(), 0);
}

#[test]
fn auto_filled_plans_keep_the_placer_order() {
    // Like the app: one unit by hand, the rest auto-filled, then re-checked as a manual plan.
    let req = request(vec![cube("a", 20)]);
    let fixed = evaluate(&req, vec![place(&req, &[], "a", Orientation::Whd, 400.0, 0.0, None, None).unwrap()]).placements;
    let filled = pack_with_fixed(&req, &fixed).unwrap().containers.remove(0);
    let plan = evaluate(&req, filled.placements.clone());
    assert!(plan.violations.is_empty());
    let order = |c: &ContainerPlan| c.placements.iter().map(|p| p.instance_id.clone()).collect::<Vec<_>>();
    assert_eq!(order(&plan), order(&filled));
    // So replaying it matches the placer's own decisions (apart from the hand-placed one).
    let steps = learn::examples(&req, &plan);
    let w = placer::default_weights(&req.options);
    assert!(learn::top1(&steps, &w) > 0.9, "{}", learn::top1(&steps, &w));
}
