# Cold-Start Prediction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the cold-start problem in myro-predict by always computing user embeddings on-the-fly from time-weighted solve history, shipping only problem embeddings.

**Architecture:** Split `SolvePredictionModel` into `ProblemModel` (shipped, ~200KB) and `UserProfile` (local, cached). All users — new and existing — get their skill vector fitted from CF contest history + myro-tui history via time-weighted SGD. Cache fitted params locally with hash-based invalidation.

**Tech Stack:** Rust (existing myro-predict crate), serde/bincode for serialization, SHA-256 for cache invalidation, chrono for timestamps.

---

### Task 1: Add `ProblemModel` type and timestamp to observations

**Files:**
- Modify: `crates/myro-predict/src/model/types.rs`

**Step 1: Write the failing test**

Add to `crates/myro-predict/tests/integration_test.rs`:

```rust
#[test]
fn test_time_decay_weight() {
    // 0 days ago → weight 1.0
    let w0 = time_decay_weight(0.0, 365.0);
    assert!((w0 - 1.0).abs() < 1e-10, "0 days ago should give weight 1.0, got {}", w0);

    // 365 days ago → weight 0.5
    let w365 = time_decay_weight(365.0, 365.0);
    assert!((w365 - 0.5).abs() < 1e-10, "365 days ago should give weight 0.5, got {}", w365);

    // 730 days ago → weight 0.25
    let w730 = time_decay_weight(730.0, 365.0);
    assert!((w730 - 0.25).abs() < 1e-10, "730 days ago should give weight 0.25, got {}", w730);
}

fn time_decay_weight(days_ago: f64, half_life_days: f64) -> f64 {
    2.0_f64.powf(-days_ago / half_life_days)
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test -p myro-predict test_time_decay_weight`
Expected: PASS (this is a standalone function in the test file — validates the math)

**Step 3: Add `ProblemModel` type and `WeightedObservation`**

In `crates/myro-predict/src/model/types.rs`, add:

```rust
/// A problem-only model for inference. Ships without user params.
/// User params are always fitted on-the-fly from history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemModel {
    pub config: ModelConfig,
    pub problem_params: Vec<ProblemParams>,
    pub problem_index: HashMap<String, usize>,
    pub problem_ratings: Vec<Option<i32>>,
    pub problem_tags: Vec<Vec<String>>,
    pub tag_dim_map: HashMap<String, usize>,
}
```

Add a `From<SolvePredictionModel>` impl for easy conversion:

```rust
impl From<SolvePredictionModel> for ProblemModel {
    fn from(model: SolvePredictionModel) -> Self {
        ProblemModel {
            config: model.config,
            problem_params: model.problem_params,
            problem_index: model.problem_index,
            problem_ratings: model.problem_ratings,
            problem_tags: model.problem_tags,
            tag_dim_map: model.tag_dim_map,
        }
    }
}
```

Add `WeightedObservation` for the fitting API:

```rust
/// An observation with a time-decay weight for user fitting.
#[derive(Debug, Clone)]
pub struct WeightedObservation {
    pub problem_idx: usize,
    pub solved: bool,
    pub weight: f64,
}
```

**Step 4: Run `cargo check -p myro-predict` to verify compilation**

Expected: PASS

**Step 5: Commit**

```bash
git add crates/myro-predict/src/model/types.rs crates/myro-predict/tests/integration_test.rs
git commit -m "feat(predict): add ProblemModel type and WeightedObservation"
```

---

### Task 2: Implement time-weighted `fit_user`

**Files:**
- Modify: `crates/myro-predict/src/model/inference.rs`
- Modify: `crates/myro-predict/tests/integration_test.rs`

**Step 1: Write the failing test**

Add to `crates/myro-predict/tests/integration_test.rs`:

