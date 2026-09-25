//! Instance generators and importers for tests and benchmarks.

use crate::model::*;
use omnipack_geom::{Orientation, Shape};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// kg per mm³ used to give benchmark boxes a plausible mass (150 kg/m³).
const DENSITY: f64 = 1.5e-7;

fn orientations_with_vertical(flags: [bool; 3]) -> Vec<Orientation> {
    Orientation::ALL
        .into_iter()
        .filter(|o| flags[o.axis_map()[1]])
        .collect()
}

/// A Bischoff & Ratcliff–style instance (classes BR1–BR7: 3, 5, 8, 10, 12, 15,
/// 20 box types) in the classic 587 × 233 × 220 container, filled with at
/// least one container volume of boxes. Statistically similar to, not
/// identical to, the published OR-Library instances; use [`parse_thpack`] for
/// those.
pub fn br_like(class: usize, seed: u64) -> Option<PackRequest> {
    let types = *[3usize, 5, 8, 10, 12, 15, 20].get(class.checked_sub(1)?)?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_mul(1000).wrapping_add(class as u64));
    let container = ContainerSpec {
        id: format!("BR{class}-{seed}"),
        width: 233.0,
        height: 220.0,
        depth: 587.0,
        max_payload: None,
        axles: None,
        cog_limits: CogLimits::default(),
    };
    let mut items: Vec<ItemSpec> = (0..types)
        .map(|t| {
            let (l, w, h) = (
                rng.gen_range(30..=120) as f64,
                rng.gen_range(25..=100) as f64,
                rng.gen_range(20..=80) as f64,
            );
            let mut flags = [rng.gen_bool(0.7), rng.gen_bool(0.7), rng.gen_bool(0.7)];
            if !flags.iter().any(|f| *f) {
                flags[1] = true;
            }
            let shape = Shape::Box { w, h, d: l };
            ItemSpec {
                id: format!("t{}", t + 1),
                mass: shape.volume() * DENSITY,
                shape,
                quantity: 0,
                max_load_on_top: None,
                fragile: false,
                floor_only: false,
                upright_only: false,
                allowed_orientations: Some(orientations_with_vertical(flags)),
                stop: 0,
                zone: Zone::Any,
                com_offset: [0.0; 3],
                color: None,
                friction: None,
            }
        })
        .collect();
    let target = container.volume();
    let mut vol = 0.0;
    while vol < target {
        let t = rng.gen_range(0..types);
        items[t].quantity += 1;
        vol += items[t].shape.volume();
    }
    items.retain(|i| i.quantity > 0);
    Some(PackRequest {
        container,
        items,
        options: PackOptions::default(),
    })
}

/// Parses problem `n` (1-based) from an OR-Library `thpack` file.
/// Format: `P`, then per problem: `id seed`, `L W H`, `types`, and one line
/// per type `i l fl w fw h fh qty` (flag = may stand vertically).
pub fn parse_thpack(text: &str, n: usize) -> Result<PackRequest, String> {
    let mut nums = text
        .split_whitespace()
        .map(|t| t.parse::<f64>().map_err(|e| format!("{t}: {e}")));
    let mut next = || {
        nums.next()
            .ok_or_else(|| "unexpected end of file".to_string())?
    };
    let problems = next()? as usize;
    if n == 0 || n > problems {
        return Err(format!("problem {n} out of range 1..={problems}"));
    }
    for p in 1..=n {
        let (_id, _seed) = (next()?, next()?);
        let (l, w, h) = (next()?, next()?, next()?);
        let types = next()? as usize;
        let mut items = Vec::with_capacity(types);
        for _ in 0..types {
            let t = next()? as usize;
            let (bl, fl, bw, fw, bh, fh, qty) = (
                next()?,
                next()?,
                next()?,
                next()?,
                next()?,
                next()?,
                next()?,
            );
            let shape = Shape::Box {
                w: bw,
                h: bh,
                d: bl,
            };
            items.push(ItemSpec {
                id: format!("t{t}"),
                mass: shape.volume() * DENSITY,
                shape,
                quantity: qty as u32,
                max_load_on_top: None,
                fragile: false,
                floor_only: false,
                upright_only: false,
                allowed_orientations: Some(orientations_with_vertical([
                    fw != 0.0,
                    fh != 0.0,
                    fl != 0.0,
                ])),
                stop: 0,
                zone: Zone::Any,
                com_offset: [0.0; 3],
                color: None,
                friction: None,
            });
        }
        if p == n {
            let container = ContainerSpec {
                id: format!("thpack-{n}"),
                width: w,
                height: h,
                depth: l,
                max_payload: None,
                axles: None,
                cog_limits: CogLimits::default(),
            };
            return Ok(PackRequest {
                container,
                items,
                options: PackOptions::default(),
            });
        }
    }
    unreachable!()
}

