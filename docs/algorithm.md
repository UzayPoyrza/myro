# Myro Recommender: Algorithmic Deep-Dive

How myro-predict works — the math, the design choices, and the failure modes.

## 1. The Problem

Given a user's competitive programming history, predict the probability they can solve any problem in a catalog. Use these predictions to recommend problems at the right difficulty — hard enough to learn, easy enough to not be demoralizing.

This is a **collaborative filtering** problem: we have a sparse matrix of users × problems with binary outcomes (solved/not solved), and we want to fill in the missing entries.

## 2. Why Logistic Matrix Factorization

### 2.1 Alternatives Considered

**Elo / Rating-based.** The simplest approach: P(solve) = σ((user_rating - problem_rating) / 400). Uses one number per user and one per problem. This works surprisingly well (AUC 0.937 on our data) but has a fundamental limitation: it can't express that a user is strong at graphs but weak at DP. Every user is a single scalar, every problem is a single scalar. Two problems with the same rating are equally hard for every user.

**Item Response Theory (IRT).** The psychometrics standard. 1PL IRT is mathematically identical to Elo. 2PL IRT adds a per-item discrimination parameter. 3PL adds a guessing parameter. These are better but still limited to 1-2 dimensions.

**Bayesian Knowledge Tracing (BKT).** Models knowledge as a hidden Markov model with learn/forget transitions. Good for tracking mastery over time but assumes a flat set of independent skills — doesn't handle the reality that "dp" and "bitmask dp" share structure. Also requires pre-labeling which skills each problem tests.

**Deep learning (neural collaborative filtering, transformers).** Would work, but overkill for our data size (~20M observations, ~110K users, ~7.8K problems). Matrix factorization achieves 0.965 AUC already. Deep models add complexity without clear gains at this scale, and they're much harder to interpret.

**Our choice: Logistic Matrix Factorization.** It sits in a sweet spot:
- Multi-dimensional (captures skill profiles, not just overall ability)
- Interpretable (each dimension ≈ a skill axis)
- Fast to train (SGD, minutes not hours)
- Fast to infer (one dot product per prediction)
- Cold-start friendly (fitting a new user is convex optimization)

### 2.2 Relationship to IRT

Our model is a **multidimensional IRT model** (MIRT). The standard 2PL IRT model is:

```
P(solve) = σ(a_p · (θ_u - b_p))
```

where `a_p` is item discrimination and `b_p` is item difficulty. Our model generalizes this to k dimensions:

```
P(solve) = σ(θ_u · α_p + b_u + d_p)
```

This is equivalent to MIRT with:
- `θ_u ∈ ℝ^k` — user ability vector (k latent skills)
- `α_p ∈ ℝ^k` — problem discrimination/loading vector (how much each skill matters)
- `b_u ∈ ℝ` — user bias (overall ability beyond the k skills)
- `d_p ∈ ℝ` — problem easiness (negative of difficulty)

The key difference from standard MF (like Netflix Prize matrix factorization) is the **logistic link function**. Standard MF minimizes squared error on real-valued ratings. We minimize binary cross-entropy on solve/no-solve outcomes, which gives proper probability estimates in [0, 1].

## 3. The Model

### 3.1 Forward Pass

For user `u` and problem `p`:

```
z = θ_u · α_p + b_u + d_p

P(solve) = σ(z) = 1 / (1 + exp(-z))
```

where:
- **θ_u ∈ ℝ^k** — user latent skill vector
- **α_p ∈ ℝ^k** — problem latent requirement vector
- **b_u ∈ ℝ** — user ability bias
- **d_p ∈ ℝ** — problem easiness bias
- **k** — number of latent dimensions (we use 20–30)

The dot product `θ_u · α_p = Σ_d θ_u[d] · α_p[d]` measures skill-requirement alignment across all dimensions simultaneously. If dimension d=2 represents DP skill, then a user with high `θ_u[2]` and a problem with high `α_p[2]` will have a higher predicted solve probability — but only along that axis. The biases capture the residual: `b_u` represents "this user is generally strong/weak" and `d_p` represents "this problem is generally easy/hard."

