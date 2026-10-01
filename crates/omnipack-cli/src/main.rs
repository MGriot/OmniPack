//! `omnipack` command-line tool.
//!
//! ```text
//! omnipack pack <request.json> [-o plan.json]
//! omnipack gen br <class 1-7> <seed> [-o request.json]
//! omnipack gen mixed|shapes <seed> [-o request.json]
//! omnipack thpack <thpackN.txt> <problem 1..> [-o request.json]
//! omnipack optimize <request.json> [--budget seconds] [--evals count] [-o plan.json]
//! omnipack bench [instances-per-class] [--optimize seconds]
//!   (OMNIPACK_WEIGHTS=contact,blocking,dead_gap,flat_top overrides the score weights)
//! ```

use omnipack_core::generate;
use omnipack_core::{pack, PackRequest, PackResult, SecuringClass};
use omnipack_opt::{optimize, OptimizeOptions, Score};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  omnipack pack <request.json> [-o plan.json]\n  omnipack gen br <class 1-7> <seed> [-o request.json]\n  \
         omnipack gen mixed <seed> [-o request.json]\n  omnipack thpack <thpackN.txt> <problem> [-o request.json]\n  \
         omnipack optimize <request.json> [--budget seconds] [--evals count] [-o plan.json]
  \n         omnipack bench [instances-per-class] [--optimize seconds]"
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
        if r.is_valid() { "all checks passed" } else { "VIOLATIONS FOUND" }
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
        let b = &c.balance;
        s += &format!(
            "\n    balance: CoG {:+.0} mm lengthwise, {:+.0} mm sideways, {:.0}% in the middle half, CoG at {:.0}% of the height{}{}",
            b.lengthwise_offset,
            b.lateral_offset,
            b.central_share * 100.0,
            b.cog_height_ratio * 100.0,
            b.vgm.map_or(String::new(), |v| format!(", VGM {v:.0} kg")),
            if b.shift != 0.0 { format!(", moved {:+.0} mm", b.shift) } else { String::new() }
        );
        if let Some(v) = &b.vehicle {
            s += &format!("\n    axles: steer {:.0}, drive {:.0}, trailer {:.0}, gross {:.0} kg", v.steer, v.drive, v.trailer, v.gross);
        }
        for i in &b.issues {
            s += &format!("\n    balance warning: {i:?}");
        }
        for v in &c.violations {
            s += &format!("\n    violation: {v:?}");
        }
    }
    if !r.unpacked.is_empty() {
        s += &format!("\n  unpacked: {}", r.unpacked.len());
    }
    s
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let i = args.iter().position(|a| a == flag)?;
    let v = args.get(i + 1).cloned();
    args.drain(i..(i + 2).min(args.len()));
    v
}

fn score_line(s: &Score) -> String {
    format!(
        "{} units, {} container(s), {:.1}% vol, {} need lashing ({:.1} kN), dunnage {:.0} mm, min margin {:.1} mm, {} balance warnings, {} over floor rating, value {:.2}",
        s.packed_units,
        s.containers,
        s.volume_utilization * 100.0,
        s.lashing_units,
        s.lashing_kn,
        s.dunnage_mm,
        s.min_margin,
        s.balance_issues,
        s.floor_overloads,
        s.value
    )
}

