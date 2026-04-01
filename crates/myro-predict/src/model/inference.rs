use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;

use super::types::*;
use crate::db::model_store;
use myro_cf::CfClient;

/// Sigmoid function.
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Predict P(solve) for a user and problem.
pub fn predict(user: &UserParams, problem: &ProblemParams) -> f64 {
    let logit = dot(&user.theta, &problem.alpha) + user.bias + problem.difficulty;
    sigmoid(logit)
}

/// Predict P(solve) for a batch of problems.
pub fn predict_batch(user: &UserParams, problems: &[&ProblemParams]) -> Vec<f64> {
    problems.iter().map(|p| predict(user, p)).collect()
}

/// Predict P(solve) for all problems in a ProblemModel at once via GEMV.
///
/// logits = A · θ_u + b_u + d   where A is (n_problems × k), d is difficulty biases
/// P(solve) = σ(logits)
pub fn predict_all(user: &UserParams, model: &ProblemModel) -> Vec<f64> {
    let k = model.latent_dim;
    let n = model.problem_params.len();
    let mut result = Vec::with_capacity(n);
    for p in &model.problem_params {
        let logit = dot(&user.theta[..k], &p.alpha[..k]) + user.bias + p.difficulty;
        result.push(sigmoid(logit));
    }
    result
}

/// Default half-life for time decay weighting (in days).
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 365.0;

/// Compute time decay weight: 2^(-days_ago / half_life).
pub fn time_decay_weight(days_ago: f64, half_life_days: f64) -> f64 {
    2.0_f64.powf(-days_ago / half_life_days)
}

/// Fit user parameters with per-observation weights.
///
/// Holds problem parameters fixed and optimizes user params (θ_u, b_u)
/// via gradient descent. Gradient is scaled by each observation's weight.
pub fn fit_user_weighted(
    model: &ProblemModel,
    obs: &[WeightedObservation],
    lr: f64,
    epochs: usize,
    lambda: f64,
) -> UserParams {
    let k = model.latent_dim;
    let mut user = UserParams {
        theta: vec![0.0; k],
        bias: 0.0,
    };

    for _ in 0..epochs {
        for o in obs {
            let problem = &model.problem_params[o.problem_idx];
            let pred = predict(&user, problem);
            let y = if o.solved { 1.0 } else { 0.0 };
            let error = pred - y;

            for dim in 0..k {
                let grad = o.weight * error * problem.alpha[dim] + lambda * user.theta[dim];
                user.theta[dim] -= lr * grad;
            }
            let grad_b = o.weight * error + lambda * user.bias;
            user.bias -= lr * grad_b;
        }
    }

    user
}

/// Fit user parameters for a new user given their solve history (unweighted).
///
/// Delegates to `fit_user_weighted` with weight=1.0 for all observations.
pub fn fit_user(
    model: &SolvePredictionModel,
    solves: &[(usize, bool)],
    lr: f64,
    epochs: usize,
    lambda: f64,
) -> UserParams {
    let problem_model = ProblemModel {
        latent_dim: model.config.latent_dim,
        problem_params: model.problem_params.clone(),
        problem_index: model.problem_index.clone(),
        problem_ratings: model.problem_ratings.clone(),
        problem_tags: model.problem_tags.clone(),
        tag_dim_map: model.tag_dim_map.clone(),
    };
    let obs: Vec<WeightedObservation> = solves
        .iter()
        .map(|&(problem_idx, solved)| WeightedObservation {
            problem_idx,
            solved,
            weight: 1.0,
        })
        .collect();
    fit_user_weighted(&problem_model, &obs, lr, epochs, lambda)
}

/// Build weighted observations from CF submissions against a ProblemModel.
///
/// For each model problem, checks if the user submitted to it:
/// - solved=true if any submission verdict is "OK"
/// - solved=false if user submitted but never got "OK"
/// - No observation for problems never submitted to
///
/// Returns (observations, solved_keys) for downstream use.
pub fn build_observations_from_submissions(
    model: &ProblemModel,
    submissions: &[myro_cf::CfSubmission],
    now_ts: i64,
    half_life_days: f64,
) -> (Vec<WeightedObservation>, HashMap<String, bool>) {
    // Build per-problem best result: (solved, latest_timestamp)
    let mut problem_results: HashMap<String, (bool, i64)> = HashMap::new();
    for sub in submissions {
        if let Some(cid) = sub.contest_id {
            let key = format!("{}:{}", cid, sub.problem.index);
            let solved = sub.verdict.as_deref() == Some("OK");
            let entry = problem_results.entry(key).or_insert((false, sub.creation_time_seconds));
            if solved {
                entry.0 = true;
            }
            if sub.creation_time_seconds > entry.1 {
                entry.1 = sub.creation_time_seconds;
            }
        }
    }

    let mut obs = Vec::new();
    let mut solved_keys = HashMap::new();
    for (key, &idx) in &model.problem_index {
        if let Some(&(solved, ts)) = problem_results.get(key) {
            let days_ago = (now_ts - ts) as f64 / 86400.0;
            let weight = time_decay_weight(days_ago.max(0.0), half_life_days);
            obs.push(WeightedObservation {
                problem_idx: idx,
                solved,
                weight,
            });
            solved_keys.insert(key.clone(), solved);
        }
    }

    (obs, solved_keys)
}