### 3.2 Loss Function

Binary cross-entropy with L2 regularization:

```
L = -Σ_{(u,p)} [y · log(p̂) + (1-y) · log(1-p̂)] + λ · R

where:
  y ∈ {0, 1}         — did user u solve problem p?
  p̂ = σ(θ_u · α_p + b_u + d_p)
  R = ||θ_u||² + ||α_p||² + b_u² + d_p²    — L2 regularization
  λ = 0.01           — regularization strength
```

**Why cross-entropy, not MSE?** For binary outcomes, MSE treats a prediction of 0.9 on a positive example as "0.01 error" — barely different from 0.95 or 0.99. Cross-entropy gives `-log(0.9) = 0.105` vs `-log(0.99) = 0.01` vs `-log(0.5) = 0.693`. It properly penalizes confident wrong predictions and encourages well-calibrated probabilities.

**Why L2 regularization?** Without regularization, the model can push parameters to extreme values to perfectly fit the training data. A user who solved 3 easy problems would get θ pushed to huge values, predicting P(solve) ≈ 1.0 for everything. L2 regularization (also called weight decay) adds a cost proportional to parameter magnitude, keeping values small and predictions conservative. λ=0.01 is mild — just enough to prevent divergence.

### 3.3 Training (SGD)

Stochastic gradient descent on the full dataset. For each observation (u, p, y):

```
error = p̂ - y        (prediction minus truth; positive if overconfident, negative if underconfident)

∂L/∂θ_u[d] = error · α_p[d] + λ · θ_u[d]
∂L/∂α_p[d] = error · θ_u[d] + λ · α_p[d]
∂L/∂b_u    = error           + λ · b_u
∂L/∂d_p    = error           + λ · d_p

Update rule (for each parameter w):
  w ← w - lr · ∂L/∂w
```

The gradient derivation comes from the chain rule. Since `∂σ(z)/∂z = σ(z)(1-σ(z))` and `∂L/∂p̂ = (p̂ - y) / (p̂(1-p̂))`, we get `∂L/∂z = p̂ - y`, which is pleasantly simple. This is the same gradient as logistic regression — the matrix factorization structure just determines how z depends on the parameters.

**Hyperparameters:**
- Learning rate: 0.01
- Epochs: 30–50 (full passes over the dataset)
- Regularization: λ = 0.01
- Latent dimensions: k = 20–30

### 3.4 Tag-Informed Initialization

Standard MF initializes parameters randomly (small Gaussian noise). This works but is slow to converge and produces uninterpretable dimensions.

Our approach: assign each of the k most frequent Codeforces tags to one latent dimension. Then initialize:

```
α_p[d] = 0.3   if problem p has the tag assigned to dimension d
α_p[d] = 0.01  otherwise (small random noise)
d_p    = -(problem_rating - 1500) / 500   (from CF's official rating)
```

This gives SGD a meaningful starting point:
- Dimension 0 starts correlated with "implementation" problems
- Dimension 1 starts correlated with "math" problems
- Dimension 2 starts correlated with "dp" problems
- etc.

**Does this survive training?** Yes — strongly. After 30 epochs, the diagonal structure in the tag-dimension heatmap is preserved and even sharpened (see REPORT.md Section 5). The model reinforces the initialization rather than overwriting it, because the tag structure genuinely corresponds to the underlying skill decomposition.

**Why not hard-code the tags?** Because the learned dimensions end up encoding *more* than just the assigned tag. Dimension 8 (graphs) also learns to activate on "dfs and similar" problems — the model discovers that DFS is a graph skill without being told. The initialization is a guide, not a constraint.

### 3.5 Difficulty Bias Initialization

The problem difficulty bias `d_p` is initialized from CF's official problem rating:

```
d_p = -(rating - 1500) / 500
```

