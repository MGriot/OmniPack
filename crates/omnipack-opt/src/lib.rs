//! Search for the best load plan, with the full physics in the loop.
//!
//! Trying every order × orientation × position is out of reach (n! · 6ⁿ), so
//! the search runs in three phases within a time budget:
//!
//! 1. **Sweep**: every fill pattern × load priority through the constructive
//!    placer (exhaustive over the settings).
//! 2. **Evolve**: a biased random-key genetic algorithm (Gonçalves & Resende
//!    2013). A chromosome holds one key per unit for the loading order, one per
//!    unit for a preferred orientation and one for the fill pattern; the placer
//!    decodes it into a plan. It is seeded with the best sweep plans.
//! 3. **Polish**: local search around the best chromosome (swap two units,
//!    change one orientation or the fill pattern).
//!
//! Every plan is built by the same placer and re-checked by the independent
//! validator; plans with violations are discarded, and plans where fewer
//! units would tip in transport beat denser ones. Load-balance warnings (CTU
//! Code window, vehicle axles) and floor overloads cost value. Loading constraints
//! (stops, zones, floor-only units) are kept by only reordering units within
//! the same [`load_group`].

use omnipack_core::placer::Instance;
use omnipack_core::balance::container_issues;
use omnipack_core::{default_sequence, load_group, pack, pack_sequence, BalanceIssue, FillBias, IssueKind, LoadPriority, PackError, PackRequest, PackResult, SecuringClass};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const BIASES: [FillBias; 5] = [FillBias::WallBuilding, FillBias::FloorFirst, FillBias::Longitudinal, FillBias::Lateral, FillBias::CornerFirst];
pub const PRIORITIES: [LoadPriority; 5] = [LoadPriority::Volume, LoadPriority::Mass, LoadPriority::BaseArea, LoadPriority::Height, LoadPriority::AsListed];

/// What a good plan is, beyond packing every unit into as few containers as
/// possible (those two always come first).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Objective {
    /// Per percentage point of volume utilization.
    pub density: f64,
    /// Per percentage point of units that need lashing.
    pub securing: f64,
    /// Per metre of dunnage (total gap width to fill).
    pub dunnage: f64,
    /// Per 100 mm of the smallest stability margin (capped at 100 mm).
    pub stability: f64,
    /// Per percentage point of load-balance excess (CTU Code window, central
    /// share, vehicle axles; see `BalanceReport::excess`).
    pub balance: f64,
}

