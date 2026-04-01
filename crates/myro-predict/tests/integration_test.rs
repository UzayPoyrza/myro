// Integration tests for myro-predict.
// Now that myro-predict is a lib crate, we can import real types directly.
// Self-contained math tests are kept for clarity; new tests use real types.

use myro_predict::model::eval::{compute_auc, compute_logloss, per_depth_metrics};
use myro_predict::model::inference::{
    fit_user_weighted, predict, predict_all, time_decay_weight,
};
use myro_predict::model::types::{ProblemModel, ProblemParams, UserParams, WeightedObservation};
use std::collections::HashMap;

// ────────────────────────── Helper functions ──────────────────────────

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn predict_inline(
    user_theta: &[f64],
    user_bias: f64,
    prob_alpha: &[f64],
    prob_diff: f64,
) -> f64 {
    sigmoid(dot(user_theta, prob_alpha) + user_bias + prob_diff)
}

// ────────────────────────── Math tests (self-contained) ──────────────────────────

#[test]
fn test_sigmoid_properties() {
    assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
    assert!(sigmoid(10.0) > 0.999);
    assert!(sigmoid(-10.0) < 0.001);
    assert!(sigmoid(1.0) > sigmoid(0.0));
    assert!(sigmoid(0.0) > sigmoid(-1.0));
}

#[test]
fn test_dot_product() {
    assert!((dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]) - 32.0).abs() < 1e-10);
    assert!((dot(&[0.0, 0.0], &[1.0, 1.0]) - 0.0).abs() < 1e-10);
}

#[test]
fn test_predict_extreme_cases() {
    let user = UserParams {
        theta: vec![1.0, 1.0],
        bias: 2.0,
    };
    let easy = ProblemParams {
        alpha: vec![0.5, 0.5],
        difficulty: 1.0,
    };
    let p = predict(&user, &easy);
    assert!(
        p > 0.95,
        "Strong user + easy problem should give high P(solve), got {}",
        p
    );

    let weak = UserParams {
        theta: vec![-1.0, -1.0],
        bias: -2.0,
    };
    let hard = ProblemParams {
        alpha: vec![0.5, 0.5],
        difficulty: -3.0,
    };
    let p = predict(&weak, &hard);
    assert!(
        p < 0.05,
        "Weak user + hard problem should give low P(solve), got {}",
        p
    );
}

#[test]
fn test_auc_perfect_ranking() {
    let preds = vec![
        (0.9, true),
        (0.8, true),
        (0.7, true),
        (0.3, false),
        (0.2, false),
        (0.1, false),
    ];
    let auc = compute_auc(&preds);
    assert!(
        (auc - 1.0).abs() < 1e-10,
        "Perfect ranking should give AUC=1.0, got {}",
        auc
    );
}

#[test]
fn test_auc_mixed_ranking() {
    let preds = vec![
        (0.9, true),
        (0.8, false),
        (0.7, true),
        (0.6, false),
        (0.5, true),
        (0.4, false),
    ];
    let auc = compute_auc(&preds);
    assert!(
        auc > 0.5 && auc < 1.0,
        "Mixed ranking should give 0.5 < AUC < 1.0, got {}",
        auc
    );
}

#[test]
fn test_auc_inverted_ranking() {
    let preds = vec![(0.1, true), (0.2, true), (0.8, false), (0.9, false)];
    let auc = compute_auc(&preds);
    assert!(auc < 0.1, "Inverted ranking should give AUC~0.0, got {}", auc);
}

#[test]
fn test_logloss_perfect_predictions() {
    let preds = vec![(0.999, true), (0.001, false)];
    let ll = compute_logloss(&preds);
    assert!(
        ll < 0.01,
        "Near-perfect predictions should give low logloss, got {}",
        ll
    );
}

#[test]
fn test_logloss_worst_predictions() {
    let preds = vec![(0.001, true), (0.999, false)];
    let ll = compute_logloss(&preds);
    assert!(
        ll > 5.0,
        "Worst predictions should give high logloss, got {}",
        ll
    );
}

#[test]
fn test_logloss_random_baseline() {
    let preds: Vec<(f64, bool)> = (0..1000).map(|i| (0.5, i % 2 == 0)).collect();
    let ll = compute_logloss(&preds);
    assert!(
        (ll - 0.693).abs() < 0.01,
        "Random baseline logloss should be ~0.693, got {}",
        ll
    );
}

// ────────────────────────── Time decay ──────────────────────────