So a 2000-rated problem starts with `d_p = -1.0` (hard, negative bias reduces P(solve)), and an 800-rated problem starts with `d_p = +1.4` (easy, positive bias increases P(solve)). After training, the learned `d_p` correlates at r = -0.881 with the CF rating — the model discovers the rating scale from data alone, but with corrections for problems that are harder or easier than their rating suggests.

## 4. Cold-Start User Fitting

### 4.1 The Core Idea

After training, we freeze all problem parameters (α_p, d_p for every problem) and **throw away all user parameters**. We ship only the `ProblemModel`.

For any new user, we:
1. Fetch their Codeforces submission history (public API, no auth needed)
2. Map submissions to problems in our model
3. Fit user parameters (θ_u, b_u) by gradient descent against their history

This is **transfer learning**: the problem embeddings encode the structure of competitive programming skills. A new user's data is used only to locate them within this pre-learned skill space.

### 4.2 Why This Is Convex

With problem parameters fixed, the logit is:

```
z = θ · α_p + b + d_p
```

This is **linear in the parameters (θ, b)**. Logistic regression on a linear model is convex — there is a unique global optimum. This means:
- SGD converges reliably regardless of initialization
- No local minima, no sensitivity to learning rate (within reason)
- 100 iterations is always enough
- The solution is deterministic (up to floating-point precision)

This is the key architectural advantage: training the full model (with both user and problem parameters) is **non-convex** (the dot product θ·α makes it bilinear). But fitting a single user against fixed problems is convex. We pay the non-convex optimization cost once during training, then get cheap, reliable optimization for every new user.

### 4.3 Time-Weighted Observations

Not all history is equally relevant. A problem solved 3 years ago says less about current ability than one solved last week. We weight each observation by exponential time decay:

```
w = 2^(-days_ago / half_life)

where half_life = 365 days (default)
```

So:
- A problem solved today: w = 1.0
- 6 months ago: w ≈ 0.71
- 1 year ago: w ≈ 0.50
- 2 years ago: w ≈ 0.25
- 5 years ago: w ≈ 0.03

The weight scales the gradient:

```
∂L/∂θ[d] = w · (p̂ - y) · α_p[d] + λ · θ[d]
```

This means old failures barely penalize the model, while recent improvements dominate. A user who was weak at DP two years ago but has been solving DP problems recently will correctly get high DP predictions.

**Why exponential decay?** It has a single interpretable parameter (half-life), decays smoothly, and never reaches zero (very old observations still contribute a tiny amount). The half-life of 365 days was chosen to roughly match competitive programming skill decay rates — skills degrade slowly if not practiced, but don't vanish.

### 4.4 What Counts as an Observation

From a user's CF submission history (`user.status` API):

- **Solved (y=1):** At least one submission with verdict "OK" for this problem
- **Failed (y=0):** User submitted but never got "OK"
- **No signal:** User never submitted — this problem is **not** included as an observation

This is a critical distinction. We do **not** treat "never attempted" as "failed." The absence of a submission is not evidence of inability — the user may simply have never encountered the problem. Including non-attempts as negatives would massively bias the model toward predicting failure (since the average user has attempted <1% of all problems).

### 4.5 Batch Prediction

Once user parameters are fitted, we predict P(solve) for all ~2500 problems simultaneously:

```
logits = A · θ + d + b     (A is n_problems × k matrix)
P(solve) = σ(logits)       (element-wise sigmoid)
```

This is a single matrix-vector multiply (GEMV). For n=2500, k=30: 75,000 multiply-adds = microseconds. No need for BLAS or GPU — plain Rust loops are fast enough.

## 5. The Recommendation Loop

### 5.1 Target Probability

The user configures a target solve probability `p` (default 0.5). The recommender picks a random unsolved problem from the band `[p - 0.1, p + 0.1]`.

**Why 0.5?** Research on the "zone of proximal development" (Vygotsky) and "desirable difficulties" (Bjork) suggests that learning is maximized when success probability is around 50% — hard enough to require effort, easy enough to be achievable. But this is configurable: anxious learners might prefer 0.6–0.7, and ambitious ones might want 0.3–0.4.