```rust
/// Test that time-weighted fitting gives more weight to recent observations.
/// A user who was weak in the past but strong recently should have a positive bias.
#[test]
fn test_time_weighted_fitting_recency_bias() {
    let k = 3;
    // Simple problem: alpha = [1, 0, 0], difficulty = 0
    let prob_alpha = vec![1.0, 0.0, 0.0];
    let prob_diff = 0.0;

    // Simulate: user failed this problem many times long ago (low weight),
    // but solved it recently (high weight).
    // With time decay, the recent solve should dominate.

    // 10 failures at weight 0.1 (old) + 1 solve at weight 1.0 (recent)
    // The recent solve should push the user toward positive predictions.
    struct WObs {
        solved: bool,
        weight: f64,
    }

    let observations = vec![
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: false, weight: 0.1 },
        WObs { solved: true, weight: 1.0 },  // recent solve
    ];

    // Fit user params via weighted SGD
    let mut theta = vec![0.0; k];
    let mut bias = 0.0;
    let lr = 0.05;
    let lambda = 0.01;

    for _ in 0..100 {
        for obs in &observations {
            let logit: f64 = theta.iter().zip(&prob_alpha).map(|(t, a)| t * a).sum::<f64>() + bias + prob_diff;
            let pred = sigmoid(logit);
            let y = if obs.solved { 1.0 } else { 0.0 };
            let error = pred - y;

            for dim in 0..k {
                theta[dim] -= lr * (obs.weight * error * prob_alpha[dim] + lambda * theta[dim]);
            }
            bias -= lr * (obs.weight * error + lambda * bias);
        }
    }

    // The final prediction should be > 0.5 because the recent solve (weight=1.0)
    // outweighs the 10 old failures (total weight=1.0, but spread across negatives)
    let final_pred = sigmoid(theta.iter().zip(&prob_alpha).map(|(t, a)| t * a).sum::<f64>() + bias + prob_diff);
    assert!(
        final_pred > 0.45,
        "Recent solve should pull prediction up despite old failures, got {}",
        final_pred
    );

    // Now test WITHOUT time decay (all weight=1.0) — should predict < 0.5
    let mut theta_unweighted = vec![0.0; k];
    let mut bias_unweighted = 0.0;
    for _ in 0..100 {
        for obs in &observations {
            let logit: f64 = theta_unweighted.iter().zip(&prob_alpha).map(|(t, a)| t * a).sum::<f64>() + bias_unweighted + prob_diff;
            let pred = sigmoid(logit);
            let y = if obs.solved { 1.0 } else { 0.0 };
            let error = pred - y;

            for dim in 0..k {
                theta_unweighted[dim] -= lr * (1.0 * error * prob_alpha[dim] + lambda * theta_unweighted[dim]);
            }
            bias_unweighted -= lr * (1.0 * error + lambda * bias_unweighted);
        }
    }
    let unweighted_pred = sigmoid(theta_unweighted.iter().zip(&prob_alpha).map(|(t, a)| t * a).sum::<f64>() + bias_unweighted + prob_diff);

    // Weighted prediction should be higher than unweighted (recency bias)
    assert!(
        final_pred > unweighted_pred,
        "Weighted pred ({}) should be higher than unweighted ({})",
        final_pred, unweighted_pred
    );
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test -p myro-predict test_time_weighted_fitting_recency_bias`
Expected: PASS (self-contained test validating the math)

**Step 3: Update `fit_user` in inference.rs**

Replace the existing `fit_user` function in `crates/myro-predict/src/model/inference.rs`:

```rust
/// Fit user parameters from weighted observations.
///
/// Holds problem parameters fixed and optimizes user params (θ_u, b_u)
/// via weighted gradient descent. Each observation's gradient is scaled
/// by its weight (typically a time-decay factor).
pub fn fit_user_weighted(
    model: &ProblemModel,
    observations: &[WeightedObservation],
    lr: f64,
    epochs: usize,
    lambda: f64,
) -> UserParams {
    let k = model.config.latent_dim;
    let mut user = UserParams {
        theta: vec![0.0; k],
        bias: 0.0,
    };

    for _ in 0..epochs {
        for obs in observations {
            let problem = &model.problem_params[obs.problem_idx];
            let pred = predict(&user, problem);
            let y = if obs.solved { 1.0 } else { 0.0 };
            let error = pred - y;

            for dim in 0..k {
                let grad = obs.weight * error * problem.alpha[dim] + lambda * user.theta[dim];
                user.theta[dim] -= lr * grad;
            }
            let grad_b = obs.weight * error + lambda * user.bias;
            user.bias -= lr * grad_b;
        }
    }

    user
}
```