fn search_options(args: &mut Vec<String>) -> Result<OptimizeOptions, String> {
    let mut o = OptimizeOptions::default();
    if let Some(b) = take_flag(args, "--budget") {
        o.budget_ms = (b.parse::<f64>().map_err(|_| "--budget <seconds>")? * 1000.0) as u64;
    }
    if let Some(e) = take_flag(args, "--evals") {
        o.max_evaluations = e.parse().map_err(|_| "--evals <count>")?;
    }
    Ok(o)
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    let out = take_output(&mut args);
    let bench_budget = take_flag(&mut args, "--optimize");
    let search = search_options(&mut args)?;
    match args.first().map(String::as_str) {
        Some("optimize") => {
            let path = args.get(1).ok_or("missing request path")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let req: PackRequest = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            let cancel = AtomicBool::new(false);
            let r = optimize(&req, &search, &cancel, &mut |p| eprint!("\r{:?}: {} plans, best {:.2}   ", p.phase, p.evaluated, p.best.value)).map_err(|e| e.to_string())?;
            eprintln!("\n{} plans in {} ms", r.evaluated, r.elapsed_ms);
            eprintln!("  your settings: {}", score_line(&r.baseline));
            for (i, s) in r.solutions.iter().enumerate() {
                eprintln!("  #{} ({}): {}", i + 1, s.label, score_line(&s.score));
            }
            if out.is_some() {
                write_json(&r.solutions[0].result, out)?;
            }
            Ok(())
        }
        Some("pack") => {
            let path = args.get(1).ok_or("missing request path")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let req: PackRequest = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
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
                    let class: usize = args.get(2).and_then(|s| s.parse().ok()).ok_or("class 1-7")?;
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
            let n: usize = args.get(2).and_then(|s| s.parse().ok()).ok_or("problem number")?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let req = generate::parse_thpack(&text, n)?;
            write_json(&req, out)
        }
        Some("bench") => {
            let per_class: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
            let opt_budget: Option<f64> = bench_budget.map(|b| b.parse().map_err(|_| "--optimize <seconds>")).transpose()?;
            print!("class  instances  avg_util%  min_util%  lashing%  dunnage_mm  avg_ms  all_valid");
            println!("{}", if opt_budget.is_some() { "  opt_util%  opt_lashing%  opt_valid" } else { "" });
            for class in 1..=7 {
                let (mut sum, mut min, mut ms, mut valid) = (0.0, f64::INFINITY, 0u64, true);
                let (mut lashing, mut units, mut dunnage) = (0usize, 0usize, 0.0);
                let (mut opt_util, mut opt_lash, mut opt_units, mut opt_valid) = (0.0, 0usize, 0usize, true);
                for seed in 1..=per_class {
                    let mut req = generate::br_like(class, seed).unwrap();
                    req.options.max_containers = 1;
                    if let Ok(w) = std::env::var("OMNIPACK_WEIGHTS") {
                        let v: Vec<f64> = w.split(',').filter_map(|s| s.parse().ok()).collect();
                        req.options.weights = omnipack_core::ScoreWeights { contact_area: v[0], blocking: v[1], dead_gap: v[2], flat_top: v[3] };
                    }
                    let r = pack(&req).map_err(|e| e.to_string())?;
                    let u = r.containers.first().map_or(0.0, |c| c.metrics.volume_utilization);
                    sum += u;
                    min = min.min(u);
                    ms += r.elapsed_ms;
                    valid &= r.is_valid();
                    for c in &r.containers {
                        units += c.placements.len();
                        lashing += c.placements.iter().filter(|p| p.securing >= SecuringClass::Lashing).count();
                        dunnage += c.transport.iter().flat_map(|t| &t.gaps).map(|g| g.gap_mm).sum::<f64>();
                    }
                    if let Some(secs) = opt_budget {
                        let o = OptimizeOptions { budget_ms: (secs * 1000.0) as u64, ..search.clone() };
                        let best = optimize(&req, &o, &AtomicBool::new(false), &mut |_| {}).map_err(|e| e.to_string())?.solutions.remove(0);
                        opt_util += best.result.containers.first().map_or(0.0, |c| c.metrics.volume_utilization);
                        opt_lash += best.score.lashing_units;
                        opt_units += best.score.packed_units;
                        opt_valid &= best.result.is_valid();
                    }
                }
                print!(
                    "BR{class:<4} {per_class:>9}  {:>9.2}  {:>9.2}  {:>8.1}  {:>10.0}  {:>6}  {valid}",
                    sum / per_class as f64 * 100.0,
                    min * 100.0,
                    lashing as f64 / units.max(1) as f64 * 100.0,
                    dunnage / per_class as f64,
                    ms / per_class
                );
                if opt_budget.is_some() {
                    print!("  {:>9.2}  {:>12.1}  {opt_valid}", opt_util / per_class as f64 * 100.0, opt_lash as f64 / opt_units.max(1) as f64 * 100.0);
                }
                println!();
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