#[test]
fn test_time_decay_weight() {
    // At t=0, weight should be 1.0
    let w0 = time_decay_weight(0.0, 365.0);
    assert!(
        (w0 - 1.0).abs() < 1e-10,
        "Weight at t=0 should be 1.0, got {}",
        w0
    );

    // At t=half_life, weight should be 0.5
    let w_half = time_decay_weight(365.0, 365.0);
    assert!(
        (w_half - 0.5).abs() < 1e-10,
        "Weight at half-life should be 0.5, got {}",
        w_half
    );

    // Weight should decrease with time
    let w1 = time_decay_weight(100.0, 365.0);
    let w2 = time_decay_weight(200.0, 365.0);
    assert!(
        w1 > w2,
        "Weight should decrease: w(100)={} > w(200)={}",
        w1,
        w2
    );
}

// ────────────────────────── Recency bias test ──────────────────────────

#[test]
fn test_recency_bias() {
    // Build a small ProblemModel
    let k = 3;
    let problem = ProblemParams {
        alpha: vec![0.5, 0.3, -0.2],
        difficulty: -0.5,
    };
    let model = ProblemModel {
        latent_dim: k,
        problem_params: vec![problem],
        problem_index: HashMap::from([("1:A".to_string(), 0)]),
        problem_ratings: vec![Some(1500)],
        problem_tags: vec![vec![]],
        tag_dim_map: HashMap::new(),
    };

    // 10 old failures (low weight) + 1 recent solve (high weight)
    let mut weighted_obs = Vec::new();
    for _ in 0..10 {
        weighted_obs.push(WeightedObservation {
            problem_idx: 0,
            solved: false,
            weight: 0.1,
        });
    }
    weighted_obs.push(WeightedObservation {
        problem_idx: 0,
        solved: true,
        weight: 1.0,
    });

    // Uniform-weight version
    let uniform_obs: Vec<WeightedObservation> = (0..10)
        .map(|_| WeightedObservation {
            problem_idx: 0,
            solved: false,
            weight: 1.0,
        })
        .chain(std::iter::once(WeightedObservation {
            problem_idx: 0,
            solved: true,
            weight: 1.0,
        }))
        .collect();

    let weighted_user = fit_user_weighted(&model, &weighted_obs, 0.01, 100, 0.01);
    let uniform_user = fit_user_weighted(&model, &uniform_obs, 0.01, 100, 0.01);

    let weighted_pred = predict(&weighted_user, &model.problem_params[0]);
    let uniform_pred = predict(&uniform_user, &model.problem_params[0]);

    assert!(
        weighted_pred > uniform_pred,
        "Weighted pred ({:.4}) should be higher than uniform ({:.4}) — recent solve should dominate",
        weighted_pred,
        uniform_pred
    );
}

// ────────────────────────── predict_all consistency ──────────────────────────

#[test]
fn test_predict_all_consistency() {
    let k = 4;
    let user = UserParams {
        theta: vec![0.5, -0.3, 0.1, 0.8],
        bias: 0.2,
    };

    let problems: Vec<ProblemParams> = (0..20)
        .map(|i| ProblemParams {
            alpha: (0..k)
                .map(|d| ((i * 7 + d * 13) % 100) as f64 / 100.0 - 0.5)
                .collect(),
            difficulty: (i as f64 - 10.0) / 10.0,
        })
        .collect();

    let model = ProblemModel {
        latent_dim: k,
        problem_params: problems.clone(),
        problem_index: (0..20)
            .map(|i| (format!("{}:A", i), i))
            .collect(),
        problem_ratings: vec![None; 20],
        problem_tags: vec![vec![]; 20],
        tag_dim_map: HashMap::new(),
    };

    let batch = predict_all(&user, &model);
    assert_eq!(batch.len(), 20);

    for (i, &p) in batch.iter().enumerate() {
        let single = predict(&user, &problems[i]);
        assert!(
            (p - single).abs() < 1e-10,
            "predict_all[{}]={:.6} != predict={:.6}",
            i,
            p,
            single
        );
    }
}

// ────────────────────────── Synthetic SGD (self-contained) ──────────────────────────

