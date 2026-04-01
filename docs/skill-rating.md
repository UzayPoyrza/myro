# Per-Tag Skill Ratings

## Why This Algorithm Works

The goal is to answer: "What CF rating level can this user handle for dp problems? For graph problems?" The naive approach would be to look at the user's latent dimension for each tag (`theta[d]` where `d = tag_dim_map[tag]`), but that has serious problems. Instead, we use a prediction-based approach that's more robust.

### The Naive Approach (and Why We Don't Use It)

The model learns a `tag_dim_map` during training that maps tags like "dp" to specific latent dimensions. You might think: just read `theta[5]` to get the user's "dp skill." But this breaks down because:

1. **SGD drift.** Tag-informed init seeds dimension 5 with dp signal, but after 100 epochs of SGD on all observations simultaneously, dimension 5 captures whatever structure minimizes loss — which is correlated with dp but is not purely dp. The model is free to spread dp signal across multiple dimensions or mix dp with other tag signal in a single dimension.

2. **Raw theta values aren't interpretable.** `theta[5] = +0.412` means the user has a positive loading on dimension 5 relative to the population, but you can't convert that to a CF rating without knowing how the corresponding problem alphas are distributed.

3. **Tag overlap.** A problem tagged `[dp, graphs, trees]` contributes to training signal for dimensions 0, 1, and 7 simultaneously. After training, you can't cleanly attribute a dimension to a single tag.

### The Prediction-Based Approach

Instead of looking at raw theta values, we ask the model a concrete question: **for problems tagged T at each rating level, what's your predicted P(solve)?**

This works because:

1. **Predictions use all dimensions.** `P(solve) = σ(θ·α + b_u + d_p)` — the dot product naturally weights all 30 dimensions according to how relevant they are for each problem. A dp problem at rating 1800 has its own learned `alpha` vector that captures what skills it actually requires, regardless of how cleanly the latent dimensions align with tags.

2. **Tag metadata is ground truth.** We filter problems by their CF tags (stored in `problem_tags`), which are human-assigned labels from problem setters — not learned representations. So "dp problems" means exactly what you'd expect.

3. **Rating is the natural scale.** By finding where P(solve) crosses 0.5 along the rating axis, we get an answer in CF rating units that's directly comparable to the user's overall rating and to other users.

### Why Binning + Interpolation

The relationship between problem rating and P(solve) for a given tag is monotonically decreasing (harder problems → lower P(solve)), but it's noisy at the individual problem level because:

- Problem difficulty varies within a rating level (some 1800-rated dp problems are much harder than others)
- The model has per-problem difficulty biases that don't align perfectly with official ratings
- Some problems are mistagged or have unusual difficulty for their rating

Binning into 200-rating intervals smooths this noise. Within each bin, the mean P(solve) is a robust estimate. Walking the bins to find where the mean crosses 0.5 then gives a stable estimate of the user's skill boundary.

200 was chosen because CF ratings cluster in multiples of 100 (800, 900, ..., 3500), and bins of 100 are too noisy for tags with fewer problems while bins of 400 lose too much resolution.

### Why P(solve) = 0.5 as the Boundary

0.5 is the natural decision boundary of the logistic model — it's where the logit is zero, meaning the user's skill vector exactly balances the problem's difficulty vector. Below 0.5, the user is more likely to fail than succeed; above 0.5, more likely to succeed.

This maps cleanly to the intuitive notion of "skill level": the rating at which problems become a coin flip for you.

### What the `tag_dim_map` Actually Does Here

The `tag_dim_map` is used for exactly one thing in the skill rating algorithm: **enumerating which tags exist in the model.** We iterate `tag_dim_map.keys()` to get the tag list, then use `problem_tags` (per-problem metadata) to filter problems for each tag.

