# Cold-Start Prediction: Time-Weighted User Fitting

**Date:** 2026-03-05
**Status:** Approved

## Problem

myro-predict learns a fixed embedding per user during training. Users not in the training set hit a cold-start problem — they get no predictions until the model is retrained with their data. The goal is a TikTok-like experience: a new user logs into Codeforces, and myro immediately understands their skillset from contest history.

## Approach: Time-Weighted SGD (Approach A)

The model ships **problem embeddings only**. User embeddings are always computed on-the-fly by fitting against the user's solve history with time-decay weighting.

### Model Split

**Before:** `SolvePredictionModel` contains both user and problem params (~3.3MB).

**After:**
- `ProblemModel` — problem params + metadata (~200KB, shipped with myro-tui)
- `UserProfile` — fitted user params, cached locally

`ProblemModel` contains:
- `problem_params: Vec<ProblemParams>` (alpha vectors + difficulty biases)
- `problem_index: HashMap<String, usize>`
- `problem_ratings: Vec<Option<i32>>`
- `problem_tags: Vec<Vec<String>>`
- `tag_dim_map: HashMap<String, usize>`
- `config: ModelConfig`

### Time-Weighted Fitting

Each observation carries a timestamp. Weight is computed as:

```
w = 2^(-days_ago / 365)
```

365-day half-life: a solve from 1 year ago counts 50%, 2 years ago 25%, etc.

The gradient in `fit_user` is scaled by weight:

```
grad_theta[d] = w * (pred - y) * alpha_p[d] + lambda * theta[d]
grad_bias    = w * (pred - y) + lambda * bias
```

This is weighted logistic regression with fixed features — still convex, same convergence guarantees.

### Observation Sources

| Source | Outcome | Signal |
|--------|---------|--------|
| CF contest | Solved in contest | `(pid, true, contest_time)` |
| CF contest | Participated, didn't solve | `(pid, false, contest_time)` |
| Myro-tui | Solved | `(pid, true, solve_time)` |
| Myro-tui | Attempted, gave up/failed | `(pid, false, attempt_time)` |
| Either | Never attempted | No observation (excluded) |

### Local Storage (`~/.local/share/myro/`)

**`history.json`** — Primary myro-tui solve history:
```json
[
  {"problem_id": "1800:A", "solved": true, "timestamp": 1709654400},
  {"problem_id": "1801:B", "solved": false, "timestamp": 1709740800}
]
```

**`user_params.bin`** — Cached fitted params:
- Contains: `UserParams` (theta + bias) + history hash
- Invalidated when history changes (new CF submissions or myro-tui solves)

**Cache miss flow:**
1. Read CF handle from `state.json`
2. Load `history.json` (myro-tui history)
3. Fetch CF history via API (~1-2s)
4. Merge both sources, compute time-decay weights
5. Run `fit_user` (~100ms for 1000 observations)
6. Write cache

### CLI Changes

- **`train`**: No algorithm change. Still trains full MF model for evaluation.
- **`export-model`** (new): Exports problem-only `ProblemModel` file.
- **`query`**: Simplified — always fits from history, no "user in model" branching.
- **`eval`**: Adapted to simulate fit-from-history flow for test users.

### Training Pipeline

Problem embeddings require periodic retraining when new contests/problems are added. User embeddings are never stored in the model — always derived at inference time.

---

## Future: Approach B — Bayesian Online Update

*Not implemented. Documented for potential future comparison.*

### Motivation

Approach A re-fits from scratch on every history change. For users with very large histories (10,000+ observations), this could become slow. Approach B offers truly incremental updates.

### Architecture

Maintain a Gaussian posterior over user params instead of point estimates:

```
p(θ_u | history) = N(μ, Σ)
```

Where:
- `μ ∈ ℝ^k` — posterior mean (replaces θ_u point estimate)
- `Σ ∈ ℝ^{k×k}` — posterior covariance (captures uncertainty)

### Update Rule

For each new observation `(problem_p, solved, timestamp)`:

1. Compute prior precision: `Λ_prior = Σ^{-1}` (with time-based precision decay)
2. Compute likelihood Hessian at current mean (Laplace approximation):
   ```
   pred = σ(μ · α_p + b_u + d_p)
   h = pred * (1 - pred)  // Hessian of log-likelihood
   Λ_obs = h * (α_p ⊗ α_p)  // outer product scaled by curvature
   ```
3. Update precision: `Λ_post = Λ_prior + w * Λ_obs` (w = time-decay weight)
4. Update mean: `μ_post = Λ_post^{-1} * (Λ_prior * μ_prior + w * (y - pred) * α_p)`
5. Store `Σ_post = Λ_post^{-1}`

### Time Decay via Precision Decay

Instead of decaying observation weights, decay the precision matrix over time:

```
Λ(t) = Λ_0 * decay^(Δt)
```

This naturally increases uncertainty for stale skills — if a user hasn't solved a DP problem in 2 years, their DP skill uncertainty grows, signaling that re-assessment is needed.

### Trade-offs

| | Approach A (SGD) | Approach B (Bayesian) |
|---|---|---|
| Update cost | O(n_obs × k × epochs) | O(k² + k³) per observation |
| Storage | theta (k floats) + hash | mu (k floats) + Sigma (k² floats) |
| Uncertainty | No | Yes (free from posterior) |
| Complexity | Low | High (matrix inversions, Laplace approx) |
| Accuracy | Good | Potentially better with small histories |
| Time decay | Weight-based | Precision decay (more principled) |

### When to Switch

Consider Approach B when:
- Users have 10,000+ observations and re-fitting takes >500ms
- Uncertainty estimates are needed for recommendation (explore vs exploit)
- Per-skill confidence display is implemented

---

## Future: Interpretable Skill Profiles

*Not implemented. Documented for future design.*

### Problem

The current MF dimensions are not purely interpretable. Tag-informed initialization creates partial correspondence (tag→dimension), but after training each dimension captures a mix of signals. Displaying "your DP skill is 0.82" from a latent dimension is approximate at best.

### Potential Approaches

**A. Constrained NMF (Non-negative Matrix Factorization)**
- Enforce non-negative α and θ — forces additive part-based decomposition
- Each dimension naturally corresponds to a "skill" that's present or absent
- Loses some accuracy vs unconstrained MF

**B. Post-hoc Rotation**
- Train unconstrained MF, then rotate the latent space to maximize alignment with known tags
- Procrustes-like alignment between learned dimensions and tag indicator vectors
- Preserves accuracy, interpretability is approximate

**C. Supervised Skill Decomposition**
- Define skills explicitly (from CF tags or a custom taxonomy)
- Train per-skill logistic regressors on solve data filtered by tag
- Fully interpretable but loses cross-skill correlations

**D. Sparse Coding**
- L1 regularization on α to encourage sparse problem representations
- Each problem activates only a few "skills"
- Combined with tag-informed init, could yield clean skill axes

### Recommendation

Start with **B (post-hoc rotation)** as it doesn't require retraining and can be evaluated against the existing model. If interpretability is insufficient, move to **A (constrained NMF)**.

### Display Design

```
Skill Profile for tourist:
  dp:              ████████████████░░░░  0.82
  graphs:          ████████████░░░░░░░░  0.61
  math:            ██████████░░░░░░░░░░  0.54
  data structures: ████████████████████  0.95
  ...

  Confidence: High (1,247 observations, last active 2 days ago)
```

Requires: interpretable dimensions + uncertainty estimates (Approach B).