impl Default for Objective {
    fn default() -> Self {
        Objective { density: 1.0, securing: 0.5, dunnage: 0.2, stability: 1.0, balance: 0.3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OptimizeOptions {
    /// Wall-clock budget, ms.
    pub budget_ms: u64,
    /// Stop after this many plans (0 = only the time budget). With a limit
    /// and a large budget the search is deterministic for a given seed.
    pub max_evaluations: usize,
    pub population: usize,
    pub elite_fraction: f64,
    pub mutant_fraction: f64,
    /// Probability that a child takes a gene from its elite parent.
    pub inherit: f64,
    /// Number of distinct plans to return.
    pub keep: usize,
    pub objective: Objective,
    /// Worker threads (0 = all cores).
    pub threads: usize,
    pub seed: u64,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        OptimizeOptions {
            budget_ms: 15_000,
            max_evaluations: 0,
            population: 40,
            elite_fraction: 0.2,
            mutant_fraction: 0.15,
            inherit: 0.7,
            keep: 3,
            objective: Objective::default(),
            threads: 0,
            seed: 0,
        }
    }
}

/// Quality of one plan. Compared by whether every unit is packed, then by
/// containers used, then by units that would tip in transport (fewer first),
/// then by `value` (which rewards packed volume, so when not
/// everything fits, more cargo wins).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub all_packed: bool,
    pub packed_units: usize,
    pub containers: usize,
    /// Weighted objective (higher is better).
    pub value: f64,
    pub volume_utilization: f64,
    /// Units that would tip in transport unless lashed.
    pub tipping_units: usize,
    /// Units that need lashing or are overloaded in transport.
    pub lashing_units: usize,
    /// Sum over those units of the largest securing force, kN.
    pub lashing_kn: f64,
    /// Total gap width to fill with dunnage (worst transport case), mm.
    pub dunnage_mm: f64,
    /// Smallest stability margin of any unit, mm.
    pub min_margin: f64,
    /// Load-balance warnings over all containers (floor pressure not included).
    #[serde(default)]
    pub balance_issues: usize,
    /// Sum of the balance excess over all containers, percentage points.
    #[serde(default)]
    pub balance_excess: f64,
    /// Units whose floor pressure exceeds the floor rating.
    #[serde(default)]
    pub floor_overloads: usize,
}

impl Score {
    pub fn of(r: &PackResult, obj: &Objective) -> Score {
        let placements = || r.containers.iter().flat_map(|c| &c.placements);
        let lashing: Vec<&str> = placements().filter(|p| p.securing >= SecuringClass::Lashing).map(|p| p.instance_id.as_str()).collect();
        let tipping: std::collections::HashSet<&str> = r
            .containers
            .iter()
            .flat_map(|c| &c.transport)
            .flat_map(|t| &t.issues)
            .filter(|i| i.kind == IssueKind::Tipping)
            .map(|i| i.item.as_str())
            .collect();
        let mut force: HashMap<&str, f64> = HashMap::new();
        for i in r.containers.iter().flat_map(|c| &c.transport).flat_map(|t| &t.issues) {
            if i.kind != IssueKind::StackOverload {
                let f = force.entry(i.item.as_str()).or_insert(0.0);
                *f = f.max(i.required);
            }
        }
        let dunnage_mm: f64 = r
            .containers
            .iter()
            .map(|c| c.transport.iter().map(|t| t.gaps.iter().map(|g| g.gap_mm).sum::<f64>()).fold(0.0, f64::max))
            .sum::<f64>()
            + 0.0; // an empty float sum is −0.0
        let min_margin = r.containers.iter().map(|c| c.metrics.min_support_margin).filter(|m| m.is_finite()).fold(f64::INFINITY, f64::min);
        let balance_issues = r.containers.iter().map(|c| container_issues(&c.balance)).sum();
        let balance_excess: f64 = r.containers.iter().map(|c| c.balance.excess).sum();
        let floor_overloads =
            r.containers.iter().flat_map(|c| &c.balance.issues).filter(|i| matches!(i, BalanceIssue::FloorPressure { .. })).count();
        let share = |n: usize| if r.packed_units > 0 { n as f64 / r.packed_units as f64 } else { 0.0 };
        let margin_term = if min_margin.is_finite() { min_margin.clamp(0.0, 100.0) / 100.0 } else { 1.0 };
        // Units over the floor rating need load-spreading beams: securing effort.
        let value = obj.density * r.volume_utilization * 100.0 - obj.securing * (share(lashing.len()) + share(floor_overloads)) * 100.0
            - obj.dunnage * dunnage_mm / 1000.0
            + obj.stability * margin_term
            - obj.balance * balance_excess;
        Score {
            all_packed: r.packed_units == r.requested_units,
            packed_units: r.packed_units,
            containers: r.containers.len(),
            value,
            volume_utilization: r.volume_utilization,
            tipping_units: tipping.len(),
            lashing_units: lashing.len(),
            lashing_kn: force.values().sum(),
            dunnage_mm,
            min_margin,
            balance_issues,
            balance_excess,
            floor_overloads,
        }
    }

    pub fn better_than(&self, o: &Score) -> bool {
        self.all_packed
            .cmp(&o.all_packed)
            .then(o.containers.cmp(&self.containers))
            .then(o.tipping_units.cmp(&self.tipping_units))
            .then(self.value.total_cmp(&o.value))
            .is_gt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Sweep,
    Evolve,
    Polish,
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub evaluated: usize,
    pub elapsed_ms: u64,
    pub best: Score,
}

#[derive(Debug, Clone, Serialize)]
pub struct Solution {
    /// How the plan was found, e.g. "sweep: floor_first / mass" or "evolve".
    pub label: String,
    pub score: Score,
    pub result: PackResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizeResult {
    /// Best plans, best first, pairwise different in at least
    /// [`MIN_DIFFERENCE`] of the unit positions.
    pub solutions: Vec<Solution>,
    /// Score of the plan the plain placer gives with the request's own options.
    pub baseline: Score,
    pub evaluated: usize,
    pub elapsed_ms: u64,
    pub cancelled: bool,
}

/// Share of units placed differently for two plans to count as distinct.
pub const MIN_DIFFERENCE: f64 = 0.15;

/// A candidate: order keys, orientation keys, then one fill-pattern key.
type Genes = Vec<f64>;

struct Decoder<'a> {
    req: &'a PackRequest,
    base: Vec<Instance>,
    groups: Vec<(i64, u8, bool)>,
}

impl Decoder<'_> {
    fn len(&self) -> usize {
        2 * self.base.len() + 1
    }

    fn bias(g: &Genes) -> FillBias {
        let k = *g.last().unwrap_or(&0.0);
        BIASES[((k * BIASES.len() as f64) as usize).min(BIASES.len() - 1)]
    }