**Why random within the band?** Rather than always picking the problem closest to p, randomness provides variety and prevents the recommender from getting stuck on one problem type. The model's predictions have uncertainty — two problems at P(solve) = 0.48 and 0.52 are effectively indistinguishable.

### 5.2 Live Embedding Updates

After each solve or failure:
1. Record the outcome in local `SolveHistory`
2. Refit the user embedding against the updated history
3. Recompute all predictions
4. Next recommendation uses the updated predictions

This creates a feedback loop: solving a hard DP problem increases the user's DP dimension in θ, which in turn changes which problems the model recommends. The user's skill profile evolves in real-time.

### 5.3 The /isuck Signal

When a user gives up on a problem (`/isuck` command), it's recorded as `solved=false`. This provides genuine negative signal — the model learns "this problem was too hard for this user right now." The user's embedding shifts slightly, and the next recommendation will account for this difficulty.

This is better than simply skipping — a skip provides no information, while an explicit "I can't solve this" tells the model something real about the user's current abilities.

## 6. Choosing k (Latent Dimensions)

### 6.1 What k Controls

k is the dimensionality of the skill space. Each dimension is (loosely) a "skill axis."

- **k=1:** Equivalent to Elo. One number per user, one per problem. Can only express "generally strong/weak."
- **k=5:** Can distinguish broad categories (math vs. graphs vs. DP) but can't differentiate sub-skills.
- **k=20:** Our default. Roughly one dimension per major CF tag. Can express "strong at DP, weak at geometry, medium at graphs."
- **k=50+:** Diminishing returns. The 30th dimension captures very subtle patterns (e.g., difference between two-pointer and sliding-window problems). Risk of overfitting with limited data.

### 6.2 How to Choose

The bias-variance tradeoff:
- **Too small k:** Underfitting. Can't capture the skill structure. High bias.
- **Too large k:** Overfitting. Learns noise in the training data. High variance. Also slower to train and fit.

In practice, k=20–30 works well for ~2500 problems with ~2M observations. We chose k=20 for our initial experiments and k=30 for the expanded dataset.

**Empirical test:** AUC on holdout data as a function of k would show diminishing returns past k~20. We use k=30 in production (20 tag-assigned + 10 free dimensions), achieving AUC 0.965 on holdout evaluation.

## 7. Failure Modes and Limitations

### 7.1 Cold-Start Problems (Not Users)

The model cannot predict on problems not in the training set. If a new CF contest creates 6 new problems, they have no α_p or d_p — we can't make predictions. Fallback options:
- Use the Elo baseline (P(solve) = σ((user_rating - problem_rating) / 400))
- Use tag-based heuristics to find similar known problems
- Wait until the problem is added to the model in a retraining cycle

### 7.2 Submission-Only Signal

We only observe problems the user has submitted to. This creates **selection bias**: users tend to submit to problems they think they can solve. A user with 50 solved problems and 0 failures might look omnipotent, but they may have simply been conservative in their problem selection.

The model partially handles this via the difficulty bias — if a user only solves easy problems, they'll have a moderate bias b_u, and their θ won't have strong activations in hard-skill dimensions. But it would be better to also have **contest participation** signal: "user entered contest X, saw problem Y, chose not to attempt it" → soft negative signal.

### 7.3 Temporal Dynamics

The time-weighted fitting handles gradual skill improvement, but it can't model abrupt changes (e.g., a user studies graph theory intensively for a week and suddenly gets much better). The exponential decay is symmetric — it doesn't know that skills improve faster than they decay.

### 7.4 Problem Similarity

The model treats each problem independently. Two very similar problems (e.g., same algorithm, different input format) get independent embeddings that may or may not be similar. The model can't generalize from "user solved problem A, which is similar to problem B" unless both problems share similar training observations across the user population.