Keep the old `fit_user` but have it delegate to `fit_user_weighted` with weight=1.0 for backwards compatibility:

```rust
pub fn fit_user(
    model: &ProblemModel,
    solves: &[(usize, bool)],
    lr: f64,
    epochs: usize,
    lambda: f64,
) -> UserParams {
    let observations: Vec<WeightedObservation> = solves
        .iter()
        .map(|&(problem_idx, solved)| WeightedObservation {
            problem_idx,
            solved,
            weight: 1.0,
        })
        .collect();
    fit_user_weighted(model, &observations, lr, epochs, lambda)
}
```

Note: both `fit_user` and `fit_user_weighted` now take `&ProblemModel` instead of `&SolvePredictionModel`. Update the import and any call sites.

**Step 4: Add time decay utility function**

Add to `crates/myro-predict/src/model/inference.rs`:

```rust
/// Compute time-decay weight: w = 2^(-days_ago / half_life_days).
/// half_life_days = 365 means a solve from 1 year ago has weight 0.5.
pub fn time_decay_weight(days_ago: f64, half_life_days: f64) -> f64 {
    2.0_f64.powf(-days_ago / half_life_days)
}

/// Default half-life for time decay (365 days).
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 365.0;
```

**Step 5: Run all tests**

Run: `cargo test -p myro-predict`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/myro-predict/src/model/inference.rs crates/myro-predict/tests/integration_test.rs
git commit -m "feat(predict): time-weighted fit_user with decay half-life"
```

---

### Task 3: `ProblemModel` serialization

**Files:**
- Modify: `crates/myro-predict/src/db/model_store.rs`

**Step 1: Add save/load functions for ProblemModel**

```rust
use crate::model::types::{ProblemModel, SolvePredictionModel};

pub fn save_problem_model(model: &ProblemModel, path: &Path) -> Result<()> {
    let encoded = bincode::serialize(model).context("Failed to serialize problem model")?;
    let file = File::create(path)
        .with_context(|| format!("Failed to create model file at {}", path.display()))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&encoded).context("Failed to write compressed model")?;
    encoder.finish().context("Failed to finalize gzip stream")?;
    Ok(())
}

