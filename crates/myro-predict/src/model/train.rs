use anyhow::Result;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;

use super::types::*;

/// Sigmoid function.
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Dot product of two vectors.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Predict P(solve) for a given user and problem.
fn predict(user: &UserParams, problem: &ProblemParams) -> f64 {
    let logit = dot(&user.theta, &problem.alpha) + user.bias + problem.difficulty;
    sigmoid(logit)
}

/// Build a tag→dimension mapping from problem tags in the dataset.
/// The first `max_tag_dims` unique tags each get their own latent dimension.
fn build_tag_dim_map(dataset: &TrainingDataset, max_tag_dims: usize) -> HashMap<String, usize> {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for tags in &dataset.problem_tags {
        for tag in tags {
            *tag_counts.entry(tag.clone()).or_default() += 1;
        }
    }

    // Sort by frequency (most common tags get dedicated dimensions)
    let mut sorted_tags: Vec<_> = tag_counts.into_iter().collect();
    sorted_tags.sort_by(|a, b| b.1.cmp(&a.1));

    sorted_tags
        .into_iter()
        .take(max_tag_dims)
        .enumerate()
        .map(|(dim, (tag, _))| (tag, dim))
        .collect()
}

/// Initialize problem parameters using tag information.
/// Dimensions corresponding to a problem's tags get a small positive init,
/// other dimensions get near-zero random values.
fn initialize_from_tags(
    num_problems: usize,
    latent_dim: usize,
    problem_tags: &[Vec<String>],
    tag_dim_map: &HashMap<String, usize>,
    rng: &mut impl Rng,
) -> Vec<ProblemParams> {
    let scale = 0.1;
    let tag_scale = 0.3;

    (0..num_problems)
        .map(|p| {
            let mut alpha = vec![0.0; latent_dim];
            // Random init for all dims
            for v in &mut alpha {
                *v = rng.gen_range(-scale..scale);
            }
            // Boost dims corresponding to this problem's tags
            if p < problem_tags.len() {
                for tag in &problem_tags[p] {
                    if let Some(&dim) = tag_dim_map.get(tag) {
                        if dim < latent_dim {
                            alpha[dim] = tag_scale;
                        }
                    }
                }
            }

            // Initialize difficulty bias from problem rating if available
            let difficulty = rng.gen_range(-scale..scale);

            ProblemParams { alpha, difficulty }
        })
        .collect()
}

/// Downsample negative observations (unsolved) to balance the dataset.
/// Returns indices into the observations vector.
fn sample_indices(
    observations: &[Observation],
    negative_sample_ratio: f64,
    rng: &mut impl Rng,
) -> Vec<usize> {
    if negative_sample_ratio <= 0.0 {
        return (0..observations.len()).collect();
    }

    let mut pos_indices = Vec::new();
    let mut neg_indices = Vec::new();

    for (i, obs) in observations.iter().enumerate() {
        if obs.solved {
            pos_indices.push(i);
        } else {
            neg_indices.push(i);
        }
    }

    let target_neg = (pos_indices.len() as f64 * negative_sample_ratio) as usize;
    if target_neg >= neg_indices.len() {
        // No downsampling needed
        return (0..observations.len()).collect();
    }

    neg_indices.shuffle(rng);
    neg_indices.truncate(target_neg);

    let mut indices: Vec<usize> = pos_indices;
    indices.extend(neg_indices);
    indices.sort_unstable();
    indices
}

/// A single epoch's metrics for the training curve.
#[derive(Debug, Clone)]
pub struct EpochMetrics {
    pub epoch: usize,
    pub loss: f64,
    pub train_auc: f64,
}

/// Train the logistic matrix factorization model using mini-batch SGD.
/// Returns the trained model and per-epoch training metrics.
pub fn train_with_curve(
    dataset: &TrainingDataset,
    config: &ModelConfig,
) -> Result<(SolvePredictionModel, Vec<EpochMetrics>)> {
    let (model, curve) = train_inner(dataset, config)?;
    Ok((model, curve))
}

/// Train the logistic matrix factorization model using mini-batch SGD.
pub fn train(dataset: &TrainingDataset, config: &ModelConfig) -> Result<SolvePredictionModel> {
    let (model, _) = train_inner(dataset, config)?;
    Ok(model)
}