/// Run the query subcommand: fetch user history, fit params, print predictions.
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

    let now_ts = chrono::Utc::now().timestamp();
    let (obs, solved_keys) =
        build_observations_from_submissions(&model, &submissions, now_ts, DEFAULT_HALF_LIFE_DAYS);

    if obs.is_empty() {
        bail!("No overlap between user's submission history and model problems");
    }

    let submitted_count = obs.len();
    let solved_count = obs.iter().filter(|o| o.solved).count();
    println!(
        "Found {} submitted problems in model ({} solved by {})",
        submitted_count, solved_count, handle
    );

    println!("Fitting user parameters from submission history...");
    let user_params = fit_user_weighted(&model, &obs, 0.01, 100, 0.01);

    // If specific problems requested, predict those
    if let Some(ref prob_str) = problems {
        let prob_ids: Vec<&str> = prob_str.split(',').map(|s| s.trim()).collect();
        println!();
        println!(
            "{:<15} {:>10} {:>8} {:>10}",
            "Problem", "P(solve)", "Rating", "Tags"
        );
        println!("{}", "-".repeat(50));

        for pid in prob_ids {
            let found = model
                .problem_index
                .iter()
                .find(|(key, _)| {
                    key.ends_with(&format!(":{}", pid.chars().last().unwrap_or(' ')))
                        && key.starts_with(&pid[..pid.len().saturating_sub(1)])
                        || *key == pid
                });

            if let Some((key, &idx)) = found {
                let pred = predict(&user_params, &model.problem_params[idx]);
                let rating = model
                    .problem_ratings
                    .get(idx)
                    .and_then(|r| *r)
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let tags = model
                    .problem_tags
                    .get(idx)
                    .map(|t| t.join(", "))
                    .unwrap_or_default();
                println!("{:<15} {:>10.3} {:>8} {:>10}", key, pred, rating, tags);
            } else {
                println!("{:<15} {:>10}", pid, "not found");
            }
        }
    }

    // If top-N requested, show hardest/easiest problems
    if let Some(n) = top_n {
        let mut all_preds: Vec<(String, f64, Option<i32>, Vec<String>)> = model
            .problem_index
            .iter()
            .map(|(key, &idx)| {
                let pred = predict(&user_params, &model.problem_params[idx]);
                let rating = model.problem_ratings.get(idx).and_then(|r| *r);
                let tags = model
                    .problem_tags
                    .get(idx)
                    .cloned()
                    .unwrap_or_default();
                (key.clone(), pred, rating, tags)
            })
            .filter(|(key, _, _, _)| !solved_keys.contains_key(key))
            .collect();

        all_preds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        println!();
        println!("Top {} unsolved problems by predicted difficulty:", n);
        println!();
        println!(
            "{:<15} {:>10} {:>8} {}",
            "Problem", "P(solve)", "Rating", "Tags"
        );
        println!("{}", "-".repeat(70));

        println!("--- Most likely to solve ---");
        for (key, pred, rating, tags) in all_preds.iter().rev().take(n) {
            let r = rating.map(|r| r.to_string()).unwrap_or_else(|| "?".to_string());
            println!(
                "{:<15} {:>10.3} {:>8} {}",
                key,
                pred,
                r,
                tags.join(", ")
            );
        }

        println!();
        println!("--- Least likely to solve ---");
        for (key, pred, rating, tags) in all_preds.iter().take(n) {
            let r = rating.map(|r| r.to_string()).unwrap_or_else(|| "?".to_string());
            println!(
                "{:<15} {:>10.3} {:>8} {}",
                key,
                pred,
                r,
                tags.join(", ")
            );
        }
    }

    // Default: summary stats
    if problems.is_none() && top_n.is_none() {
        let all_preds = predict_all(&user_params, &model);

        let avg: f64 = all_preds.iter().sum::<f64>() / all_preds.len() as f64;
        let min = all_preds.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = all_preds.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        println!();
        println!("Prediction summary for {} ({} problems):", handle, all_preds.len());
        println!("  Mean P(solve): {:.3}", avg);
        println!("  Min P(solve):  {:.3}", min);
        println!("  Max P(solve):  {:.3}", max);
        println!();
        println!("Use --top-n or --problems for detailed predictions.");
    }

    Ok(())
}