pub fn load_problem_model(path: &Path) -> Result<ProblemModel> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open model file at {}", path.display()))?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).context("Failed to decompress model")?;
    let model: ProblemModel =
        bincode::deserialize(&buf).context("Failed to deserialize problem model")?;
    Ok(model)
}
```

Keep `save_model`/`load_model` for the full `SolvePredictionModel` (used during training/eval).

**Step 2: Run `cargo check -p myro-predict`**

Expected: PASS

**Step 3: Commit**

```bash
git add crates/myro-predict/src/db/model_store.rs
git commit -m "feat(predict): add ProblemModel serialization"
```

---

### Task 4: Add `export-model` CLI subcommand

**Files:**
- Modify: `crates/myro-predict/src/main.rs`

**Step 1: Add the subcommand to the CLI**

Add to the `Commands` enum:

```rust
/// Export problem-only model (strips user embeddings) for shipping with myro-tui
ExportModel {
    /// Full trained model path (input)
    #[arg(long, default_value = "model.bin.gz")]
    model_path: PathBuf,

    /// Problem-only model output path
    #[arg(long, default_value = "problem_model.bin.gz")]
    output: PathBuf,
},
```

**Step 2: Add the handler in the match block**

```rust
Commands::ExportModel { model_path, output } => {
    let full_model = db::model_store::load_model(&model_path)?;
    let problem_model: model::types::ProblemModel = full_model.into();
    db::model_store::save_problem_model(&problem_model, &output)?;
    println!(
        "Exported problem model: {} problems, {} dimensions",
        problem_model.problem_params.len(),
        problem_model.config.latent_dim
    );
    println!("Saved to {}", output.display());
}
```

**Step 3: Run `cargo check -p myro-predict`**

Expected: PASS

**Step 4: Commit**

```bash
git add crates/myro-predict/src/main.rs
git commit -m "feat(predict): add export-model subcommand for problem-only model"
```

---

### Task 5: Rewrite `query` to always fit from history with time decay

**Files:**
- Modify: `crates/myro-predict/src/model/inference.rs`

**Step 1: Rewrite `run_query`**

The new `run_query` should:
1. Load a `ProblemModel` (not `SolvePredictionModel`)
2. Fetch CF submission history via API
3. Build `WeightedObservation` list with time-decay weights
4. Call `fit_user_weighted`
5. Remove the "user found in model" vs "user not in model" branching

Key changes to `run_query`:
- Replace `model_store::load_model` with `model_store::load_problem_model`
- Build observations with timestamps from CF submissions
- Use `time_decay_weight` on each observation
- For CF contest history: both solved and unsolved-but-participated count
- Call `fit_user_weighted` instead of using stored params

The CF API `fetch_user_status` returns submissions with `creationTimeSeconds`. For contest-level data, we need `fetch_user_rating` (contest participation history) to know which contests the user entered (for negative signal on unsolved problems). However, the simpler approach is:
- From `fetch_user_status`: extract all submissions, group by contest+problem
- Mark `solved=true` if any submission has verdict "OK"
- Mark `solved=false` if the user submitted to that problem but never got "OK"
- Problems the user never submitted to in their contests are not included (we don't have contest participation data from user.status alone)

Actually, for richer signal, also use `fetch_user_rating` to get the list of contests the user participated in, then cross-reference with problems in those contests (from the model's problem_index) to get negative signal.

The implementation should:
1. `fetch_user_status(handle)` → get all submissions → build solved set per problem
2. `fetch_user_rating(handle)` → get contest participation list → for each contest that has problems in the model, add unsolved problems as negative signal
3. Apply time decay weights based on contest/submission timestamp
4. Fit and predict

**Step 2: Update the function signature and implementation**

```rust
pub async fn run_query(
    handle: &str,
    model_path: &Path,
    problems: Option<String>,
    top_n: Option<usize>,
) -> Result<()> {
    let model = model_store::load_problem_model(model_path)?;
    let client = CfClient::new();

    println!("Fetching solve history for {}...", handle);
    let submissions = client
        .fetch_user_status(handle)
        .await
        .context("Failed to fetch user submissions")?;

    let now = chrono::Utc::now().timestamp();

    // Build solve map: problem_key → (solved, timestamp)
    let mut problem_outcomes: HashMap<String, (bool, i64)> = HashMap::new();
    for sub in &submissions {
        if let Some(cid) = sub.contest_id {
            let key = format!("{}:{}", cid, sub.problem.index);
            let is_ok = sub.verdict.as_deref() == Some("OK");
            let ts = sub.creation_time_seconds.unwrap_or(0);

            problem_outcomes
                .entry(key)
                .and_modify(|(solved, existing_ts)| {
                    if is_ok {
                        *solved = true;
                    }
                    // Keep the latest timestamp
                    if ts > *existing_ts {
                        *existing_ts = ts;
                    }
                })
                .or_insert((is_ok, ts));
        }
    }

    // Build weighted observations for problems in the model
    let mut observations: Vec<WeightedObservation> = Vec::new();
    for (key, (solved, ts)) in &problem_outcomes {
        if let Some(&idx) = model.problem_index.get(key) {
            let days_ago = (now - ts) as f64 / 86400.0;
            let weight = time_decay_weight(days_ago.max(0.0), DEFAULT_HALF_LIFE_DAYS);
            observations.push(WeightedObservation {
                problem_idx: idx,
                solved: *solved,
                weight,
            });
        }
    }

    if observations.is_empty() {
        bail!("No overlap between user's history and model problems");
    }

    let solved_count = observations.iter().filter(|o| o.solved).count();
    println!(
        "Found {} problems in model ({} solved, {} unsolved)",
        observations.len(),
        solved_count,
        observations.len() - solved_count,
    );

    println!("Fitting user parameters with time-weighted decay (half-life: {} days)...",
        DEFAULT_HALF_LIFE_DAYS as i64);
    let user_params = fit_user_weighted(&model, &observations, 0.01, 100, 0.01);

    // ... rest of the query output logic (same as before, but using &model instead of &model)
}
```

Note: remove the `_db_path` parameter from the function signature since it's no longer needed.

**Step 3: Update the `Commands::Query` in main.rs**

Remove `db_path` from the Query variant and update the call:

```rust
Commands::Query {
    handle,
    model_path,
    problems,
    top_n,
} => {
    model::inference::run_query(&handle, &model_path, problems, top_n).await?;
}
```

**Step 4: Run `cargo check -p myro-predict`**

Check that `fetch_user_status` returns `creation_time_seconds` in the submission struct. If not, check `crates/myro-cf/src/lib.rs` for the `CfSubmission` type and add the field if missing.

Expected: PASS after any necessary myro-cf adjustments

**Step 5: Commit**

```bash
git add crates/myro-predict/src/model/inference.rs crates/myro-predict/src/main.rs
git commit -m "feat(predict): rewrite query to always fit from history with time decay"
```

---

### Task 6: Add `fit-from-history` eval mode

**Files:**
- Modify: `crates/myro-predict/src/model/eval.rs`
- Modify: `crates/myro-predict/src/main.rs`

**Step 1: Add a new eval mode that simulates cold-start fitting**

The idea: for each user in the test set, fit their params from their training-set history using `fit_user_weighted` (with time decay), then predict on their test-set observations. This simulates the real cold-start flow.

Add to `eval.rs`:

```rust
/// Cold-start evaluation: for each test user, fit params from their
/// training history using time-weighted SGD, then predict on test observations.
pub fn run_coldstart_eval(
    full_model: &SolvePredictionModel,
    train_data: &TrainingDataset,
    test_data: &TrainingDataset,
    cutoff_timestamp: i64,
    verbose: bool,
) -> Result<()> {
    let problem_model: ProblemModel = full_model.clone().into();

    // Group train observations by user
    let mut user_train_obs: HashMap<usize, Vec<WeightedObservation>> = HashMap::new();
    // We need timestamps — use cutoff as approximate timestamp for all training obs
    // (In production, we'd have real timestamps; for eval this is a simplification)
    for obs in &train_data.observations {
        // Map train problem indices to model problem indices
        let train_key = train_data.problem_index.iter()
            .find(|(_, &v)| v == obs.problem_idx)
            .map(|(k, _)| k.clone());
        if let Some(key) = train_key {
            if let Some(&model_idx) = problem_model.problem_index.get(&key) {
                user_train_obs.entry(obs.user_idx).or_default().push(WeightedObservation {
                    problem_idx: model_idx,
                    solved: obs.solved,
                    weight: 1.0, // All training obs treated equally for this eval
                });
            }
        }
    }

    // For each test observation, fit user from train history and predict
    let mut predictions: Vec<(f64, bool)> = Vec::new();
    let mut predictions_with_rating: Vec<(f64, bool, Option<i32>)> = Vec::new();
    let mut user_cache: HashMap<usize, UserParams> = HashMap::new();

    for obs in &test_data.observations {
        // Map test problem to model problem
        let test_key = test_data.problem_index.iter()
            .find(|(_, &v)| v == obs.problem_idx)
            .map(|(k, _)| k.clone());
        let model_problem_idx = test_key.as_ref()
            .and_then(|k| problem_model.problem_index.get(k))
            .copied();

        if let Some(mp_idx) = model_problem_idx {
            let user = user_cache.entry(obs.user_idx).or_insert_with(|| {
                let train_obs = user_train_obs.get(&obs.user_idx)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                fit_user_weighted(&problem_model, train_obs, 0.01, 100, 0.01)
            });

            let pred = predict(user, &problem_model.problem_params[mp_idx]);
            predictions.push((pred, obs.solved));
            predictions_with_rating.push((pred, obs.solved, obs.problem_rating));
        }
    }

    // Print results
    println!("\nCold-start eval (fit-from-history): {} predictions", predictions.len());
    let auc = compute_auc(&predictions);
    let ll = compute_logloss(&predictions);
    println!("{:<30} {:>8.4} {:>10.4} {:>8}", "MF cold-start (ours)", auc, ll, predictions.len());

    // Per-band breakdown
    if verbose && !predictions_with_rating.is_empty() {
        println!("\nPer-rating-band breakdown (cold-start):");
        println!("{:<15} {:>8} {:>10} {:>8}", "Band", "AUC", "Log-loss", "N");
        println!("{}", "-".repeat(45));
        for (band, auc, ll, n) in per_band_metrics(&predictions_with_rating) {
            println!("{:<15} {:>8.4} {:>10.4} {:>8}", band, auc, ll, n);
        }
    }

    Ok(())
}
```

**Step 2: Add CLI option**

Add `"coldstart"` as a valid `eval_mode` in the Eval handler in `main.rs`.

**Step 3: Run `cargo check -p myro-predict`**

Expected: PASS

**Step 4: Commit**

```bash
git add crates/myro-predict/src/model/eval.rs crates/myro-predict/src/main.rs
git commit -m "feat(predict): add cold-start eval mode (fit-from-history)"
```

---

### Task 7: Myro-tui history storage

**Files:**
- Create: `crates/myro-predict/src/history.rs` (or `crates/myro-tui/src/history.rs` — see note)
- Modify: `crates/myro-predict/src/model/mod.rs` (if adding to predict crate)

**Note:** This module should live in `myro-predict` since it's part of the prediction pipeline, not TUI-specific. The TUI will call into it.

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_history_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.json");

        let mut history = SolveHistory::new();
        history.record(HistoryEntry {
            problem_id: "1800:A".to_string(),
            solved: true,
            timestamp: 1709654400,
        });
        history.record(HistoryEntry {
            problem_id: "1801:B".to_string(),
            solved: false,
            timestamp: 1709740800,
        });

        history.save(&path).unwrap();
        let loaded = SolveHistory::load(&path).unwrap();

        assert_eq!(loaded.entries.len(), 2);
        assert!(loaded.entries[0].solved);
        assert!(!loaded.entries[1].solved);
    }

    #[test]
    fn test_history_hash_changes_on_update() {
        let mut history = SolveHistory::new();
        let hash1 = history.content_hash();

        history.record(HistoryEntry {
            problem_id: "1800:A".to_string(),
            solved: true,
            timestamp: 1709654400,
        });
        let hash2 = history.content_hash();

        assert_ne!(hash1, hash2);
    }
}
```