#[test]
fn test_synthetic_sgd_training() {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let k = 5;
    let num_users = 50;
    let num_problems = 30;

    let true_user_theta: Vec<Vec<f64>> = (0..num_users)
        .map(|_| (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect();
    let true_user_bias: Vec<f64> = (0..num_users).map(|_| rng.gen_range(-0.5..0.5)).collect();
    let true_prob_alpha: Vec<Vec<f64>> = (0..num_problems)
        .map(|_| (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect();
    let true_prob_diff: Vec<f64> =
        (0..num_problems).map(|_| rng.gen_range(-1.0..1.0)).collect();

    struct Obs {
        user: usize,
        problem: usize,
        solved: bool,
    }

    let mut observations = Vec::new();
    for u in 0..num_users {
        for p in 0..num_problems {
            let prob = predict_inline(
                &true_user_theta[u],
                true_user_bias[u],
                &true_prob_alpha[p],
                true_prob_diff[p],
            );
            let solved = rng.gen::<f64>() < prob;
            observations.push(Obs {
                user: u,
                problem: p,
                solved,
            });
        }
    }

    let scale = 0.1;
    let mut user_theta: Vec<Vec<f64>> = (0..num_users)
        .map(|_| (0..k).map(|_| rng.gen_range(-scale..scale)).collect())
        .collect();
    let mut user_bias = vec![0.0; num_users];
    let mut prob_alpha: Vec<Vec<f64>> = (0..num_problems)
        .map(|_| (0..k).map(|_| rng.gen_range(-scale..scale)).collect())
        .collect();
    let mut prob_diff = vec![0.0; num_problems];

    let lr = 0.05;
    let lambda = 0.001;
    let epochs = 30;

    for _ in 0..epochs {
        for obs in &observations {
            let u = obs.user;
            let p = obs.problem;
            let pred =
                predict_inline(&user_theta[u], user_bias[u], &prob_alpha[p], prob_diff[p]);
            let y = if obs.solved { 1.0 } else { 0.0 };
            let error = pred - y;

            for dim in 0..k {
                let gt = error * prob_alpha[p][dim] + lambda * user_theta[u][dim];
                let ga = error * user_theta[u][dim] + lambda * prob_alpha[p][dim];
                user_theta[u][dim] -= lr * gt;
                prob_alpha[p][dim] -= lr * ga;
            }
            user_bias[u] -= lr * (error + lambda * user_bias[u]);
            prob_diff[p] -= lr * (error + lambda * prob_diff[p]);
        }
    }

    let preds: Vec<(f64, bool)> = observations
        .iter()
        .map(|obs| {
            let p = predict_inline(
                &user_theta[obs.user],
                user_bias[obs.user],
                &prob_alpha[obs.problem],
                prob_diff[obs.problem],
            );
            (p, obs.solved)
        })
        .collect();

    let auc = compute_auc(&preds);
    let ll = compute_logloss(&preds);

    assert!(auc > 0.70, "Trained model AUC should be > 0.70, got {:.4}", auc);
    assert!(ll < 0.65, "Trained model logloss should be < 0.65, got {:.4}", ll);

    let random_preds: Vec<(f64, bool)> = observations.iter().map(|obs| (0.5, obs.solved)).collect();
    let random_ll = compute_logloss(&random_preds);
    assert!(
        ll < random_ll,
        "Model logloss ({:.4}) should beat random ({:.4})",
        ll,
        random_ll
    );
}

// ────────────────────────── Cold-start e2e ──────────────────────────

/// Generate synthetic data, train SGD, convert to ProblemModel,
/// fit multiple new unseen users with fit_user_weighted, predict_all,
/// verify average AUC > 0.55.
#[test]
fn test_coldstart_e2e() {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(42);

    let k = 5;
    let num_train_users = 50;
    let num_problems = 60;
    let num_test_users = 20;

    // Ground truth problem parameters
    let true_prob_alpha: Vec<Vec<f64>> = (0..num_problems)
        .map(|_| (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect();
    let true_prob_diff: Vec<f64> =
        (0..num_problems).map(|_| rng.gen_range(-1.0..1.0)).collect();

    // Train users
    let true_train_theta: Vec<Vec<f64>> = (0..num_train_users)
        .map(|_| (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect();
    let true_train_bias: Vec<f64> =
        (0..num_train_users).map(|_| rng.gen_range(-0.5..0.5)).collect();

    // Train SGD on training users
    let scale = 0.1;
    let mut user_theta: Vec<Vec<f64>> = (0..num_train_users)
        .map(|_| (0..k).map(|_| rng.gen_range(-scale..scale)).collect())
        .collect();
    let mut user_bias = vec![0.0; num_train_users];
    let mut prob_alpha: Vec<Vec<f64>> = (0..num_problems)
        .map(|_| (0..k).map(|_| rng.gen_range(-scale..scale)).collect())
        .collect();
    let mut prob_diff = vec![0.0; num_problems];

    struct Obs {
        user: usize,
        problem: usize,
        solved: bool,
    }

    let mut train_obs = Vec::new();
    for u in 0..num_train_users {
        for p in 0..num_problems {
            let prob = predict_inline(
                &true_train_theta[u],
                true_train_bias[u],
                &true_prob_alpha[p],
                true_prob_diff[p],
            );
            let solved = rng.gen::<f64>() < prob;
            train_obs.push(Obs {
                user: u,
                problem: p,
                solved,
            });
        }
    }

    for _ in 0..40 {
        for obs in &train_obs {
            let u = obs.user;
            let p = obs.problem;
            let pred =
                predict_inline(&user_theta[u], user_bias[u], &prob_alpha[p], prob_diff[p]);
            let y = if obs.solved { 1.0 } else { 0.0 };
            let error = pred - y;
            for dim in 0..k {
                let gt = error * prob_alpha[p][dim] + 0.001 * user_theta[u][dim];
                let ga = error * user_theta[u][dim] + 0.001 * prob_alpha[p][dim];
                user_theta[u][dim] -= 0.05 * gt;
                prob_alpha[p][dim] -= 0.05 * ga;
            }
            user_bias[u] -= 0.05 * (error + 0.001 * user_bias[u]);
            prob_diff[p] -= 0.05 * (error + 0.001 * prob_diff[p]);
        }
    }

    // Build ProblemModel from trained parameters
    let problem_params: Vec<ProblemParams> = (0..num_problems)
        .map(|p| ProblemParams {
            alpha: prob_alpha[p].clone(),
            difficulty: prob_diff[p],
        })
        .collect();

    let problem_model = ProblemModel {
        latent_dim: k,
        problem_params,
        problem_index: (0..num_problems)
            .map(|i| (format!("{}:A", i), i))
            .collect(),
        problem_ratings: vec![None; num_problems],
        problem_tags: vec![vec![]; num_problems],
        tag_dim_map: HashMap::new(),
    };

    // Test cold-start on multiple unseen users, average AUC
    let train_problems: Vec<usize> = (0..num_problems / 2).collect();
    let test_problems: Vec<usize> = (num_problems / 2..num_problems).collect();
    let mut all_test_preds: Vec<(f64, bool)> = Vec::new();

    for _ in 0..num_test_users {
        let new_user_theta: Vec<f64> = (0..k).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let new_user_bias: f64 = rng.gen_range(-0.5..0.5);

        let history: Vec<WeightedObservation> = train_problems
            .iter()
            .map(|&p| {
                let prob = predict_inline(
                    &new_user_theta,
                    new_user_bias,
                    &true_prob_alpha[p],
                    true_prob_diff[p],
                );
                let solved = rng.gen::<f64>() < prob;
                WeightedObservation {
                    problem_idx: p,
                    solved,
                    weight: 1.0,
                }
            })
            .collect();

        let fitted = fit_user_weighted(&problem_model, &history, 0.01, 100, 0.01);
        let preds = predict_all(&fitted, &problem_model);

        for &p in &test_problems {
            let true_prob = predict_inline(
                &new_user_theta,
                new_user_bias,
                &true_prob_alpha[p],
                true_prob_diff[p],
            );
            let solved = rng.gen::<f64>() < true_prob;
            all_test_preds.push((preds[p], solved));
        }
    }

    let auc = compute_auc(&all_test_preds);
    assert!(
        auc > 0.55,
        "Cold-start average AUC over {} users ({} predictions) should be > 0.55, got {:.4}",
        num_test_users,
        all_test_preds.len(),
        auc
    );
}

// ────────────────────────── per_depth_metrics ──────────────────────────

#[test]
fn test_per_depth_metrics() {
    // Construct synthetic predictions with varying depth
    let mut preds: Vec<(f64, bool, usize)> = Vec::new();

    // Depth 5-9: low-quality predictions (AUC ~0.5)
    for i in 0..100 {
        preds.push((0.5, i % 2 == 0, 7));
    }

    // Depth 10-19: better predictions
    for i in 0..100 {
        let pred = if i % 2 == 0 { 0.8 } else { 0.2 };
        preds.push((pred, i % 2 == 0, 15));
    }

    // Depth 20-49: good predictions
    for i in 0..100 {
        let pred = if i % 2 == 0 { 0.9 } else { 0.1 };
        preds.push((pred, i % 2 == 0, 35));
    }

    // Depth 50+: great predictions
    for i in 0..100 {
        let pred = if i % 2 == 0 { 0.95 } else { 0.05 };
        preds.push((pred, i % 2 == 0, 60));
    }

    let results = per_depth_metrics(&preds);

    assert_eq!(results.len(), 4, "Should have 4 depth bins");
    assert_eq!(results[0].0, "5-10");
    assert_eq!(results[1].0, "10-20");
    assert_eq!(results[2].0, "20-50");
    assert_eq!(results[3].0, "50+");

    // Each bin should have 100 predictions
    for (_, _, _, n) in &results {
        assert_eq!(*n, 100);
    }

    // AUC should increase with depth (better predictions for deeper bins)
    assert!(
        results[0].1 < results[1].1,
        "AUC should improve: 5-10 ({:.4}) < 10-20 ({:.4})",
        results[0].1,
        results[1].1
    );
    assert!(
        results[1].1 <= results[2].1,
        "AUC should improve: 10-20 ({:.4}) <= 20-50 ({:.4})",
        results[1].1,
        results[2].1
    );
}
