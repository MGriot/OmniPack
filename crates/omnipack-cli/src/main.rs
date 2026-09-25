//! `omnipack` command-line tool.
//!
//! ```text
//! omnipack pack <request.json> [-o plan.json]
//! omnipack gen br <class 1-7> <seed> [-o request.json]
//! omnipack gen mixed|shapes <seed> [-o request.json]
//! omnipack thpack <thpackN.txt> <problem 1..> [-o request.json]
//! omnipack bench [instances-per-class]
//! ```

use omnipack_core::generate;

use omnipack_core::{pack, PackRequest, PackResult};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  omnipack pack <request.json> [-o plan.json]\n  omnipack gen br <class 1-7> <seed> [-o request.json]\n  \
         omnipack gen mixed <seed> [-o request.json]\n  omnipack thpack <thpackN.txt> <problem> [-o request.json]\n  \
         omnipack bench [instances-per-class]"
    );
    ExitCode::from(2)
}

fn take_output(args: &mut Vec<String>) -> Option<String> {
    let i = args.iter().position(|a| a == "-o")?;
    let path = args.get(i + 1).cloned();
    args.drain(i..(i + 2).min(args.len()));
    path
}

fn write_json<T: serde::Serialize>(value: &T, out: Option<String>) -> Result<(), String> {
    let s = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    match out {
        Some(p) => std::fs::write(&p, s).map_err(|e| format!("{p}: {e}")),
        None => {
            println!("{s}");
            Ok(())
        }
    }
}

fn summary(r: &PackResult) -> String {
    let mut s = format!(
        "{} / {} units packed in {} container(s), volume utilization {:.1}%, {} ms, {}",
        r.packed_units,
        r.requested_units,
        r.containers.len(),
        r.volume_utilization * 100.0,
        r.elapsed_ms,
        if r.is_valid() {
            "all checks passed"
        } else {
            "VIOLATIONS FOUND"
        }
    );
    for c in &r.containers {
        let m = &c.metrics;
        s += &format!(
            "\n  {}: {} items, {:.1}% vol, {:.0} kg, CoG ({:.0}, {:.0}, {:.0}), accessibility {:.0}%, min margin {:.1} mm",
            c.id,
            m.item_count,
            m.volume_utilization * 100.0,
            m.total_mass,
            m.center_of_mass[0],
            m.center_of_mass[1],
            m.center_of_mass[2],
            m.accessibility * 100.0,
            m.min_support_margin
        );
        for v in &c.violations {
            s += &format!("\n    violation: {v:?}");
        }
    }
    if !r.unpacked.is_empty() {
        s += &format!("\n  unpacked: {}", r.unpacked.len());
    }
    s
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    let out = take_output(&mut args);
    match args.first().map(String::as_str) {
        Some("pack") => {
            let path = args.get(1).ok_or("missing request path")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let req: PackRequest =
                serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            let res = pack(&req).map_err(|e| e.to_string())?;
            eprintln!("{}", summary(&res));
            if out.is_some() {
                write_json(&res, out)?;
            }
            Ok(())
        }
        Some("gen") => {
            let req = match args.get(1).map(String::as_str) {
                Some("br") => {
                    let class: usize = args
                        .get(2)
                        .and_then(|s| s.parse().ok())
                        .ok_or("class 1-7")?;
                    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
                    generate::br_like(class, seed).ok_or("class must be 1-7")?
                }
                Some("mixed") => {
                    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
                    generate::mixed(seed)
                }
                Some("shapes") => {
                    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
                    generate::shapes(seed)
                }
                _ => return Err("gen br|mixed|shapes".into()),
            };
            write_json(&req, out)
        }
        Some("thpack") => {
            let path = args.get(1).ok_or("missing thpack path")?;
            let n: usize = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .ok_or("problem number")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let req = generate::parse_thpack(&text, n)?;
            write_json(&req, out)
        }
        Some("bench") => {
            let per_class: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
            println!("class  instances  avg_util%  min_util%  avg_ms  all_valid");
            for class in 1..=7 {
                let (mut sum, mut min, mut ms, mut valid) = (0.0, f64::INFINITY, 0u64, true);
                for seed in 1..=per_class {
                    let mut req = generate::br_like(class, seed).unwrap();
                    req.options.max_containers = 1;
                    let r = pack(&req).map_err(|e| e.to_string())?;
                    let u = r
                        .containers
                        .first()
                        .map_or(0.0, |c| c.metrics.volume_utilization);
                    sum += u;
                    min = min.min(u);
                    ms += r.elapsed_ms;
                    valid &= r.is_valid();
                }
                println!(
                    "BR{class:<4} {per_class:>9}  {:>9.2}  {:>9.2}  {:>6}  {valid}",
                    sum / per_class as f64 * 100.0,
                    min * 100.0,
                    ms / per_class
                );
            }
            Ok(())
        }
        _ => Err(String::new()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is_empty() => usage(),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