**Step 2: Implement `SolveHistory`**

Create `crates/myro-predict/src/history.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub problem_id: String,  // "contestId:problemIdx" e.g. "1800:A"
    pub solved: bool,
    pub timestamp: i64,       // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolveHistory {
    pub entries: Vec<HistoryEntry>,
}

impl SolveHistory {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path)?;
        let history: SolveHistory = serde_json::from_str(&json)?;
        Ok(history)
    }

    /// SHA-256 hash of the history contents for cache invalidation.
    pub fn content_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        // Hash the serialized form for stability
        let json = serde_json::to_string(&self.entries).unwrap_or_default();
        json.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
```

Note: Using `DefaultHasher` for simplicity — no need for a cryptographic hash here since this is just cache invalidation, not security. Avoids adding a SHA-256 dependency.

**Step 3: Register the module**

In the crate's `main.rs` or `lib.rs`, add `mod history;` and make it public.

Since `myro-predict` is a binary crate, add the module in `main.rs`:

```rust
mod history;
```

And make it `pub` so myro-tui can use it (or better: consider making the prediction logic a lib crate). For now, myro-tui can depend on myro-predict's types directly, or we duplicate the simple history types. The cleanest approach: add `history.rs` to `myro-predict` and re-export the types.

Actually, since `myro-predict` is a `[[bin]]` crate, other crates can't depend on it as a library. The history module should either go in `myro-cf` (shared lib) or we make `myro-predict` also a lib. The simplest approach for now: **put `history.rs` in `myro-tui`** since that's where it's consumed, and `myro-predict` doesn't need it (query command fetches from CF API).

