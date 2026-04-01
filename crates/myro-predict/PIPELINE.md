# myro-predict: Training & Evaluation Pipeline

Run this pipeline whenever you pull new data or retrain the model. All commands run from `crates/myro-predict/`.

## 1. Collect data (skip if data is already up to date)

```bash
cargo run --release -- collect --retry-failed
cargo run --release -- backfill-ratings
```

## 2. Train model

```bash
cargo run --release -- train \
  --cutoff 2030-01-01 \
  --exclude-users kalimm \
  --epochs 100 \
  --verbose
```

- `--cutoff 2030-01-01` includes all data (far-future date)
- `--exclude-users kalimm` keeps kalimm out for divergence analysis
- Outputs: `model.bin.gz`, `analysis_training_curve.csv`

## 3. Export problem model

```bash
cargo run --release -- export-model
```

Outputs: `problem_model.bin.gz`

## 4. Run temporal eval

```bash
cargo run --release -- eval \
  --model-path problem_model.bin.gz \
  --min-history 5 \
  --verbose
```

Save the output — you'll need the numbers for REPORT.md.

## 5. Export analysis CSVs

```bash
cargo run --release -- export-analysis
```

## 6. Run kalimm divergence query

```bash
cargo run --release -- query \
  --handle kalimm \
  --model-path problem_model.bin.gz
```

Save the output for DIVERGENCE.md.

## 7. Update docs

All of these must be updated with the new numbers **and new interpretation where warranted**. Don't just swap numbers — if results changed meaningfully (e.g. AUC shifted, a baseline overtook another, per-band patterns differ), rewrite the surrounding prose to reflect the new story.

| File | What to update |
|------|----------------|
| `REPORT.md` | Section 1 (intro counts), Section 3 (dataset table), Section 4.1 (overall results + boundary zone), Section 4.3 (per-band), Section 4.4 (per-depth), Section 4.5 (training dynamics), Section 9 (conclusion AUC numbers). Revise interpretation/commentary if trends changed. |
| `analysis/DIVERGENCE.md` | Header stats (submitted/solved/total counts), model description. Revise tag/rating analysis if divergence patterns shifted. |
| `docs/myro-predict.md` | Only if CLI flags change |
| `CLAUDE.md` | Only if public API or architecture changes |

## Quick checklist

- [ ] Train completed (check final epoch loss/AUC)
- [ ] Problem model exported
- [ ] Temporal eval ran successfully, MF AUC > Elo AUC
- [ ] REPORT.md updated with new numbers
- [ ] DIVERGENCE.md updated with new model info
- [ ] `cargo check` passes
- [ ] `cargo test -p myro-predict` passes