/// One of every shape kind in a 20 ft container: prisms, pyramids, cones,
/// balls, angle profiles, drums and cartons, over two delivery stops.
pub fn shapes(seed: u64) -> PackRequest {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut q = |lo: u32, hi: u32| rng.gen_range(lo..=hi);
    let item = |id: &str, shape: Shape, mass: f64, quantity: u32, color: &str| ItemSpec {
        id: id.into(),
        shape,
        mass,
        quantity,
        max_load_on_top: None,
        fragile: false,
        floor_only: false,
        upright_only: false,
        allowed_orientations: None,
        stop: 0,
        zone: Zone::Any,
        com_offset: [0.0; 3],
        color: Some(color.into()),
        friction: None,
    };
    let items = vec![
        ItemSpec {
            stop: 2,
            max_load_on_top: Some(800.0),
            ..item(
                "crate",
                Shape::Box {
                    w: 1000.0,
                    h: 800.0,
                    d: 800.0,
                },
                250.0,
                q(3, 5),
                "#4e79a7",
            )
        },
        ItemSpec {
            stop: 2,
            ..item(
                "beam",
                Shape::Prism {
                    sides: 3,
                    radius: 250.0,
                    length: 2000.0,
                },
                120.0,
                q(2, 4),
                "#f28e2b",
            )
        },
        ItemSpec {
            stop: 1,
            max_load_on_top: Some(200.0),
            ..item(
                "hex-bar",
                Shape::Prism {
                    sides: 6,
                    radius: 150.0,
                    length: 1500.0,
                },
                80.0,
                q(3, 6),
                "#59a14f",
            )
        },
        ItemSpec {
            stop: 1,
            fragile: true,
            upright_only: true,
            ..item(
                "pyramid",
                Shape::Pyramid {
                    w: 600.0,
                    d: 600.0,
                    height: 500.0,
                },
                40.0,
                q(2, 4),
                "#e15759",
            )
        },
        ItemSpec {
            stop: 1,
            fragile: true,
            ..item(
                "cone",
                Shape::Cone {
                    radius: 250.0,
                    height: 700.0,
                },
                8.0,
                q(4, 8),
                "#76b7b2",
            )
        },
        ItemSpec {
            stop: 2,
            fragile: true,
            ..item(
                "ball",
                Shape::Sphere { radius: 220.0 },
                15.0,
                q(3, 6),
                "#edc948",
            )
        },
        ItemSpec {
            stop: 2,
            max_load_on_top: Some(300.0),
            ..item(
                "angle",
                Shape::LProfile {
                    a: 300.0,
                    b: 300.0,
                    thickness: 30.0,
                    length: 2400.0,
                },
                60.0,
                q(3, 6),
                "#b07aa1",
            )
        },
        ItemSpec {
            stop: 1,
            upright_only: true,
            max_load_on_top: Some(400.0),
            ..item(
                "drum",
                Shape::Cylinder {
                    radius: 290.0,
                    length: 880.0,
                },
                200.0,
                q(3, 6),
                "#9c755f",
            )
        },
        ItemSpec {
            stop: 1,
            max_load_on_top: Some(150.0),
            ..item(
                "carton",
                Shape::Box {
                    w: 400.0,
                    h: 300.0,
                    d: 300.0,
                },
                10.0,
                q(10, 20),
                "#ff9da7",
            )
        },
    ];
    PackRequest {
        container: ContainerSpec {
            id: "20ft".into(),
            width: 2350.0,
            height: 2390.0,
            depth: 5900.0,
            max_payload: Some(28000.0),
            axles: None,
            cog_limits: CogLimits::default(),
        },
        items,
        options: PackOptions::default(),
    }
}

/// A realistic mixed truck load: pallets, fragile cartons, upright drums,
/// lying pipe rolls, off-centre machinery, three delivery stops, axle limits.
pub fn mixed(seed: u64) -> PackRequest {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut q = |lo: u32, hi: u32| rng.gen_range(lo..=hi);
    let item = |id: &str, shape: Shape, mass: f64, quantity: u32| ItemSpec {
        id: id.into(),
        shape,
        mass,
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
    };
    let items = vec![
        ItemSpec {
            upright_only: true,
            max_load_on_top: Some(1500.0),
            stop: 3,
            color: Some("#4e79a7".into()),
            ..item(
                "pallet",
                Shape::Box {
                    w: 1200.0,
                    h: 1000.0,
                    d: 800.0,
                },
                450.0,
                q(4, 8),
            )
        },
        ItemSpec {
            fragile: true,
            stop: 1,
            color: Some("#f28e2b".into()),
            ..item(
                "glass",
                Shape::Box {
                    w: 600.0,
                    h: 500.0,
                    d: 400.0,
                },
                25.0,
                q(4, 10),
            )
        },
        ItemSpec {
            upright_only: true,
            max_load_on_top: Some(300.0),
            stop: 2,
            color: Some("#59a14f".into()),
            ..item(
                "drum",
                Shape::Cylinder {
                    radius: 290.0,
                    length: 880.0,
                },
                180.0,
                q(4, 8),
            )
        },
        ItemSpec {
            max_load_on_top: Some(400.0),
            stop: 2,
            color: Some("#b07aa1".into()),
            ..item(
                "pipe",
                Shape::Cylinder {
                    radius: 150.0,
                    length: 1800.0,
                },
                90.0,
                q(3, 6),
            )
        },
        ItemSpec {
            upright_only: true,
            floor_only: true,
            max_load_on_top: Some(0.0),
            stop: 3,
            com_offset: [250.0, -100.0, 0.0],
            color: Some("#e15759".into()),
            ..item(
                "machine",
                Shape::Box {
                    w: 1400.0,
                    h: 1300.0,
                    d: 900.0,
                },
                900.0,
                q(1, 2),
            )
        },
        ItemSpec {
            max_load_on_top: Some(200.0),
            stop: 1,
            color: Some("#edc948".into()),
            ..item(
                "carton",
                Shape::Box {
                    w: 400.0,
                    h: 300.0,
                    d: 300.0,
                },
                12.0,
                q(10, 30),
            )
        },
    ];
    PackRequest {
        container: ContainerSpec {
            id: "truck".into(),
            width: 2400.0,
            height: 2500.0,
            depth: 7000.0,
            max_payload: Some(12000.0),
            axles: Some([
                Axle {
                    z: 800.0,
                    max_load: 8000.0,
                },
                Axle {
                    z: 6000.0,
                    max_load: 9000.0,
                },
            ]),
            cog_limits: CogLimits {
                max_lateral_offset: Some(300.0),
                ..Default::default()
            },
        },
        items,
        options: PackOptions::default(),
    }
}