### 7.5 Calibration at Extremes

The model is well-calibrated in the 30-70% probability range but less so at extremes. A prediction of P(solve) = 0.95 might in reality be 0.90 or 0.99 — the sigmoid squashes everything near the boundaries, making fine distinctions hard. This matters less for recommendation (we mostly care about the 0.3–0.7 range) but affects metrics like log-loss.

### 7.6 Population Bias

The model learns from the Codeforces population, which skews toward competitive programmers, not the general developer population. Predictions are calibrated for "how likely is a CF-active user to solve this" — not "how likely is a random programmer to solve this."

### 7.7 Tag Taxonomy Lock-In

The tag-informed initialization ties our dimensions to CF's tag taxonomy, which is imperfect. Some CF tags are too broad ("implementation"), some overlap ("dfs and similar" vs "graphs"), and some important skills have no tag (amortized complexity analysis, problem-specific observations). The model can partially work around this via off-diagonal activations, but the taxonomy shapes what the model can learn.

## 8. Comparison to What Codeforces Does

Codeforces uses a modified Elo system for user ratings and a separate system for problem ratings (based on actual solve rates). Their approach is:
- 1-dimensional (single rating)
- Population-calibrated (ratings reflect relative standing)
- Well-established (10+ years of rating data)

Our model is complementary, not competitive:
- Multi-dimensional (20+ skill axes)
- Personalized to the individual (your specific strengths and weaknesses)
- Designed for recommendation, not ranking

The Elo baseline achieves AUC 0.937 — already excellent. Our model's 0.965 represents a 44% reduction in ranking errors. The value isn't that our predictions are hugely better in aggregate, but that they're **personalized** — they can distinguish between two 1600-rated users where one is strong at DP and weak at graphs, and the other is the reverse. Elo treats them identically.

## 9. Mathematical Notation Summary

| Symbol | Meaning | Shape |
|--------|---------|-------|
| θ_u | User u's latent skill vector | ℝ^k |
| α_p | Problem p's latent requirement vector | ℝ^k |
| b_u | User ability bias | ℝ |
| d_p | Problem easiness bias | ℝ |
| k | Number of latent dimensions | scalar (20–30) |
| σ(·) | Sigmoid function: 1/(1+e^(-x)) | ℝ → (0,1) |
| y | Binary solve outcome | {0, 1} |
| p̂ | Predicted P(solve) | (0, 1) |
| λ | L2 regularization strength | scalar (0.01) |
| w | Time decay weight | (0, 1] |
| A | Problem embedding matrix (all α_p stacked) | n_problems × k |

## 10. Key Implementation Files

| File | What it does |
|------|-------------|
| `crates/myro-predict/src/model/types.rs` | `ProblemModel`, `UserParams`, `ProblemParams`, `WeightedObservation` |
| `crates/myro-predict/src/model/train.rs` | Full SGD training loop (both user and problem params) |
| `crates/myro-predict/src/model/inference.rs` | `fit_user_weighted`, `predict_all`, `build_observations_from_submissions` |
| `crates/myro-predict/src/model/eval.rs` | AUC, log-loss, baseline comparisons, cold-start eval |
| `crates/myro-predict/src/db/model_store.rs` | Serialization (bincode + gzip) |
| `crates/myro-predict/REPORT.md` | Evaluation results with plots |
| `crates/myro-predict/src/model/skills.rs` | Per-tag skill ratings from embeddings |

## 11. Interpreting Learned Dimensions as Skill Ratings

The tag-informed initialization maps CF problem tags to specific latent dimensions. After training, we can extract **per-tag effective ratings** by finding where P(solve) = 0.5 for each tag's problems. This gives users an interpretable skill profile (e.g., "dp: 1847, graphs: 1623").

The algorithm bins problems by 200-rating intervals, computes mean P(solve) per bin, and linearly interpolates to find the 0.5 crossing. See [docs/skill-rating.md](skill-rating.md) for the full algorithm, edge cases, and limitations.
