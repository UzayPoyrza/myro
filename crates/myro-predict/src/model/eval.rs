use anyhow::Result;
use std::collections::{BTreeMap, HashMap, HashSet};

use rayon::prelude::*;

use super::inference;
use super::types::*;

/// Sigmoid function.
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Compute AUC-ROC from (predicted_score, actual_label) pairs.
pub fn compute_auc(predictions: &[(f64, bool)]) -> f64 {
    let mut sorted: Vec<_> = predictions.to_vec();
    sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let total_pos = sorted.iter().filter(|(_, y)| *y).count() as f64;
    let total_neg = sorted.len() as f64 - total_pos;

    if total_pos == 0.0 || total_neg == 0.0 {
        return 0.5;
    }

    let mut tp = 0.0;
    let mut auc = 0.0;
    for &(_, label) in &sorted {
        if label {
            tp += 1.0;
        } else {
            auc += tp;
        }
    }
    auc / (total_pos * total_neg)
}

/// Compute log-loss (binary cross-entropy).
pub fn compute_logloss(predictions: &[(f64, bool)]) -> f64 {
    let n = predictions.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let sum: f64 = predictions
        .iter()
        .map(|&(p, y)| {
            let p_clamped = p.clamp(1e-7, 1.0 - 1e-7);
            let label = if y { 1.0 } else { 0.0 };
            -(label * p_clamped.ln() + (1.0 - label) * (1.0 - p_clamped).ln())
        })
        .sum();
    sum / n
}

/// Per-rating-band accuracy metrics.
pub fn per_band_metrics(predictions: &[(f64, bool, Option<i32>)]) -> Vec<(String, f64, f64, usize)>
{
    let bands = [
        ("800-1200", 800, 1200),
        ("1200-1600", 1200, 1600),
        ("1600-2000", 1600, 2000),
        ("2000-2400", 2000, 2400),
        ("2400-3500", 2400, 3500),
    ];

    bands
        .iter()
        .filter_map(|(name, lo, hi)| {
            let band_preds: Vec<(f64, bool)> = predictions
                .iter()
                .filter(|(_, _, rating)| {
                    if let Some(r) = rating {
                        *r >= *lo && *r < *hi
                    } else {
                        false
                    }
                })
                .map(|(p, y, _)| (*p, *y))
                .collect();

            if band_preds.is_empty() {
                return None;
            }

            let auc = compute_auc(&band_preds);
            let logloss = compute_logloss(&band_preds);
            Some((name.to_string(), auc, logloss, band_preds.len()))
        })
        .collect()
}

/// Per-history-depth accuracy metrics.
pub fn per_depth_metrics(
    predictions: &[(f64, bool, usize)],
) -> Vec<(String, f64, f64, usize)> {
    let bins: &[(&str, usize, usize)] = &[
        ("5-10", 5, 10),
        ("10-20", 10, 20),
        ("20-50", 20, 50),
        ("50+", 50, usize::MAX),
    ];

    bins.iter()
        .filter_map(|(name, lo, hi)| {
            let bin_preds: Vec<(f64, bool)> = predictions
                .iter()
                .filter(|(_, _, depth)| *depth >= *lo && *depth < *hi)
                .map(|(p, y, _)| (*p, *y))
                .collect();

            if bin_preds.is_empty() {
                return None;
            }

            let auc = compute_auc(&bin_preds);
            let logloss = compute_logloss(&bin_preds);
            Some((name.to_string(), auc, logloss, bin_preds.len()))
        })
        .collect()
}

// ──────────────────────── Baselines ────────────────────────

/// Compute per-problem solve rate from dataset.
fn compute_solve_rates(dataset: &TrainingDataset) -> HashMap<usize, f64> {
    let mut counts: HashMap<usize, (usize, usize)> = HashMap::new();
    for obs in &dataset.observations {
        let entry = counts.entry(obs.problem_idx).or_default();
        if obs.solved {
            entry.0 += 1;
        }
        entry.1 += 1;
    }
    counts
        .into_iter()
        .map(|(idx, (solved, total))| (idx, solved as f64 / total as f64))
        .collect()
}

