# Learned placement

OmniPack can learn how you like units placed from plans you mark as good. These can
be plans you placed by hand, automatic plans, or a mix of both. The model learns the
**placement score**: the number the placer uses to choose among the positions a unit
could take. It does not learn new physics. Every plan it produces is checked exactly
like any other plan.

Code: `crates/omnipack-core/src/learn.rs`, and `placer.rs` (`features`,
`default_weights`, `FillBias::Learned`).

## Features

For a unit at a candidate position the placer computes twelve features, each between
0 and 1. Lower scores win.

| # | Name | Meaning |
|---|---|---|
| 0 | `x` | position across the width, x / W |
| 1 | `y` | height of the unit's base, y / H |
| 2 | `depth` | distance from where filling starts: the front wall, or the door for FIFO and door-zone units |
| 3 | `tip_deficit` | how far, in g, the unit standing alone falls short of resisting tipping in transport (/ 2) |
| 4 | `lateral_balance` | sideways offset of the cargo's centre of gravity after placing it (/ half width) |
| 5 | `lengthwise_balance` | lengthwise offset of the cargo's centre of gravity after placing it (/ half length) |
| 6 | `side_contact` | share of the four side faces touching walls or units |
| 7 | `blocked_sides` | share of sides held against sliding, directly or across a fillable gap |
| 8 | `dead_gaps` | share of sides leaving a gap too wide for dunnage but too narrow for any unit |
| 9 | `flat_top` | 1 if its top is flush with a touching neighbour's top |
| 10 | `top_height` | height of its top, (y + h) / H |
| 11 | `heavy_high` | its mass relative to the heaviest unit, times y / H |

The built-in fill patterns are fixed weights on these features (`default_weights`).
For example, walls across the width weight `depth` 1, `y` 0.01 and `x` 0.0001, plus the
small tie-break terms. With the fill pattern **Learned**, the placer uses the trained
weights instead. The search's lower bound stays valid for any weights, because every
feature lies in [0, 1].

## Training data

Each plan marked for training (in the app: **Solutions → train**) is replayed unit by
unit, in its loading order:

1. The placer's own candidate positions for the unit are generated: every anchor, in
   every allowed orientation, dropped under gravity.
2. The 48 best by the default score are kept, then filtered to those the placer would
   accept in its first pass: static checks and, with "Avoid tipping", no tipping in
   transport.
3. The position the plan actually used is the **positive** example; the other
   feasible candidates are the **negatives**.
4. The unit is then placed where the plan put it, and the next one is replayed.

Some cases are left out:
- Plans with violations are never used.
- Steps where the plan's position is one the placer would not have chosen in its first
  pass are skipped. These are fallbacks: units placed although they would tip, or put
  by hand where the checks fail.
- Hand-made plans keep the order the units were placed in, which is the order you
  made your decisions in. Only if that order is not physically possible (a unit
  placed "in the air" before the one under it) are they loaded lowest first instead.

## Training

The ranker is a weight vector w. For each step and each negative, the loss is

```
ln(1 + exp(w · (f_chosen − f_other)))
```

It is small when the chosen position scores lower than the other one. Each step
counts equally, however many candidates it has. An L2 term (10⁻³) pulls w towards
the starting weights, so a few examples cannot push it to extremes.

Optimisation:
- Full-batch Adam runs for 300 epochs at learning rates 0.003, 0.01 and 0.03.
- Every 10 epochs, OmniPack measures the **top-1 accuracy**: the share of steps where
  the chosen position scores best. It keeps the weights with the highest accuracy.
- The start is the built-in fill pattern that already explains the steps best. The
  result is never worse than that start.

The report gives the steps, the pairs, and the top-1 accuracy before and after
training. Replaying a plan the placer made with a built-in pattern gives 100% for that
pattern, which is a good sanity check.

Training takes a second or two for a few thousand decisions. The model is stored in
the app data folder as `model.json`. **Reset model** deletes it.

## Using the model

- The fill pattern **Learned (from your saved plans)** packs with the trained weights.
- **★ Best** adds Learned to the patterns it sweeps and evolves, next to the five
  built-in ones.
- From the command line:

  ```
  omnipack train solution1.json solution2.json -o ranker.json
  omnipack pack request.json --ranker ranker.json
  omnipack optimize request.json --ranker ranker.json
  ```

  The solution files are the app's saved plans (`<app data>/solutions/<id>.json`), or
  any JSON with `request` and `result`.

## Export for other models (ONNX)

**Solutions → Export training data**, or `omnipack export-training …`, writes one JSON
object per line, one line per placement decision:

```json
{"version":1,"features":["x","y","depth",…],"item":"pallet","chosen":[0.0,0.0,…],"others":[[…],[…]]}
```

- `chosen` is the feature vector of the position the plan used.
- `others` are the feasible alternatives the placer would have considered.

This is a standard learning-to-rank data set. A model trained on it (for example a
small neural ranker exported to ONNX) can replace the linear scoring later. Milestone
M6 tracks that.