Revised plan: Create `crates/myro-tui/src/history.rs` instead.

**Step 4: Run tests**

Run: `cargo test -p myro-tui test_history`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/myro-tui/src/history.rs crates/myro-tui/src/main.rs
git commit -m "feat(tui): add myro-tui solve history storage"
```

---

### Task 8: User params cache with hash invalidation

**Files:**
- Create: `crates/myro-tui/src/predict_cache.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_hit_and_miss() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("user_params.bin");

        let params = CachedUserParams {
            theta: vec![0.1, 0.2, 0.3],
            bias: 0.5,
            history_hash: "abc123".to_string(),
        };

        save_cached_params(&params, &path).unwrap();

        // Cache hit: same hash
        let loaded = load_cached_params(&path, "abc123").unwrap();
        assert!(loaded.is_some());

        // Cache miss: different hash
        let loaded = load_cached_params(&path, "different_hash").unwrap();
        assert!(loaded.is_none());
    }
}
```

**Step 2: Implement the cache**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUserParams {
    pub theta: Vec<f64>,
    pub bias: f64,
    pub history_hash: String,
}

pub fn save_cached_params(params: &CachedUserParams, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let encoded = bincode::serialize(params)?;
    std::fs::write(path, encoded)?;
    Ok(())
}

pub fn load_cached_params(path: &Path, expected_hash: &str) -> Result<Option<CachedUserParams>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(path)?;
    let params: CachedUserParams = bincode::deserialize(&data)?;
    if params.history_hash == expected_hash {
        Ok(Some(params))
    } else {
        Ok(None) // Cache miss — hash doesn't match
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p myro-tui test_cache`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/myro-tui/src/predict_cache.rs crates/myro-tui/src/main.rs
git commit -m "feat(tui): add user params cache with hash invalidation"
```

---

### Task 9: Data refresh — collect, train, evaluate

**Files:** No code changes — this is a data pipeline execution task.

**Step 1: Re-collect CF contests**

```bash
cd crates/myro-predict
cargo run --release -- collect --retry-failed
```

This will fetch all available CF/ICPC contests not already in the database. With the 2-second rate limiter, expect ~4 seconds per contest. If there are ~1400 new contests, this takes ~90 minutes. Run it and let it go. If it takes too long, Ctrl+C after ~1 hour — it saves progress incrementally.

**Step 2: Backfill ratings**

```bash
cargo run --release -- backfill-ratings
```

**Step 3: Train model on full dataset**

```bash
cargo run --release -- train \
  --cutoff 2026-03-01 \
  --min-contests 10 \
  --latent-dim 30 \
  --epochs 50 \
  --tag-init true \
  --verbose \
  --output model.bin.gz