    fn decode(&self, g: &Genes) -> PackResult {
        let n = self.base.len();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| self.groups[a].cmp(&self.groups[b]).then(g[a].total_cmp(&g[b])).then(a.cmp(&b)));
        let seq: Vec<Instance> = order
            .iter()
            .map(|&i| {
                let mut inst = self.base[i].clone();
                let k = g[n + i];
                let m = inst.shapes.len();
                inst.orient_pref = (k >= 0.5 && m > 1).then(|| (((k - 0.5) * 2.0 * m as f64) as usize).min(m - 1));
                inst
            })
            .collect();
        let mut req = self.req.clone();
        req.options.bias = Self::bias(g);
        pack_sequence(&req, &seq)
    }

    /// Genes that reproduce `order` (unit ids in loading order) with no
    /// orientation preference.
    fn encode(&self, order: &[String], bias: FillBias) -> Genes {
        let n = self.base.len();
        let rank: HashMap<&str, usize> = order.iter().enumerate().map(|(r, id)| (id.as_str(), r)).collect();
        let mut g = vec![0.25; self.len()];
        for (i, inst) in self.base.iter().enumerate() {
            g[i] = (rank.get(inst.id.as_str()).copied().unwrap_or(i) as f64 + 0.5) / n as f64;
        }
        let bi = BIASES.iter().position(|b| *b == bias).unwrap_or(0);
        g[2 * n] = (bi as f64 + 0.5) / BIASES.len() as f64;
        g
    }
}

struct Search<'a> {
    opts: &'a OptimizeOptions,
    cancel: &'a AtomicBool,
    start: Instant,
    deadline: Instant,
    evaluated: usize,
    threads: usize,
    /// Best plans seen, best first (bounded).
    archive: Vec<Solution>,
}

impl Search<'_> {
    fn out_of_budget(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
            || Instant::now() >= self.deadline
            || (self.opts.max_evaluations > 0 && self.evaluated >= self.opts.max_evaluations)
    }

    /// Runs `jobs` in parallel (results in job order). Jobs left when the
    /// budget runs out are skipped (`None`), as are plans with violations.
    fn run<J: Sync>(&mut self, jobs: &[J], f: impl Fn(&J) -> PackResult + Sync) -> Vec<Option<(Score, PackResult)>> {
        let allowed = if self.opts.max_evaluations > 0 { jobs.len().min(self.opts.max_evaluations.saturating_sub(self.evaluated)) } else { jobs.len() };
        let obj = self.opts.objective;
        let (cancel, deadline) = (self.cancel, self.deadline);
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut out: Vec<Option<(Score, PackResult)>> = (0..jobs.len()).map(|_| None).collect();
        let slots: Vec<std::sync::Mutex<Option<(Score, PackResult)>>> = (0..allowed).map(|_| std::sync::Mutex::new(None)).collect();
        // Without a wall-clock limit every allowed job runs, so capped runs are reproducible.
        let timed = self.opts.max_evaluations == 0;
        std::thread::scope(|s| {
            for _ in 0..self.threads.min(allowed.max(1)) {
                s.spawn(|| loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    if k >= allowed || cancel.load(Ordering::Relaxed) || (timed && Instant::now() >= deadline) {
                        break;
                    }
                    let r = f(&jobs[k]);
                    if r.is_valid() {
                        *slots[k].lock().unwrap() = Some((Score::of(&r, &obj), r));
                    }
                });
            }
        });
        let done = next.load(Ordering::Relaxed).min(allowed);
        self.evaluated += done;
        for (k, slot) in slots.into_iter().enumerate() {
            out[k] = slot.into_inner().unwrap();
        }
        out
    }

    fn remember(&mut self, label: String, score: &Score, result: &PackResult) {
        const ARCHIVE: usize = 24;
        if self.archive.len() >= ARCHIVE && !score.better_than(&self.archive[ARCHIVE - 1].score) {
            return;
        }
        let at = self.archive.iter().position(|s| score.better_than(&s.score)).unwrap_or(self.archive.len());
        self.archive.insert(at, Solution { label, score: score.clone(), result: result.clone() });
        self.archive.truncate(ARCHIVE);
    }

    fn progress(&self, phase: Phase, report: &mut dyn FnMut(&Progress)) {
        if let Some(best) = self.archive.first() {
            report(&Progress { phase, evaluated: self.evaluated, elapsed_ms: self.start.elapsed().as_millis() as u64, best: best.score.clone() });
        }
    }
}