fn train_inner(
    dataset: &TrainingDataset,
    config: &ModelConfig,
) -> Result<(SolvePredictionModel, Vec<EpochMetrics>)> {
    let k = config.latent_dim;
    let mut rng = rand::thread_rng();
    let scale = 0.1;

    // Build tag→dim map
    let max_tag_dims = k.min(20);
    let tag_dim_map = if config.tag_init {
        build_tag_dim_map(dataset, max_tag_dims)
    } else {
        HashMap::new()
    };

    // Initialize user params
    let mut user_params: Vec<UserParams> = (0..dataset.num_users)
        .map(|_| {
            let theta: Vec<f64> = (0..k).map(|_| rng.gen_range(-scale..scale)).collect();
            UserParams {
                theta,
                bias: rng.gen_range(-scale..scale),
            }
        })
        .collect();

    // Initialize problem params (tag-informed or random)
    let mut problem_params: Vec<ProblemParams> = if config.tag_init && !tag_dim_map.is_empty() {
        initialize_from_tags(
            dataset.num_problems,
            k,
            &dataset.problem_tags,
            &tag_dim_map,
            &mut rng,
        )
    } else {
        (0..dataset.num_problems)
            .map(|_| {
                let alpha: Vec<f64> = (0..k).map(|_| rng.gen_range(-scale..scale)).collect();
                ProblemParams {
                    alpha,
                    difficulty: rng.gen_range(-scale..scale),
                }
            })
            .collect()
    };

    // Initialize problem difficulty bias from ratings when available
    for (p, params) in problem_params.iter_mut().enumerate() {
        if let Some(Some(rating)) = dataset.problem_ratings.get(p) {
            // Normalize: CF ratings ~800-3500, center around 1500, scale down
            params.difficulty = -(*rating as f64 - 1500.0) / 500.0;
        }
    }

    let lr = config.learning_rate;
    let lambda = config.lambda;

    let progress = indicatif::ProgressBar::new(config.epochs as u64);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} epochs | loss: {msg}")
            .unwrap(),
    );

    let mut training_curve = Vec::new();

    for epoch in 0..config.epochs {
        // Optionally sample indices (negative downsampling)
        let indices = sample_indices(&dataset.observations, config.negative_sample_ratio, &mut rng);

        let mut total_loss = 0.0;
        let mut count = 0usize;

        // Shuffle training order
        let mut shuffled = indices.clone();
        shuffled.shuffle(&mut rng);

        for &idx in &shuffled {
            let obs = &dataset.observations[idx];
            let u = obs.user_idx;
            let p = obs.problem_idx;
            let y = if obs.solved { 1.0 } else { 0.0 };

            // Forward pass
            let pred = predict(&user_params[u], &problem_params[p]);
            let error = pred - y; // gradient of BCE w.r.t. logit = (σ(z) - y)

            // Log-loss for monitoring
            let clamped = pred.clamp(1e-7, 1.0 - 1e-7);
            total_loss += -(y * clamped.ln() + (1.0 - y) * (1.0 - clamped).ln());
            count += 1;

            // SGD updates
            // ∂L/∂θ_u = error * a_p + λ * θ_u
            // ∂L/∂a_p = error * θ_u + λ * a_p
            // ∂L/∂b_u = error + λ * b_u
            // ∂L/∂d_p = error + λ * d_p
            for dim in 0..k {
                let grad_theta = error * problem_params[p].alpha[dim]
                    + lambda * user_params[u].theta[dim];
                let grad_alpha = error * user_params[u].theta[dim]
                    + lambda * problem_params[p].alpha[dim];
                user_params[u].theta[dim] -= lr * grad_theta;
                problem_params[p].alpha[dim] -= lr * grad_alpha;
            }

            let grad_bu = error + lambda * user_params[u].bias;
            let grad_dp = error + lambda * problem_params[p].difficulty;
            user_params[u].bias -= lr * grad_bu;
            problem_params[p].difficulty -= lr * grad_dp;
        }

        let avg_loss = if count > 0 {
            total_loss / count as f64
        } else {
            0.0
        };

        progress.set_message(format!("{:.4}", avg_loss));
        progress.inc(1);

        // Compute train AUC every epoch for the training curve
        let mut scored: Vec<(f64, bool)> = dataset
            .observations
            .iter()
            .map(|obs| {
                let p = predict(&user_params[obs.user_idx], &problem_params[obs.problem_idx]);
                (p, obs.solved)
            })
            .collect();
        let auc = quick_auc(&mut scored);

        training_curve.push(EpochMetrics {
            epoch,
            loss: avg_loss,
            train_auc: auc,
        });

        if config.verbose && (epoch % 5 == 0 || epoch == config.epochs - 1) {
            eprintln!(
                "  Epoch {}: loss={:.4}, train_auc={:.4}",
                epoch, avg_loss, auc
            );
        }
    }

    progress.finish_with_message("done");

    Ok((SolvePredictionModel {
        config: config.clone(),
        user_params,
        problem_params,
        user_index: dataset.user_index.clone(),
        problem_index: dataset.problem_index.clone(),
        problem_ratings: dataset.problem_ratings.clone(),
        problem_tags: dataset.problem_tags.clone(),
        tag_dim_map,
    }, training_curve))
}

/// Quick AUC calculation for training monitoring.
fn quick_auc(scored: &mut [(f64, bool)]) -> f64 {
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let total_pos = scored.iter().filter(|(_, y)| *y).count() as f64;
    let total_neg = scored.len() as f64 - total_pos;
    if total_pos == 0.0 || total_neg == 0.0 {
        return 0.5;
    }
    let mut tp = 0.0;
    let mut auc = 0.0;
    for &(_, label) in scored.iter() {
        if label {
            tp += 1.0;
        } else {
            auc += tp;
        }
    }
    auc / (total_pos * total_neg)
}
