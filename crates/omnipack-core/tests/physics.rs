//! Physics regression cases: hand-built plans the validator must reject or
//! accept, and packer behaviour on classic stability traps.

use omnipack_core::validate::{self, validate};
use omnipack_core::*;
use omnipack_geom::{Orientation, Shape};

fn container() -> ContainerSpec {
    ContainerSpec {
        id: "c".into(),
        width: 1000.0,
        height: 1000.0,
        depth: 1000.0,
        max_payload: None,
        axles: None,
        cog_limits: CogLimits::default(),
    }
}

fn spec(id: &str, shape: Shape, mass: f64) -> ItemSpec {
    ItemSpec {
        id: id.into(),
        shape,
        mass,
        quantity: 1,
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

fn place(item: &ItemSpec, seq: usize, pos: [f64; 3]) -> Placement {
    let size = item.shape.local_extents();
    Placement {
        instance_id: format!("{}#{seq}", item.id),
        item_id: item.id.clone(),
        seq,
        shape: item.shape.clone(),
        orientation: Orientation::Whd,
        position: pos,
        size,
        center_of_mass: [pos[0] + size[0] / 2.0, pos[1] + size[1] / 2.0, pos[2] + size[2] / 2.0],
        mass: item.mass,
        load_on_top: 0.0,
        support_margin: 0.0,
        stop: 0,
        color: None,
        needs_chocks: false,
    }
}

fn opts() -> PackOptions {
    PackOptions { stability_margin: 0.0, min_support_ratio: 0.0, ..Default::default() }
}

#[test]
fn overhang_past_centre_of_gravity_tips() {
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0);
    // Upper box shifted 60 mm: its CoG (x=110) is beyond the lower edge (x=100).
    let plan = [place(&b, 0, [0.0, 0.0, 0.0]), place(&b, 1, [60.0, 100.0, 0.0])];
    let v = validate(&container(), std::slice::from_ref(&b), &opts(), &plan);
    assert!(v.iter().any(|v| matches!(v, Violation::Unstable { .. })), "{v:?}");
    // Shifted 40 mm it stands.
    let plan = [place(&b, 0, [0.0, 0.0, 0.0]), place(&b, 1, [40.0, 100.0, 0.0])];
    assert!(validate(&container(), &[b], &opts(), &plan).is_empty());
}

#[test]
fn floating_item_is_unsupported() {
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0);
    let plan = [place(&b, 0, [0.0, 50.0, 0.0])];
    let v = validate(&container(), &[b], &opts(), &plan);
    assert!(v.iter().any(|v| matches!(v, Violation::Unsupported { .. })), "{v:?}");
}

#[test]
fn load_propagates_through_the_whole_stack() {
    // Bottom box tolerates 15 kg on top; two 10 kg boxes above it = 20 kg.
    let bottom = ItemSpec { max_load_on_top: Some(15.0), ..spec("bottom", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0) };
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0);
    let plan = [place(&bottom, 0, [0.0; 3]), place(&b, 1, [0.0, 100.0, 0.0]), place(&b, 2, [0.0, 200.0, 0.0])];
    let v = validate(&container(), &[bottom.clone(), b.clone()], &opts(), &plan);
    assert!(
        v.iter().any(|v| matches!(v, Violation::Overloaded { item, load, .. } if item == "bottom#0" && (*load - 20.0).abs() < 1e-6)),
        "{v:?}"
    );
}