/// Share of units whose container or position differs between two plans.
pub fn difference(a: &PackResult, b: &PackResult) -> f64 {
    let pos = |r: &PackResult| -> HashMap<String, (usize, [f64; 3])> {
        r.containers.iter().enumerate().flat_map(|(ci, c)| c.placements.iter().map(move |p| (p.instance_id.clone(), (ci, p.position)))).collect()
    };
    let (pa, pb) = (pos(a), pos(b));
    let n = pa.len().max(pb.len()).max(1);
    let same = pa
        .iter()
        .filter(|(id, (ca, xa))| pb.get(*id).is_some_and(|(cb, xb)| ca == cb && (0..3).all(|k| (xa[k] - xb[k]).abs() < 1.0)))
        .count();
    1.0 - same as f64 / n as f64
}

/// Searches for the best plans. `progress` is called from this thread after
/// every batch; set `cancel` from another thread to stop early (the best
/// plans found so far are returned).
pub fn optimize(req: &PackRequest, opts: &OptimizeOptions, cancel: &AtomicBool, progress: &mut dyn FnMut(&Progress)) -> Result<OptimizeResult, PackError> {
    let start = Instant::now();
    let baseline_plan = pack(req)?;
    let baseline = Score::of(&baseline_plan, &opts.objective);
    let threads = if opts.threads > 0 { opts.threads } else { std::thread::available_parallelism().map_or(1, |n| n.get()) };
    let mut s = Search {
        opts,
        cancel,
        start,
        deadline: start + Duration::from_millis(opts.budget_ms),
        evaluated: 1,
        threads,
        archive: Vec::new(),
    };
    if baseline_plan.is_valid() {
        s.remember("your settings".into(), &baseline, &baseline_plan);
    }

    // Phase 1: every fill pattern × load priority.
    let mut configs = Vec::new();
    for bias in BIASES {
        for priority in PRIORITIES {
            if bias != req.options.bias || priority != req.options.priority {
                configs.push((bias, priority));
            }
        }
    }
    let swept = s.run(&configs, |&(bias, priority)| {
        let mut r = req.clone();
        r.options.bias = bias;
        r.options.priority = priority;
        pack_sequence(&r, &default_sequence(&r))
    });
    let mut seeds: Vec<(Score, Vec<String>, FillBias)> = Vec::new();
    for ((bias, priority), res) in configs.iter().zip(swept) {
        if let Some((score, result)) = res {
            s.remember(format!("sweep: {} / {}", name(bias), name(priority)), &score, &result);
            seeds.push((score, placement_order(&result, req), *bias));
        }
    }
    seeds.push((baseline.clone(), placement_order(&baseline_plan, req), req.options.bias));
    seeds.sort_by(|a, b| if a.0.better_than(&b.0) { std::cmp::Ordering::Less } else if b.0.better_than(&a.0) { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal });
    s.progress(Phase::Sweep, progress);

    // Phase 2: BRKGA over order, orientation and fill pattern.
    let base = default_sequence(req);
    let decoder = Decoder { req, groups: base.iter().map(|i| load_group(&req.options, i)).collect(), base };
    let mut rng = ChaCha8Rng::seed_from_u64(opts.seed ^ req.options.seed);
    let p = opts.population.max(4);
    let n_elite = ((p as f64 * opts.elite_fraction).round() as usize).clamp(1, p - 1);
    let n_mutant = ((p as f64 * opts.mutant_fraction).round() as usize).min(p - n_elite);
    let random = |rng: &mut ChaCha8Rng| -> Genes { (0..decoder.len()).map(|_| rng.gen::<f64>()).collect() };

    let mut pop: Vec<(Genes, Option<Score>)> = Vec::with_capacity(p);
    for (_, order, bias) in seeds.iter().take(p / 2) {
        pop.push((decoder.encode(order, *bias), None));
    }
    while pop.len() < p {
        pop.push((random(&mut rng), None));
    }
    let evolve_until = |s: &Search| {
        let t = s.start + Duration::from_millis(opts.budget_ms * 3 / 4);
        let cap = opts.max_evaluations > 0 && s.evaluated >= opts.max_evaluations * 3 / 4;
        Instant::now() >= t && opts.max_evaluations == 0 || cap
    };
    let mut best: Option<(Genes, Score)> = None;
    let mut generation = 0;
    while !decoder.base.is_empty() && !s.out_of_budget() && !evolve_until(&s) {
        let todo: Vec<usize> = (0..pop.len()).filter(|&i| pop[i].1.is_none()).collect();
        let genes: Vec<Genes> = todo.iter().map(|&i| pop[i].0.clone()).collect();
        let results = s.run(&genes, |g| decoder.decode(g));
        for (&i, res) in todo.iter().zip(results) {
            match res {
                Some((score, result)) => {
                    let label = format!("evolve, generation {generation}");
                    s.remember(label, &score, &result);
                    if best.as_ref().is_none_or(|(_, b)| score.better_than(b)) {
                        best = Some((pop[i].0.clone(), score.clone()));
                    }
                    pop[i].1 = Some(score);
                }
                None => pop[i].1 = Some(worst()),
            }
        }
        s.progress(Phase::Evolve, progress);
        pop.sort_by(|a, b| rank(a.1.as_ref(), b.1.as_ref()));
        let mut next: Vec<(Genes, Option<Score>)> = pop[..n_elite].to_vec();
        for _ in 0..n_mutant {
            next.push((random(&mut rng), None));
        }
        while next.len() < p {
            let a = &pop[rng.gen_range(0..n_elite)].0;
            let b = &pop[rng.gen_range(n_elite..p)].0;
            let child: Genes = a.iter().zip(b).map(|(x, y)| if rng.gen::<f64>() < opts.inherit { *x } else { *y }).collect();
            next.push((child, None));
        }
        pop = next;
        generation += 1;
    }

    // Phase 3: local search around the best chromosome.
    if let Some((mut genes, mut score)) = best {
        let n = decoder.base.len();
        while !s.out_of_budget() {
            let batch: Vec<Genes> = (0..s.threads.max(2))
                .map(|_| {
                    let mut g = genes.clone();
                    match rng.gen_range(0..4) {
                        0 | 1 => {
                            // Swap two units of the same loading group.
                            let a = rng.gen_range(0..n);
                            let same: Vec<usize> = (0..n).filter(|&b| b != a && decoder.groups[b] == decoder.groups[a]).collect();
                            if let Some(&b) = same.get(rng.gen_range(0..same.len().max(1))) {
                                g.swap(a, b);
                            }
                        }
                        2 => g[n + rng.gen_range(0..n)] = rng.gen(),
                        _ => g[2 * n] = rng.gen(),
                    }
                    g
                })
                .collect();
            let results = s.run(&batch, |g| decoder.decode(g));
            for (g, res) in batch.into_iter().zip(results) {
                if let Some((sc, result)) = res {
                    s.remember("polish".into(), &sc, &result);
                    if sc.better_than(&score) {
                        genes = g;
                        score = sc;
                    }
                }
            }
            s.progress(Phase::Polish, progress);
        }
    }

    // Best distinct plans.
    let mut solutions: Vec<Solution> = Vec::new();
    for cand in std::mem::take(&mut s.archive) {
        if solutions.len() >= opts.keep.max(1) {
            break;
        }
        if solutions.iter().all(|sol| difference(&sol.result, &cand.result) >= MIN_DIFFERENCE) {
            solutions.push(cand);
        }
    }
    if solutions.is_empty() {
        // Nothing valid was found, not even with the request's own settings.
        solutions.push(Solution { label: "your settings".into(), score: baseline.clone(), result: baseline_plan });
    }
    Ok(OptimizeResult {
        solutions,
        baseline,
        evaluated: s.evaluated,
        elapsed_ms: start.elapsed().as_millis() as u64,
        cancelled: cancel.load(Ordering::Relaxed),
    })
}

