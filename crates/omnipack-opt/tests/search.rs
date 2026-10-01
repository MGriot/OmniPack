//! The search must only ever return valid plans, never lose to the plain
//! placer, and be reproducible when capped by evaluations.

use omnipack_core::{generate, pack, PackRequest};
use omnipack_opt::{difference, optimize, OptimizeOptions, Score, MIN_DIFFERENCE};
use proptest::prelude::*;
use std::sync::atomic::AtomicBool;

fn capped(evals: usize) -> OptimizeOptions {
    OptimizeOptions { budget_ms: 600_000, max_evaluations: evals, population: 12, threads: 4, ..Default::default() }
}

fn run(req: &PackRequest, o: &OptimizeOptions) -> omnipack_opt::OptimizeResult {
    optimize(req, o, &AtomicBool::new(false), &mut |_| {}).unwrap()
}

#[test]
fn never_worse_than_the_plain_placer_and_always_valid() {
    let req = generate::mixed(1);
    let r = run(&req, &capped(80));
    let plain = Score::of(&pack(&req).unwrap(), &OptimizeOptions::default().objective);
    let best = &r.solutions[0];
    assert!(!plain.better_than(&best.score), "plain {plain:?} beats {:?}", best.score);
    assert!(r.solutions.iter().all(|s| s.result.is_valid()));
    for s in &r.solutions {
        let warnings: usize = s.result.containers.iter().map(|c| omnipack_core::balance::container_issues(&c.balance)).sum();
        assert_eq!(s.score.balance_issues, warnings);
    }
    for (i, a) in r.solutions.iter().enumerate() {
        for b in &r.solutions[i + 1..] {
            assert!(difference(&a.result, &b.result) >= MIN_DIFFERENCE);
        }
    }
    assert_eq!(r.evaluated, 80);
}

#[test]
fn capped_search_is_deterministic() {
    let mut req = generate::br_like(3, 1).unwrap();
    req.options.max_containers = 1;
    let a = run(&req, &capped(60));
    let b = run(&req, &capped(60));
    assert_eq!(a.solutions[0].score, b.solutions[0].score);
    assert_eq!(difference(&a.solutions[0].result, &b.solutions[0].result), 0.0);
}

#[test]
fn keeps_loading_order_constraints() {
    // Stops and zones: the decoder only reorders within a loading group.
    let mut req = generate::mixed(2);
    for (i, it) in req.items.iter_mut().enumerate() {
        it.stop = (i % 3) as u32;
    }
    let r = run(&req, &capped(50));
    let plain = pack(&req).unwrap();
    let stops = |res: &omnipack_core::PackResult| -> Vec<u32> { res.containers[0].placements.iter().map(|p| p.stop).collect() };
    // LIFO: stop 0 first, then later stops first — the stop sequence must be sorted the same way.
    let key = |s: u32| if s == 0 { i64::MIN } else { -(s as i64) };
    for res in [&plain, &r.solutions[0].result] {
        let s = stops(res);
        assert!(s.windows(2).all(|w| key(w[0]) <= key(w[1])), "{s:?}");
    }
}

#[test]
fn cancel_stops_early_with_a_plan() {
    let req = generate::mixed(3);
    let cancel = AtomicBool::new(true);
    let r = optimize(&req, &OptimizeOptions { budget_ms: 60_000, ..Default::default() }, &cancel, &mut |_| {}).unwrap();
    assert!(r.cancelled);
    assert!(!r.solutions.is_empty() && r.solutions[0].result.is_valid());
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 8, ..ProptestConfig::default() })]

    #[test]
    fn small_random_loads_give_valid_plans(class in 1usize..=7, seed in 1u64..50) {
        let mut req = generate::br_like(class, seed).unwrap();
        req.options.max_containers = 1;
        let r = run(&req, &capped(30));
        prop_assert!(r.solutions.iter().all(|s| s.result.is_valid()));
        prop_assert!(!r.baseline.better_than(&r.solutions[0].score));
    }
}

#[test]
fn fewer_tipping_units_beat_density() {
    let req = generate::mixed(1);
    let base = Score::of(&pack(&req).unwrap(), &OptimizeOptions::default().objective);
    let dense = Score { tipping_units: 1, value: base.value + 50.0, ..base.clone() };
    let safe = Score { tipping_units: 0, ..base };
    assert!(safe.better_than(&dense) && !dense.better_than(&safe));
}

#[test]
fn a_learned_ranker_joins_the_search() {
    let mut req = generate::br_like(2, 1).unwrap();
    req.options.max_containers = 1;
    assert!(!omnipack_opt::biases(&req).contains(&omnipack_core::FillBias::Learned));
    let weights = omnipack_core::placer::default_weights(&req.options);
    req.options.ranker = Some(omnipack_core::Ranker { weights, examples: 1, accuracy: 1.0 });
    assert!(omnipack_opt::biases(&req).contains(&omnipack_core::FillBias::Learned));
    let a = run(&req, &capped(40));
    let b = run(&req, &capped(40));
    assert_eq!(a.solutions[0].score, b.solutions[0].score);
    assert!(a.solutions.iter().all(|s| s.result.is_valid()));
}
