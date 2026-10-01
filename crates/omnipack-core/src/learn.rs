//! Learning the placement score from plans the user marked as good.
//!
//! Each saved plan is replayed unit by unit in its loading order. At every
//! step the placer's own candidate positions are scored with the placement
//! features ([`placer::FEATURES`], each in [0, 1]); the position the plan
//! actually used is the positive example, the other feasible candidates the
//! negatives. A linear ranker is fitted with a pairwise logistic loss (lower
//! scores win), starting from, and pulled towards, the hand-tuned default
//! weights, so a handful of examples cannot wreck it. [`export_jsonl`] writes
//! the same steps for training other models (for example an ONNX ranker).

use crate::model::{FillBias, PackRequest, Ranker};
use crate::placer::{self, ContainerState, NF};
use crate::plan::ContainerPlan;
use crate::{default_sequence, validate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Candidates kept per step, best first by the default score.
const MAX_CANDIDATES: usize = 48;

/// One placement decision: the features of the chosen position and of the
/// other positions the placer could have used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub item: String,
    pub chosen: Vec<f64>,
    pub others: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainReport {
    /// Plans used (valid ones only) and placement decisions taken from them.
    pub plans: usize,
    pub steps: usize,
    pub pairs: usize,
    /// Share of decisions where the chosen position scores best, before and
    /// after training (on the training data).
    pub top1_before: f64,
    pub top1_after: f64,
}

/// The decisions of one container plan. Plans with violations teach nothing
/// reliable and give no steps.
pub fn examples(req: &PackRequest, plan: &ContainerPlan) -> Vec<Step> {
    if !plan.violations.is_empty() {
        return Vec::new();
    }
    let instances = default_sequence(req);
    let index_of: HashMap<&str, usize> = instances.iter().enumerate().map(|(i, x)| (x.id.as_str(), i)).collect();
    let mut state = ContainerState::new(&req.container, &req.options, &instances);
    let mut ordered: Vec<_> = plan.placements.iter().collect();
    ordered.sort_by_key(|p| p.seq);
    let mut steps = Vec::new();
    for p in ordered {
        let Some(&i) = index_of.get(p.instance_id.as_str()) else { continue };
        let Some(orient) = instances[i].shapes.iter().position(|s| s.orientation == p.orientation) else { continue };
        let near = |o: usize, at: [f64; 3]| o == orient && (0..3).all(|k| (at[k] - p.position[k]).abs() < 0.5);
        // Rank the placer's candidates by the default score, keep the best feasible ones.
        let mut scored: Vec<(f64, usize, [f64; 3])> = Vec::new();
        for (o, x, z) in state.candidates(i) {
            if let Some(y) = state.rest_height(i, o, x, z) {
                if !near(o, [x, y, z]) {
                    scored.push((state.default_score(i, o, x, y, z), o, [x, y, z]));
                }
            }
        }
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut others = Vec::new();
        for (_, o, at) in scored {
            if others.len() >= MAX_CANDIDATES {
                break;
            }
            if state.feasible(i, o, at) {
                others.push(state.features(i, o, at[0], at[1], at[2]).to_vec());
            }
        }
        let chosen = state.features(i, orient, p.position[0], p.position[1], p.position[2]).to_vec();
        // A unit placed as a fallback (it would tip, nothing better fitted) or
        // by hand where the placer would not go teaches nothing about ranking.
        if !others.is_empty() && state.feasible(i, orient, p.position) {
            steps.push(Step { item: p.item_id.clone(), chosen, others });
        }
        if !state.place_at(i, orient, p.position) {
            state.force_place(i, orient, p.position);
        }
    }
    steps
}

fn dot(w: &[f64], f: &[f64]) -> f64 {
    w.iter().zip(f).map(|(a, b)| a * b).sum()
}

/// Share of steps where the chosen position has the lowest score.
pub fn top1(steps: &[Step], w: &[f64]) -> f64 {
    if steps.is_empty() {
        return 0.0;
    }
    let hits = steps.iter().filter(|s| {
        let c = dot(w, &s.chosen);
        s.others.iter().all(|o| c < dot(w, o) + 1e-12)
    });
    hits.count() as f64 / steps.len() as f64
}

/// Fits the ranker to `steps`, starting from `init` (usually
/// [`placer::default_weights`]). The pairwise loss is minimised by
/// full-batch Adam at a few learning rates; the weights that rank the chosen
/// positions first most often (checked every few epochs, never worse than
/// `init`) are kept. Deterministic.
pub fn train(steps: &[Step], init: &[f64]) -> (Ranker, TrainReport) {
    let pairs: usize = steps.iter().map(|s| s.others.len()).sum();
    let before = top1(steps, init);
    let mut best = (before, init.to_vec());
    for lr in [0.003, 0.01, 0.03] {
        descend(steps, init, lr, &mut best);
    }
    let (accuracy, weights) = best;
    let report = TrainReport { plans: 0, steps: steps.len(), pairs, top1_before: before, top1_after: accuracy };
    (Ranker { weights, examples: steps.len(), accuracy }, report)
}

/// Adam on the pairwise logistic loss `ln(1 + e^(w·(chosen − other)))` (the
/// chosen position should score lower), pulled towards `init`; updates
/// `best` with any better-ranking weights it passes through.
fn descend(steps: &[Step], init: &[f64], lr: f64, best: &mut (f64, Vec<f64>)) {
    const EPOCHS: usize = 300;
    const CHECK_EVERY: usize = 10;
    /// Pull towards the default weights (per unit of squared distance).
    const L2: f64 = 1e-3;
    let mut w = init.to_vec();
    w.resize(NF, 0.0);
    let (mut m, mut v) = ([0.0; NF], [0.0; NF]);
    let (b1, b2, eps) = (0.9, 0.999, 1e-8);
    for epoch in 1..=EPOCHS {
        // Each step counts equally, whatever its candidate count.
        let mut g = [0.0; NF];
        for s in steps {
            let k = 1.0 / (s.others.len() as f64 * steps.len() as f64);
            for o in &s.others {
                let d: Vec<f64> = s.chosen.iter().zip(o).map(|(a, b)| a - b).collect();
                let sig = 1.0 / (1.0 + (-dot(&w, &d)).exp());
                for j in 0..NF {
                    g[j] += k * sig * d[j];
                }
            }
        }
        for j in 0..NF {
            g[j] += 2.0 * L2 * (w[j] - init.get(j).copied().unwrap_or(0.0));
            m[j] = b1 * m[j] + (1.0 - b1) * g[j];
            v[j] = b2 * v[j] + (1.0 - b2) * g[j] * g[j];
            let mh = m[j] / (1.0 - b1.powi(epoch as i32));
            let vh = v[j] / (1.0 - b2.powi(epoch as i32));
            w[j] -= lr * mh / (vh.sqrt() + eps);
        }
        if epoch % CHECK_EVERY == 0 {
            let acc = top1(steps, &w);
            if acc > best.0 + 1e-12 {
                *best = (acc, w.clone());
            }
        }
    }
}

/// Builds the steps of every valid container of the given (request, plan)
/// pairs and trains a ranker on them. It starts from the hand-tuned fill
/// pattern that already explains the decisions best.
pub fn train_on(plans: &[(PackRequest, Vec<ContainerPlan>)]) -> Option<(Ranker, TrainReport)> {
    let (first, _) = plans.first()?;
    let mut steps = Vec::new();
    let mut used = 0;
    for (req, containers) in plans {
        let before = steps.len();
        for c in containers {
            steps.extend(examples(req, c));
        }
        used += usize::from(steps.len() > before);
    }
    if steps.is_empty() {
        return None;
    }
    let patterns = [FillBias::WallBuilding, FillBias::FloorFirst, FillBias::Longitudinal, FillBias::Lateral, FillBias::CornerFirst];
    let init = patterns
        .iter()
        .map(|&bias| placer::default_weights(&crate::model::PackOptions { bias, ..first.options.clone() }))
        .max_by(|a, b| top1(&steps, a).total_cmp(&top1(&steps, b)))?;
    let (ranker, mut report) = train(&steps, &init);
    report.plans = used;
    Some((ranker, report))
}

/// The steps as JSON lines, one decision per line, with the feature names.
pub fn export_jsonl(steps: &[Step]) -> String {
    #[derive(Serialize)]
    struct Line<'a> {
        version: u32,
        features: &'a [&'a str],
        item: &'a str,
        chosen: &'a [f64],
        others: &'a [Vec<f64>],
    }
    steps
        .iter()
        .map(|s| {
            serde_json::to_string(&Line { version: 1, features: &placer::FEATURES, item: &s.item, chosen: &s.chosen, others: &s.others }).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether every container of a plan is valid (only those may be used to train).
pub fn trainable(containers: &[ContainerPlan]) -> bool {
    !containers.is_empty() && containers.iter().all(|c| c.violations.is_empty())
}

/// Re-checks a plan from its placements alone (for plans read from files).
pub fn revalidate(req: &PackRequest, containers: &mut [ContainerPlan]) {
    for c in containers {
        c.violations = validate::validate(&req.container, &req.items, &req.options, &c.placements);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContainerSpec, FillBias, ItemSpec, PackOptions, Zone};
    use crate::pack;
    use omnipack_geom::Shape;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    fn boxes() -> PackRequest {
        let item = |id: &str, w: f64, h: f64, d: f64, quantity: u32| ItemSpec {
            id: id.into(),
            shape: Shape::Box { w, h, d },
            mass: w * h * d * 1e-7,
            quantity,
            max_load_on_top: None,
            fragile: false,
            floor_only: false,
            upright_only: true,
            allowed_orientations: None,
            stop: 0,
            zone: Zone::Any,
            com_offset: [0.0; 3],
            color: None,
            friction: None,
        };
        PackRequest {
            container: ContainerSpec::new("c", 1200.0, 900.0, 1600.0),
            items: vec![item("a", 400.0, 300.0, 400.0, 14), item("b", 300.0, 200.0, 300.0, 18)],
            options: PackOptions { bias: FillBias::WallBuilding, max_containers: 1, ..Default::default() },
        }
    }

    /// Share of units placed at a different spot in two plans.
    fn moved(a: &ContainerPlan, b: &ContainerPlan) -> f64 {
        let same = a
            .placements
            .iter()
            .filter(|p| b.placements.iter().any(|q| q.instance_id == p.instance_id && (0..3).all(|k| (p.position[k] - q.position[k]).abs() < 1.0)))
            .count();
        1.0 - same as f64 / a.placements.len().max(1) as f64
    }

    #[test]
    fn features_stay_in_the_unit_range() {
        let req = boxes();
        let plan = &pack(&req).unwrap().containers[0];
        let steps = examples(&req, plan);
        assert!(steps.len() > 10);
        for s in &steps {
            for f in std::iter::once(&s.chosen).chain(&s.others) {
                assert!(f.len() == NF && f.iter().all(|v| (0.0..=1.0).contains(v)), "{f:?}");
            }
        }
    }

    #[test]
    fn the_lower_bound_never_exceeds_the_score() {
        let mut req = boxes();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        req.options.bias = FillBias::Learned;
        for _ in 0..5 {
            req.options.ranker = Some(Ranker { weights: (0..NF).map(|_| rng.gen_range(-1.0..1.0)).collect(), examples: 0, accuracy: 0.0 });
            let instances = default_sequence(&req);
            let mut state = ContainerState::new(&req.container, &req.options, &instances);
            for i in 0..instances.len() {
                for (o, x, z) in state.candidates(i) {
                    if let Some(y) = state.rest_height(i, o, x, z) {
                        let (lb, s) = (state.score_lower_bound(i, o, x, z), state.score(i, o, x, y, z));
                        assert!(lb <= s + 1e-9, "bound {lb} > score {s}");
                    }
                }
                state.try_place(i);
            }
        }
    }

    #[test]
    fn training_learns_another_fill_pattern() {
        // Plans made with floor layers first; the ranker starts from walls first.
        let walls = boxes();
        let mut floors = walls.clone();
        floors.options.bias = FillBias::FloorFirst;
        let target = pack(&floors).unwrap().containers.remove(0);
        let wall_plan = pack(&walls).unwrap().containers.remove(0);
        let steps = examples(&floors, &target);
        let (ranker, report) = train(&steps, &placer::default_weights(&walls.options));
        assert!(report.top1_after > report.top1_before + 0.2, "{report:?}");
        let mut learned = walls.clone();
        learned.options.bias = FillBias::Learned;
        learned.options.ranker = Some(ranker);
        let result = pack(&learned).unwrap();
        assert!(result.is_valid());
        let plan = &result.containers[0];
        assert!(moved(plan, &target) < moved(&wall_plan, &target), "learned {} vs walls {}", moved(plan, &target), moved(&wall_plan, &target));
        assert!(export_jsonl(&steps).lines().count() == steps.len());
    }
}