The dimension indices in the map are not used for skill computation at all. They're only displayed in the CLI output's `theta` column as a diagnostic — to see whether the raw dimension value correlates with the effective rating (it usually does loosely, confirming the tag-init did something useful, but the correlation isn't tight enough to rely on).

This means the algorithm is robust to:
- Retraining with different `k` (latent dimensions)
- Retraining with `tag_init = false` (no tag-informed initialization at all)
- Tags that don't have a dedicated dimension (more tags than dimensions)

In all cases, the predictions still work because the model learned *something* about problem difficulty structure, and we're just asking "what does the model predict for dp problems at each rating level?"

### Worked Example

User kalimm, tag "greedy":

1. Find all problems tagged "greedy" with known ratings → 2631 problems
2. Predict P(solve) for each using kalimm's fitted embedding
3. Bin by rating:

```
Rating bin    Mean P(solve)    Count
800-1000      0.92             412
1000-1200     0.84             389
1200-1400     0.72             356
1400-1600     0.61             334
1600-1800     0.53             298    ← above 0.5
1800-2000     0.41             267    ← below 0.5
2000-2200     0.28             198
...
```

4. Crossing between bins 1600-1800 (0.53) and 1800-2000 (0.41):
   - `frac = (0.53 - 0.5) / (0.53 - 0.41) = 0.25`
   - `rating = 1700 + 0.25 * 200 = 1750`
   - But bins use centers (1700, 1900), so: `1700 + 0.25 * 200 = 1750`
5. Effective rating for greedy: **1750**

This means kalimm has about a 50/50 chance of solving a 1750-rated greedy problem.

---

## Effective Rating Algorithm

For each tag T, we find the CF problem rating where `P(solve) = 0.5` — the user's "skill boundary" for that tag.

### Steps

1. **Collect** all problems with tag T that have a known CF rating
2. **Predict** `P(solve)` for each using the user's fitted embedding
3. **Bin** by 200-rating intervals (e.g., 800-1000, 1000-1200, ...)
4. **Compute** mean `P(solve)` per bin
5. **Walk** bins low-to-high to find where mean `P(solve)` crosses 0.5
6. **Linearly interpolate** between adjacent bins to get exact crossing point
7. **Clamp** to `[800, 3500]`

### Edge Cases

- **All P(solve) > 0.5**: User solves everything — extrapolate above highest bin + 200 (capped at 3500)
- **All P(solve) < 0.5**: User struggles with all — extrapolate below lowest bin - 200 (capped at 800)
- **Fewer than 5 rated problems** for a tag: Skip that tag (insufficient data)

### Strength Percentage

Strength measures how far above median the user's effective rating is:

```
strength = clamp((effective_rating - median_tag_rating) / 400 + 0.5, 0, 1)
```

- 50% = at the median rating for that tag
- 75% = 100 rating points above median
- 100% = 200+ rating points above median

### Overall Rating

Same algorithm applied across ALL problems regardless of tag.

## Exploratory Analysis

Run per-tag skill ratings for any CF user:

```bash
just tag-skills <handle>

# With CSV export:
just tag-skills <handle> --csv skills.csv
```

## How Retraining Affects Skills

The `tag_dim_map` and problem embeddings are part of `ProblemModel`. Retraining changes:
- Problem embeddings `alpha[d]` (new latent structure → different predictions)
- The tag list (if new tags appear in training data)
- User embeddings need re-fitting but the algorithm is the same

No extra pipeline steps are needed — skill ratings compute on-the-fly from whatever `problem_model.bin.gz` is current. The algorithm is retraining-safe because it uses predictions and tag metadata, not raw dimension values.

## Limitations

1. **Tag overlap**: A "dp + graphs" problem contributes to both tags. The effective rating for each tag includes cross-tag signal. A user who's strong at "dp + graphs combo" problems will show elevated ratings for both dp and graphs, even if they struggle with pure graph problems.
2. **Sparse tags**: Tags with few rated problems (e.g., "chinese remainder theorem") are filtered out by the 5-problem minimum.
3. **Extrapolation**: When all problems are above/below 0.5, the estimated rating is rough (one bin-width beyond the last observed bin, capped).
4. **Bin granularity**: 200-rating bins may be too coarse for users near the boundary between two bins. This is a smoothness-vs-resolution tradeoff.
5. **Model accuracy ceiling**: The effective rating is only as good as the model's predictions. If the model is poorly trained or has insufficient data for a tag, the skill rating will be noisy.
6. **CF tag quality**: Some problems are mistagged on Codeforces. The algorithm trusts tag metadata as ground truth.

## Implementation

| File | What it does |
|------|-------------|
| `crates/myro-predict/src/model/skills.rs` | `compute_skill_profile`, `compute_skill_deltas`, types |
| `crates/myro-predict/src/history.rs` | `SkillHistory`, `SkillSnapshot` for persistence |
| `crates/myro-tui/src/recommend.rs` | Sends `SkillProfile` events after fitting/refitting |
| `crates/myro-tui/src/app.rs` | Stats page state, delta popup logic |
| `crates/myro-tui/src/ui.rs` | `render_stats`, `render_skill_deltas` |