#[test]
fn eccentric_bridge_loads_nearer_support_more() {
    // A plank across two blocks with a heavy box near the left block.
    let block = ItemSpec { max_load_on_top: Some(1e9), ..spec("block", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 1.0) };
    let plank = spec("plank", Shape::Box { w: 600.0, h: 20.0, d: 100.0 }, 0.0);
    let heavy = spec("heavy", Shape::Box { w: 50.0, h: 50.0, d: 50.0 }, 100.0);
    let plan = [
        place(&block, 0, [0.0, 0.0, 0.0]),
        place(&block, 1, [500.0, 0.0, 0.0]),
        place(&plank, 2, [0.0, 100.0, 0.0]),
        place(&heavy, 3, [100.0, 120.0, 25.0]),
    ];
    let specs = [block, plank, heavy];
    assert!(validate(&container(), &specs, &opts(), &plan).is_empty());
    // Heavy CoG at x=125. Elastic-foundation solution over the 8 contact
    // corners (x = 0, 100 | 500, 600): 83.7 kg left, 16.3 kg right.
    // A 60 kg limit must therefore trip only the left block.
    let mut limited = specs.clone();
    limited[0].max_load_on_top = Some(60.0);
    let v = validate(&container(), &limited, &opts(), &plan);
    let overloaded: Vec<(&str, f64)> = v
        .iter()
        .filter_map(|v| match v {
            Violation::Overloaded { item, load, .. } => Some((item.as_str(), *load)),
            _ => None,
        })
        .collect();
    assert_eq!(overloaded.len(), 1, "{v:?}");
    assert_eq!(overloaded[0].0, "block#0");
    assert!((overloaded[0].1 - 83.7).abs() < 0.2, "{overloaded:?}");
}

#[test]
fn nothing_stands_on_fragile_items() {
    let glass = ItemSpec { fragile: true, ..spec("glass", Shape::Box { w: 1000.0, h: 100.0, d: 1000.0 }, 5.0) };
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 1.0);
    let req = PackRequest {
        container: ContainerSpec { height: 300.0, ..container() },
        items: vec![glass, ItemSpec { quantity: 3, ..b }],
        options: PackOptions { max_containers: 1, ..Default::default() },
    };
    let res = pack(&req).unwrap();
    // The glass covers the whole floor, so nothing else can be loaded.
    assert_eq!(res.packed_units, 1);
    assert_eq!(res.unpacked.len(), 3);
    assert!(res.unpacked.iter().all(|u| u.reason == UnpackReason::ContainerLimit));
}

#[test]
fn lying_cylinder_needs_chocks_or_a_groove() {
    let pipe = spec("pipe", Shape::Cylinder { radius: 50.0, length: 400.0 }, 20.0);
    let pipe_lying = |seq, pos| Placement { orientation: Orientation::Hwd, size: [400.0, 100.0, 100.0], ..place(&pipe, seq, pos) };
    let fix_com = |mut p: Placement| {
        p.center_of_mass = [p.position[0] + 200.0, p.position[1] + 50.0, p.position[2] + 50.0];
        p
    };
    let no_chocks = PackOptions { physics: PhysicsOptions { use_chocks: false, ..Default::default() }, ..opts() };
    // Alone in the middle of the floor: free to roll unless wedges are used.
    let plan = [fix_com(pipe_lying(0, [0.0, 0.0, 400.0]))];
    let v = validate(&container(), std::slice::from_ref(&pipe), &no_chocks, &plan);
    assert!(v.iter().any(|v| matches!(v, Violation::MayRoll { .. })), "{v:?}");
    assert!(validate(&container(), std::slice::from_ref(&pipe), &opts(), &plan).is_empty());
    assert_eq!(validate::needs_chocks(&container(), std::slice::from_ref(&pipe), &opts(), &plan), vec![true]);
    // Wedged between the back wall and a neighbour: fine without chocks.
    let plan = [fix_com(pipe_lying(0, [0.0, 0.0, 0.0])), fix_com(pipe_lying(1, [0.0, 0.0, 100.0]))];
    let v = validate(&container(), std::slice::from_ref(&pipe), &no_chocks, &plan);
    assert!(!v.iter().any(|v| matches!(v, Violation::MayRoll { item } if item == "pipe#0")), "{v:?}");
}

#[test]
fn axle_and_payload_limits_are_checked() {
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 600.0);
    let c = ContainerSpec {
        max_payload: Some(1000.0),
        axles: Some([Axle { z: 0.0, max_load: 700.0 }, Axle { z: 1000.0, max_load: 700.0 }]),
        ..container()
    };
    // Two boxes at the back: 1200 kg, almost all on axle 0.
    let plan = [place(&b, 0, [0.0; 3]), place(&b, 1, [100.0, 0.0, 0.0])];
    let v = validate(&c, &[b], &opts(), &plan);
    assert!(v.iter().any(|v| matches!(v, Violation::PayloadExceeded { .. })));
    assert!(v.iter().any(|v| matches!(v, Violation::AxleOverloaded { axle: 0, .. })));
}