/// Train logistic regression on (rating_diff, tag indicators) and return (weights, bias).
fn train_logreg(dataset: &TrainingDataset) -> (Vec<f64>, f64) {
    let mut all_tags: Vec<String>;
    {
        let mut tag_set = HashSet::new();
        for tags in &dataset.problem_tags {
            for tag in tags {
                tag_set.insert(tag.clone());
            }
        }
        all_tags = tag_set.into_iter().collect();
        all_tags.sort();
    }
    let tag_to_idx: HashMap<&str, usize> = all_tags
        .iter()
        .enumerate()
        .map(|(i, t)| (t.as_str(), i))
        .collect();

    let num_features = 1 + all_tags.len();
    let mut weights = vec![0.0f64; num_features];
    let mut w_bias = 0.0f64;

    let build_features = |obs: &Observation| -> Vec<f64> {
        let mut feats = vec![0.0; num_features];
        if let (Some(ur), Some(pr)) = (obs.user_rating, obs.problem_rating) {
            feats[0] = (ur as f64 - pr as f64) / 400.0;
        }
        if obs.problem_idx < dataset.problem_tags.len() {
            for tag in &dataset.problem_tags[obs.problem_idx] {
                if let Some(&idx) = tag_to_idx.get(tag.as_str()) {
                    feats[1 + idx] = 1.0;
                }
            }
        }
        feats
    };

    let lr = 0.01;
    let l2 = 0.001;
    let epochs = 20;
    let mut rng = rand::thread_rng();

    for _ in 0..epochs {
        use rand::seq::SliceRandom;
        let mut indices: Vec<usize> = (0..dataset.observations.len()).collect();
        indices.shuffle(&mut rng);

        for &i in &indices {
            let obs = &dataset.observations[i];
            let feats = build_features(obs);
            let y = if obs.solved { 1.0 } else { 0.0 };

            let logit: f64 = feats.iter().zip(&weights).map(|(f, w)| f * w).sum::<f64>() + w_bias;
            let pred = sigmoid(logit);
            let error = pred - y;

            for (j, f) in feats.iter().enumerate() {
                weights[j] -= lr * (error * f + l2 * weights[j]);
            }
            w_bias -= lr * error;
        }
    }

    (weights, w_bias)
}

/// Predict with pre-trained logistic regression weights.
fn logreg_predict(
    weights: &[f64],
    bias: f64,
    user_rating: Option<i32>,
    problem_rating: Option<i32>,
    tags: &[String],
    tag_to_idx: &HashMap<&str, usize>,
) -> f64 {
    let num_features = weights.len();
    let mut feats = vec![0.0; num_features];
    if let (Some(ur), Some(pr)) = (user_rating, problem_rating) {
        feats[0] = (ur as f64 - pr as f64) / 400.0;
    }
    for tag in tags {
        if let Some(&idx) = tag_to_idx.get(tag.as_str()) {
            if 1 + idx < num_features {
                feats[1 + idx] = 1.0;
            }
        }
    }
    let logit: f64 = feats.iter().zip(weights).map(|(f, w)| f * w).sum::<f64>() + bias;
    sigmoid(logit)
}

// ──────────────────────── Temporal eval ────────────────────────