fn worst() -> Score {
    Score {
        all_packed: false,
        packed_units: 0,
        containers: usize::MAX,
        value: f64::NEG_INFINITY,
        volume_utilization: 0.0,
        tipping_units: usize::MAX,
        lashing_units: 0,
        lashing_kn: 0.0,
        dunnage_mm: 0.0,
        min_margin: 0.0,
        balance_issues: 0,
        balance_excess: 0.0,
        floor_overloads: 0,
    }
}

fn rank(a: Option<&Score>, b: Option<&Score>) -> std::cmp::Ordering {
    let (a, b) = (a.cloned().unwrap_or_else(worst), b.cloned().unwrap_or_else(worst));
    if a.better_than(&b) {
        std::cmp::Ordering::Less
    } else if b.better_than(&a) {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

/// Unit ids in the order the plan loads them (container by container), then
/// the unpacked ones.
fn placement_order(r: &PackResult, req: &PackRequest) -> Vec<String> {
    let mut ids: Vec<String> = r.containers.iter().flat_map(|c| c.placements.iter().map(|p| p.instance_id.clone())).collect();
    ids.extend(r.unpacked.iter().map(|u| u.instance_id.clone()));
    debug_assert!(ids.len() == req.items.iter().map(|i| i.quantity as usize).sum::<usize>());
    ids
}

fn name<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}