fn road() -> PackOptions {
    PackOptions { physics: PhysicsOptions { secure_load_end: false, ..Default::default() }, ..opts() }
}

fn issues(plan: &[Placement], specs: &[ItemSpec], o: &PackOptions) -> Vec<TransportIssue> {
    validate::transport(&container(), specs, o, plan).into_iter().flat_map(|r| r.issues).collect()
}

#[test]
fn free_standing_tall_item_slides_and_tips_under_braking() {
    let tall = spec("tall", Shape::Box { w: 200.0, h: 1000.0, d: 200.0 }, 100.0);
    let plan = [place(&tall, 0, [400.0, 0.0, 400.0])];
    let found = issues(&plan, std::slice::from_ref(&tall), &road());
    let has = |k: IssueKind, d: Direction| found.iter().any(|i| i.kind == k && i.direction == Some(d));
    // mu 0.4 < 0.8 g forward; lever 500 mm vs arm 100 mm.
    assert!(has(IssueKind::Sliding, Direction::Forward), "{found:?}");
    assert!(has(IssueKind::Tipping, Direction::Forward), "{found:?}");
    assert!(has(IssueKind::Tipping, Direction::Left), "{found:?}");
    let slide = found.iter().find(|i| i.kind == IssueKind::Sliding && i.direction == Some(Direction::Forward)).unwrap();
    // 100 kg * 9.81 * (0.8 - 0.4) = 392.4 N
    assert!((slide.required - 0.3924).abs() < 1e-3, "{slide:?}");
}

#[test]
fn walls_and_neighbours_block_sliding() {
    // A full-width row against the front wall: blocked forward and sideways.
    let b = spec("b", Shape::Box { w: 500.0, h: 200.0, d: 300.0 }, 50.0);
    let plan = [place(&b, 0, [0.0, 0.0, 0.0]), place(&b, 1, [500.0, 0.0, 0.0])];
    let found = issues(&plan, std::slice::from_ref(&b), &road());
    assert!(!found.iter().any(|i| i.kind == IssueKind::Sliding && i.direction != Some(Direction::Backward)), "{found:?}");
    // Nothing behind them: 0.5 g backward > mu 0.4.
    assert_eq!(found.iter().filter(|i| i.kind == IssueKind::Sliding && i.direction == Some(Direction::Backward)).count(), 2);
    // Low and wide: no tipping anywhere.
    assert!(!found.iter().any(|i| i.kind == IssueKind::Tipping), "{found:?}");
    // A locking bar across the load end blocks them backward too.
    let barred = PackOptions { physics: PhysicsOptions::default(), ..opts() };
    assert!(issues(&plan, std::slice::from_ref(&b), &barred).is_empty());
}

#[test]
fn sea_motion_overloads_stacks_dynamically() {
    let bottom = ItemSpec { max_load_on_top: Some(15.0), ..spec("bottom", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0) };
    let b = spec("b", Shape::Box { w: 100.0, h: 100.0, d: 100.0 }, 10.0);
    let plan = [place(&bottom, 0, [0.0; 3]), place(&b, 1, [0.0, 100.0, 0.0])];
    let specs = [bottom, b];
    let sea_c = TransportCase::presets().into_iter().find(|c| c.name.starts_with("Sea area C")).unwrap();
    let o = PackOptions { physics: PhysicsOptions { transport: vec![sea_c], dynamic_stacking: true, ..Default::default() }, ..opts() };
    // Static: 10 kg on a 15 kg limit is fine; at 1.8 g it is 18 kg.
    assert!(validate(&container(), &specs, &o, &plan).is_empty());
    let found = issues(&plan, &specs, &o);
    let over = found.iter().find(|i| i.kind == IssueKind::StackOverload).expect("overload");
    assert!((over.required - 3.0).abs() < 1e-6, "{over:?}");
}