/// Temporal evaluation: process each user's contests chronologically,
/// fit their embedding from only prior history, predict current contest outcomes.
///
/// This mirrors deployment: problem model is trained on all data (problem properties
/// are time-invariant), user embeddings are computed on-the-fly from prior history.
pub fn run_temporal_eval(
    problem_model: &ProblemModel,
    dataset: &TrainingDataset,
    min_history_contests: usize,
    verbose: bool,
) -> Result<()> {
    // Build mapping from dataset problem indices to ProblemModel problem indices
    let dataset_to_model_problem: HashMap<usize, usize> = dataset
        .problem_index
        .iter()
        .filter_map(|(key, &ds_idx)| {
            problem_model
                .problem_index
                .get(key)
                .map(|&model_idx| (ds_idx, model_idx))
        })
        .collect();

    // Compute baselines upfront (shared, read-only during parallel loop)
    let solve_rates = compute_solve_rates(dataset);
    let global_solve_rate = {
        let total_solved = dataset.observations.iter().filter(|o| o.solved).count();
        total_solved as f64 / dataset.observations.len().max(1) as f64
    };

    // Train logistic regression on full dataset
    let (logreg_weights, logreg_bias) = train_logreg(dataset);

    // Build tag index for logreg prediction
    let mut all_tags: Vec<String>;
    {
        let mut tag_set = HashSet::new();
        for tags in &dataset.problem_tags {
            for tag in tags {
                tag_set.insert(tag.clone());
            }
        }
        all_tags = tag_set.into_iter().collect();
        all_tags.sort();
    }
    let tag_to_idx: HashMap<&str, usize> = all_tags
        .iter()
        .enumerate()
        .map(|(i, t)| (t.as_str(), i))
        .collect();

    // Group observations by user, then by contest timestamp
    let mut user_contests: HashMap<usize, BTreeMap<i64, Vec<&Observation>>> = HashMap::new();
    for obs in &dataset.observations {
        user_contests
            .entry(obs.user_idx)
            .or_default()
            .entry(obs.contest_timestamp)
            .or_default()
            .push(obs);
    }

    // Filter to users with enough contests
    let eligible_users: Vec<(usize, BTreeMap<i64, Vec<&Observation>>)> = user_contests
        .into_iter()
        .filter(|(_, contests)| contests.len() >= min_history_contests + 1)
        .collect();

    let num_eligible = eligible_users.len();
    if verbose {
        println!(
            "Eligible users (>= {} + 1 contests): {}",
            min_history_contests, num_eligible
        );
        println!(
            "Problem coverage: {}/{} dataset problems mapped to model",
            dataset_to_model_problem.len(),
            dataset.problem_index.len()
        );
    }

    // Shared references for parallel loop
    let solve_rates_ref = &solve_rates;
    let global_solve_rate_ref = &global_solve_rate;
    let logreg_weights_ref = &logreg_weights;
    let logreg_bias_ref = &logreg_bias;
    let tag_to_idx_ref = &tag_to_idx;
    let dataset_to_model_ref = &dataset_to_model_problem;
    let dataset_ref = dataset;

    // Process each user in parallel
    // Each result: Vec<(mf_pred, elo_pred, solverate_pred, logreg_pred, actual, problem_rating, history_depth)>
    let all_results: Vec<Vec<(f64, f64, f64, f64, bool, Option<i32>, usize)>> = eligible_users
        .par_iter()
        .map(|(_user_idx, contests)| {
            let mut results = Vec::new();
            // Accumulated history: (model_problem_idx, solved, timestamp)
            let mut history: Vec<(usize, bool, i64)> = Vec::new();
            let mut contest_count = 0usize;

            for (&timestamp, obs_list) in contests.iter() {
                if contest_count >= min_history_contests {
                    // Build weighted observations from accumulated history
                    let weighted_obs: Vec<WeightedObservation> = history
                        .iter()
                        .map(|&(model_p_idx, solved, obs_ts)| {
                            let days_ago = (timestamp - obs_ts) as f64 / 86400.0;
                            let weight = inference::time_decay_weight(
                                days_ago.max(0.0),
                                inference::DEFAULT_HALF_LIFE_DAYS,
                            );
                            WeightedObservation {
                                problem_idx: model_p_idx,
                                solved,
                                weight,
                            }
                        })
                        .collect();

                    // Fit user from history
                    let user_params = inference::fit_user_weighted(
                        problem_model,
                        &weighted_obs,
                        0.01,
                        50,
                        0.01,
                    );

                    // Predict for each problem in this contest
                    for obs in obs_list {
                        if let Some(&model_p_idx) = dataset_to_model_ref.get(&obs.problem_idx) {
                            let mf_pred = inference::predict(
                                &user_params,
                                &problem_model.problem_params[model_p_idx],
                            );

                            let elo_pred = match (obs.user_rating, obs.problem_rating) {
                                (Some(ur), Some(pr)) => {
                                    sigmoid((ur as f64 - pr as f64) / 400.0)
                                }
                                _ => 0.5,
                            };

                            let solverate_pred = solve_rates_ref
                                .get(&obs.problem_idx)
                                .copied()
                                .unwrap_or(*global_solve_rate_ref);

                            let tags = if obs.problem_idx < dataset_ref.problem_tags.len() {
                                &dataset_ref.problem_tags[obs.problem_idx]
                            } else {
                                &vec![] as &Vec<String>
                            };
                            let logreg_pred = logreg_predict(
                                logreg_weights_ref,
                                *logreg_bias_ref,
                                obs.user_rating,
                                obs.problem_rating,
                                tags,
                                tag_to_idx_ref,
                            );

                            results.push((
                                mf_pred,
                                elo_pred,
                                solverate_pred,
                                logreg_pred,
                                obs.solved,
                                obs.problem_rating,
                                contest_count,
                            ));
                        }
                    }
                }

                // Accumulate this contest's observations into history
                for obs in obs_list {
                    if let Some(&model_p_idx) = dataset_to_model_ref.get(&obs.problem_idx) {
                        history.push((model_p_idx, obs.solved, timestamp));
                    }
                }
                contest_count += 1;
            }

            results
        })
        .collect();

    // Flatten results
    let flat: Vec<(f64, f64, f64, f64, bool, Option<i32>, usize)> =
        all_results.into_iter().flatten().collect();

    if flat.is_empty() {
        println!("No test observations after temporal split. Check your data and min_history.");
        return Ok(());
    }

    // Build prediction vectors for each method
    let random_preds: Vec<(f64, bool)> = flat.iter().map(|r| (0.5, r.4)).collect();
    let solverate_preds: Vec<(f64, bool)> = flat.iter().map(|r| (r.2, r.4)).collect();
    let elo_preds: Vec<(f64, bool)> = flat.iter().map(|r| (r.1, r.4)).collect();
    let logreg_preds: Vec<(f64, bool)> = flat.iter().map(|r| (r.3, r.4)).collect();
    let mf_preds: Vec<(f64, bool)> = flat.iter().map(|r| (r.0, r.4)).collect();

    // Print comparison table
    println!();
    println!(
        "{:<30} {:>8} {:>10} {:>8}",
        "Method", "AUC", "Log-loss", "N"
    );
    println!("{}", "-".repeat(60));

    let print_row = |name: &str, preds: &[(f64, bool)]| {
        let auc = compute_auc(preds);
        let ll = compute_logloss(preds);
        println!("{:<30} {:>8.4} {:>10.4} {:>8}", name, auc, ll, preds.len());
    };

    print_row("Random", &random_preds);
    print_row("Problem solve rate", &solverate_preds);
    print_row("Elo (CF ratings, s=400)", &elo_preds);
    print_row("LogReg (rating + tags)", &logreg_preds);
    print_row("MF temporal", &mf_preds);

    // Boundary zone: filter to observations where Elo predicts 0.3-0.7
    // This strips away trivially predictable observations and highlights
    // where personalization actually matters.
    let boundary_indices: Vec<usize> = flat
        .iter()
        .enumerate()
        .filter(|(_, r)| r.1 >= 0.3 && r.1 <= 0.7)
        .map(|(i, _)| i)
        .collect();

    if !boundary_indices.is_empty() {
        let filter = |preds: &[(f64, bool)]| -> Vec<(f64, bool)> {
            boundary_indices.iter().map(|&i| preds[i]).collect()
        };

        println!();
        println!("Boundary zone (Elo 0.3-0.7 — where personalization matters):");
        println!(
            "{:<30} {:>8} {:>10} {:>8}",
            "Method", "AUC", "Log-loss", "N"
        );
        println!("{}", "-".repeat(60));

        print_row("Random", &filter(&random_preds));
        print_row("Problem solve rate", &filter(&solverate_preds));
        print_row("Elo (CF ratings, s=400)", &filter(&elo_preds));
        print_row("LogReg (rating + tags)", &filter(&logreg_preds));
        print_row("MF temporal", &filter(&mf_preds));
    }

    // Per-band breakdown for MF
    if verbose {
        let mf_with_rating: Vec<(f64, bool, Option<i32>)> =
            flat.iter().map(|r| (r.0, r.4, r.5)).collect();

        println!();
        println!("Per-rating-band breakdown (MF temporal):");
        println!(
            "{:<15} {:>8} {:>10} {:>8}",
            "Band", "AUC", "Log-loss", "N"
        );
        println!("{}", "-".repeat(45));
        for (band, auc, ll, n) in per_band_metrics(&mf_with_rating) {
            println!("{:<15} {:>8.4} {:>10.4} {:>8}", band, auc, ll, n);
        }

        // Per-depth breakdown for MF
        let mf_with_depth: Vec<(f64, bool, usize)> =
            flat.iter().map(|r| (r.0, r.4, r.6)).collect();

        println!();
        println!("Per-history-depth breakdown (MF temporal):");
        println!(
            "{:<15} {:>8} {:>10} {:>8}",
            "Depth", "AUC", "Log-loss", "N"
        );
        println!("{}", "-".repeat(45));
        for (band, auc, ll, n) in per_depth_metrics(&mf_with_depth) {
            println!("{:<15} {:>8.4} {:>10.4} {:>8}", band, auc, ll, n);
        }

        println!();
        println!(
            "Total: {} users evaluated, {} test observations",
            num_eligible,
            flat.len()
        );
    }

    Ok(())
}
