//! Property tests: every plan the packer returns must be physically valid,
//! at the end and after every loading step, and must account for every unit.

use omnipack_core::validate::validate;
use omnipack_core::*;
use omnipack_geom::Shape;
use proptest::prelude::*;

fn shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        4 => (20.0..120.0f64, 20.0..120.0f64, 20.0..120.0f64).prop_map(|(w, h, d)| Shape::Box { w, h, d }),
        1 => (10.0..50.0f64, 20.0..150.0f64).prop_map(|(radius, length)| Shape::Cylinder { radius, length }),
        1 => (10.0..45.0f64).prop_map(|radius| Shape::Sphere { radius }),
        1 => (10.0..50.0f64, 20.0..100.0f64).prop_map(|(radius, height)| Shape::Cone { radius, height }),
        1 => (20.0..100.0f64, 20.0..100.0f64, 20.0..90.0f64).prop_map(|(w, d, height)| Shape::Pyramid { w, d, height }),
        1 => (3u32..9, 15.0..50.0f64, 20.0..150.0f64).prop_map(|(sides, radius, length)| Shape::Prism { sides, radius, length }),
        1 => (30.0..100.0f64, 30.0..100.0f64, 4.0..15.0f64, 40.0..150.0f64)
            .prop_map(|(a, b, thickness, length)| Shape::LProfile { a, b, thickness, length }),
    ]
}

fn item(i: usize) -> impl Strategy<Value = ItemSpec> {
    (
        shape(),
        0.0..50.0f64,
        1u32..6,
        prop::option::of(0.0..200.0f64),
        prop::bool::weighted(0.1),
        prop::bool::weighted(0.2),
        0u32..4,
        prop::array::uniform3(-5.0..5.0f64),
    )
        .prop_map(move |(shape, mass, quantity, max_load_on_top, fragile, upright_only, stop, com_offset)| ItemSpec {
            id: format!("i{i}"),
            shape,
            mass,
            quantity,
            max_load_on_top,
            fragile,
            floor_only: false,
            upright_only,
            allowed_orientations: None,
            stop,
            zone: Zone::Any,
            com_offset,
            color: None,
            friction: None,
        })
}

fn request() -> impl Strategy<Value = PackRequest> {
    (
        (1usize..8).prop_flat_map(|n| (0..n).map(item).collect::<Vec<_>>()),
        prop_oneof![
            Just(FillBias::WallBuilding),
            Just(FillBias::FloorFirst),
            Just(FillBias::Longitudinal),
            Just(FillBias::Lateral),
            Just(FillBias::CornerFirst)
        ],
        0.0..0.4f64,
        0.0..0.8f64,
        (
            prop_oneof![Just(StopOrder::Lifo), Just(StopOrder::Fifo)],
            prop_oneof![
                Just(LoadPriority::Volume),
                Just(LoadPriority::Mass),
                Just(LoadPriority::BaseArea),
                Just(LoadPriority::Height),
                Just(LoadPriority::AsListed)
            ],
            any::<bool>(),
        ),
    )
        .prop_map(|(items, bias, stability_margin, min_support_ratio, (stop_order, priority, use_chocks))| PackRequest {
            container: ContainerSpec {
                id: "c".into(),
                width: 233.0,
                height: 220.0,
                depth: 400.0,
                max_payload: None,
                axles: None,
                cog_limits: CogLimits::default(),
            },
            items,
            options: PackOptions {
                bias,
                stability_margin,
                min_support_ratio,
                stop_order,
                priority,
                physics: PhysicsOptions { use_chocks, ..Default::default() },
                max_containers: 3,
                ..Default::default()
            },
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn plans_are_physically_valid(req in request()) {
        let res = pack(&req).unwrap();
        let requested: u32 = req.items.iter().map(|i| i.quantity).sum();
        prop_assert_eq!(res.requested_units, requested as usize);
        prop_assert_eq!(res.packed_units + res.unpacked.len(), requested as usize);
        for c in &res.containers {
            if !c.violations.is_empty() {
                // Keep the failing case for replay with `omnipack pack`.
                let _ = std::fs::write(
                    concat!(env!("CARGO_TARGET_TMPDIR"), "/failing-request.json"),
                    serde_json::to_string_pretty(&req).unwrap(),
                );
            }
            prop_assert!(c.violations.is_empty(), "{:?}", c.violations);
            // Every intermediate loading state is valid too.
            for k in 1..c.placements.len() {
                let v = validate(&req.container, &req.items, &req.options, &c.placements[..k]);
                prop_assert!(v.is_empty(), "after {} items: {:?}", k, v);
            }
        }
    }

    #[test]
    fn packing_is_deterministic(req in request()) {
        let a = serde_json::to_string(&pack(&req).unwrap().containers).unwrap();
        let b = serde_json::to_string(&pack(&req).unwrap().containers).unwrap();
        prop_assert_eq!(a, b);
    }
}

#[test]
fn mixed_truck_load_is_valid() {
    for seed in 1..=5 {
        let res = pack(&generate::mixed(seed)).unwrap();
        assert!(res.is_valid(), "seed {seed}: {:?}", res.containers[0].violations);
        assert!(res.unpacked.is_empty(), "seed {seed}: {:?}", res.unpacked);
    }
}

#[test]
fn shapes_sample_is_valid() {
    for seed in 1..=3 {
        let res = pack(&generate::shapes(seed)).unwrap();
        assert!(res.is_valid(), "seed {seed}: {:?}", res.containers.iter().map(|c| &c.violations).collect::<Vec<_>>());
        assert!(res.packed_units > 0);
    }
}

#[test]
fn br_like_instances_are_valid() {
    for class in 1..=7 {
        let mut req = generate::br_like(class, 1).unwrap();
        req.options.max_containers = 1;
        let res = pack(&req).unwrap();
        assert!(res.is_valid(), "BR{class}");
        assert!(res.containers[0].metrics.volume_utilization > 0.6, "BR{class} too sparse");
    }
}