```

Using cutoff far in the future to include all data in training. latent-dim bumped to 30 for richer embeddings with the larger dataset.

**Step 4: Export problem-only model**

```bash
cargo run --release -- export-model \
  --model-path model.bin.gz \
  --output problem_model.bin.gz
```

**Step 5: Run holdout eval**

```bash
cargo run --release -- eval \
  --cutoff 2026-03-01 \
  --min-contests 10 \
  --eval-mode holdout \
  --verbose
```

**Step 6: Run cold-start eval**

```bash
cargo run --release -- eval \
  --cutoff 2024-01-01 \
  --min-contests 10 \
  --eval-mode coldstart \
  --verbose
```

This uses pre-2024 data for fitting user params and post-2024 for testing — simulates the real cold-start flow.

**Step 7: Export analysis and generate plots**

```bash
cargo run --release -- export-analysis
python3 analysis/generate_plots.py  # if plots script exists
```

**Step 8: Record results for REPORT.md update (Task 10)**

Note all metrics from Steps 5 and 6 for the report update.

**Step 9: Commit model artifacts (if tracked)**

Only commit the problem_model.bin.gz if it's small enough and intended for distribution. Otherwise .gitignore it and document how to generate.

---

### Task 10: Update REPORT.md

**Files:**
- Modify: `crates/myro-predict/REPORT.md`

**Step 1: Update the report with new data and results**

Key sections to update:

1. **Section 1 (Introduction):** Mention the new cold-start-free architecture.
2. **Section 2.1 (Method):** Add subsection on time-weighted fitting. Describe the `ProblemModel` split.
3. **Section 3 (Dataset):** Update contest count, problem count, observation count, date range.
4. **Section 4 (Results):** Add new evaluation results from Task 9. Include both holdout and cold-start eval modes. Add comparison table.
5. **Section 7 (Limitations):** Remove "Cold-start" as a limitation (it's solved!). Remove "Temporal dynamics" (solved by time decay). Update remaining limitations.
6. **Section 8 (Reproducing):** Add `export-model` and `coldstart` eval commands.

**Step 2: Commit**

```bash
git add crates/myro-predict/REPORT.md
git commit -m "docs(predict): update REPORT.md with expanded dataset and cold-start results"
```

---

### Task 11: Update project documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `myro-design.md` (if it references myro-predict architecture)
- Modify: `myro-adaptive-engine.md` (if it references the prediction model)

**Step 1: Update CLAUDE.md**

In the `myro-predict` section, update:
- Model architecture description: mention `ProblemModel` vs `SolvePredictionModel`
- CLI pipeline: add `export-model` subcommand
- Key technical decisions: add time-weighted fitting, cold-start-free design
- Add note about `history.json` and user params cache in `~/.local/share/myro/`

**Step 2: Update myro-design.md**

Search for any references to `SolvePredictionModel`, user embeddings, or cold-start. Update to reflect the new architecture.

**Step 3: Update myro-adaptive-engine.md**

The bootstrapping section (§7) should reference the new time-weighted fitting approach instead of the planned "import-based" bootstrapping.

**Step 4: Commit**

```bash
git add CLAUDE.md myro-design.md myro-adaptive-engine.md
git commit -m "docs: update project docs for cold-start-free prediction architecture"
```

---

### Task 12: Final integration test

**Files:**
- Modify: `crates/myro-predict/tests/integration_test.rs`

**Step 1: Add end-to-end test for the new flow**

```rust
/// End-to-end: train full model, export ProblemModel, fit new user, predict.
#[test]
fn test_cold_start_end_to_end() {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let k = 5;
    let num_users = 30;
    let num_problems = 20;

    // Generate synthetic data (same as test_synthetic_sgd_training)
    // ... [generate observations] ...

    // Train full model via SGD
    // ... [train] ...

    // Convert to ProblemModel (strip users)
    // ... problem_model = ProblemModel from trained model ...

    // Simulate a "new user" not in training: generate their solve history
    let new_user_theta: Vec<f64> = (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect();
    let new_user_bias: f64 = rng.gen_range(-0.5..0.5);

    let mut new_user_obs = Vec::new();
    for p in 0..num_problems {
        let prob = predict(&new_user_theta, new_user_bias, &prob_alpha[p], prob_diff[p]);
        let solved = rng.gen::<f64>() < prob;
        // Simulate time: recent solves get higher weight
        let days_ago = rng.gen_range(0.0..730.0);
        let weight = time_decay_weight(days_ago, 365.0);
        new_user_obs.push((p, solved, weight));
    }

    // Fit the new user's params from their history
    // ... fit_user_weighted with new_user_obs ...

    // Predict on a held-out problem
    // Verify AUC > 0.6 (better than random)
}
```

This is a sketch — the actual test needs to use the real types from the crate. Since `myro-predict` is a binary crate, the test file can't import its modules directly. The existing tests work around this by reimplementing the math. Follow the same pattern: implement the core logic inline in the test.

Alternatively, consider extracting the model logic into a lib crate (`myro-predict-core` or adding a `[lib]` section to `myro-predict/Cargo.toml`). This would be a cleaner long-term solution but is out of scope for now.

**Step 2: Run all tests**

Run: `cargo test -p myro-predict`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/myro-predict/tests/integration_test.rs
git commit -m "test(predict): add cold-start end-to-end integration test"
```
