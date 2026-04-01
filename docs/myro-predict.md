# myro-predict CLI

Command-line tool for the Codeforces solve probability prediction pipeline.

For algorithmic details, see [algorithm.md](algorithm.md).

## Quick Start

```bash
# Full pipeline
just predict collect                              # Fetch CF contest data
just predict backfill-ratings                     # Fill missing user ratings
just predict train --cutoff 2026-03-01            # Train model
just predict export-model                         # Export problem-only model
just predict eval -- --verbose                    # Temporal walk-forward eval
just predict query -- --handle tourist --top-n 10 # Predict for a user
```

All commands accept `--help` for full flag documentation.

## Commands

### `collect` — Fetch Codeforces contest data

Downloads contest standings and rating changes into a local SQLite database. Incremental — re-running skips already-fetched contests.

```bash
just predict collect                          # Fetch all contests
just predict collect -- --max-contests 10     # Smoke test (10 contests)
just predict collect -- --since-contest 1800  # Only contests with ID >= 1800
just predict collect -- --retry-failed        # Re-attempt failed fetches
just predict collect -- --db-path ./my.db     # Custom database path
```

| Flag | Default | Description |
|------|---------|-------------|
| `--db-path` | `predict.db` | SQLite database path |
| `--max-contests` | all | Stop after N contests |
| `--since-contest` | none | Only fetch contests with ID >= value |
| `--retry-failed` | false | Re-attempt previously failed fetches |

Rate limited at 1 request per 2 seconds with exponential backoff. Ctrl+C safe — progress is saved incrementally.

### `backfill-ratings` — Fill missing user ratings

Fetches `contest.ratingChanges` for contests missing user rating data.

```bash
just predict backfill-ratings
just predict backfill-ratings -- --db-path ./my.db
```

### `train` — Train the model

Trains a logistic matrix factorization model on contest data before a temporal cutoff date.

```bash
just predict train -- --cutoff 2026-03-01
just predict train -- --cutoff 2026-03-01 --latent-dim 10 --epochs 100 --verbose
just predict train -- --cutoff 2026-03-01 --exclude-users kalimm,tourist
just predict train -- --cutoff 2026-03-01 --output my-model.bin.gz
```

| Flag | Default | Description |
|------|---------|-------------|
| `--cutoff` | required | Temporal split date (YYYY-MM-DD) |
| `--latent-dim` | 30 | Number of latent dimensions |
| `--epochs` | 50 | SGD training epochs |
| `--lr` | 0.01 | Learning rate |
| `--lambda` | 0.01 | L2 regularization strength |
| `--tag-init` | true | Seed dims from CF problem tags |
| `--neg-ratio` | 0.0 | Negative downsampling ratio |
| `--min-contests` | 10 | Minimum contests per user |
| `--output` | `model.bin.gz` | Output model path |
| `--exclude-users` | none | Comma-separated handles to exclude |
| `--verbose` | false | Print per-epoch metrics |

Outputs: `model.bin.gz` (trained model) and `analysis_training_curve.csv` (loss + AUC per epoch).

### `export-model` — Export problem-only model

Strips user parameters from a trained model, producing a `ProblemModel` for cold-start deployment. The problem model is all that's needed for prediction — user embeddings are fit on-the-fly from solve history.

```bash
just predict export-model
just predict export-model -- --model-path my-model.bin.gz --output my-problem-model.bin.gz
```

| Flag | Default | Description |
|------|---------|-------------|
| `--model-path` | `model.bin.gz` | Full trained model path |
| `--output` | `problem_model.bin.gz` | Output problem model path |

### `eval` — Temporal walk-forward evaluation

Evaluates the model using a temporal walk-forward protocol. For each user, processes their contests chronologically: fits their embedding from only prior history, then predicts current contest outcomes. Requires a pre-trained `ProblemModel` (from `export-model`).

```bash
just predict eval -- --verbose                                        # Default (all data)
just predict eval -- --cutoff 2026-03-01 --verbose                    # Only pre-cutoff data
just predict eval -- --model-path my-problem-model.bin.gz --verbose   # Custom model
just predict eval -- --min-history 10 --min-contests 20               # Stricter filters
```

| Flag | Default | Description |
|------|---------|-------------|
| `--model-path` | `problem_model.bin.gz` | Pre-trained problem model path |
| `--min-history` | 5 | Minimum prior contests before evaluating a user |
| `--min-contests` | 10 | Minimum total contests per user |
| `--cutoff` | none | Optional date filter (YYYY-MM-DD) — only uses data before this date |
| `--verbose` | false | Per-rating-band breakdown |

**How it works:** For each user with enough history, the eval walks forward through their contests in time order. At each step, it fits a user embedding from all prior observations using `fit_user_weighted`, then predicts solve probabilities for the current contest's problems. This directly measures real-world cold-start performance since user embeddings are never trained — only problem embeddings come from the pre-trained model.

**Baselines compared:** random, per-problem solve rate, Elo (CF ratings), logistic regression (rating + tags).

### `export-analysis` — Export model parameters to CSV

Exports problem parameters, user parameters, and tag-dimension mappings as CSV files for external analysis (visualization, debugging).

```bash
just predict export-analysis
just predict export-analysis -- --output-dir ./analysis/
```

Produces:
- `analysis_problem_params.csv` — problem key, rating, tags, difficulty bias, alpha vectors
- `analysis_user_params.csv` — handle, bias, theta vectors
- `analysis_tag_dim_map.csv` — tag name to latent dimension mapping

### `query` — Predict solve probabilities for a user

Fetches a user's Codeforces submission history, fits user embeddings on-the-fly against the problem model, and predicts solve probability for all problems. Works for any CF user — no retraining needed.

```bash
just predict query -- --handle tourist --top-n 10
just predict query -- --handle tourist --problems "1800A,1801B"
just predict query -- --handle tourist
```

| Flag | Default | Description |
|------|---------|-------------|
| `--handle` | required | Codeforces handle |
| `--model-path` | `problem_model.bin.gz` | Problem model path |
| `--problems` | none | Comma-separated problem IDs |
| `--top-n` | none | Show top-N by predicted difficulty |

## Typical Pipeline

```bash
# 1. Collect data (incremental, ~2-5 hours for full fetch)
just predict collect -- --retry-failed

# 2. Backfill any missing ratings
just predict backfill-ratings

# 3. Train model
just predict train -- --cutoff 2026-03-01 --latent-dim 30 --epochs 50 --tag-init true --verbose

# 4. Export problem-only model for deployment
just predict export-model

# 5. Evaluate (temporal walk-forward)
just predict eval -- --verbose

# 6. Export analysis CSVs (optional)
just predict export-analysis

# 7. Query predictions
just predict query -- --handle kalimm --top-n 20
```

## Database

SQLite with WAL mode. Default path: `predict.db`.

| Table | Purpose |
|-------|---------|
| `cf_contests` | Contest metadata + fetch status |
| `cf_contest_problems` | Problem names, ratings, tags |
| `cf_contest_results` | Per-user solve/fail per problem, user rating |
| `prediction_models` | Stored trained models with config |

## Files

| Path | Description |
|------|-------------|
| `predict.db` | SQLite database (generated) |
| `model.bin.gz` | Full trained model (bincode + gzip) |
| `problem_model.bin.gz` | Problem-only model for deployment |
| `analysis_*.csv` | Exported analysis files |
| `~/.local/share/myro/history.json` | User solve history (for TUI integration) |
| `~/.local/share/myro/user_params.bin` | Cached user parameters |
