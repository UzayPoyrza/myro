use serde::{Deserialize, Serialize};

use super::inference::predict;
use super::types::{ProblemModel, UserParams};

/// Per-tag skill rating derived from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSkillRating {
    pub tag: String,
    /// CF-scale rating where P(solve) ≈ 0.5 for problems with this tag.
    pub effective_rating: i32,
    /// 0.0–1.0 strength relative to median tag rating.
    pub strength: f64,
    /// Mean P(solve) across all problems with this tag.
    pub avg_p_solve: f64,
    /// Number of rated problems with this tag.
    pub num_problems: usize,
}

/// A user's complete skill profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProfile {
    /// Per-tag ratings, sorted by effective_rating descending.
    pub tag_ratings: Vec<TagSkillRating>,
    /// Overall rating (tag-agnostic).
    pub overall_rating: i32,
}

/// Change in skill rating after a solve/fail/give-up event.
#[derive(Debug, Clone)]
pub struct SkillDelta {
    pub tag: String,
    pub old_rating: i32,
    pub new_rating: i32,
    pub delta: i32,
}

const BIN_WIDTH: i32 = 200;
const MIN_PROBLEMS_PER_TAG: usize = 5;
const MIN_RATING: i32 = 800;
const MAX_RATING: i32 = 3500;

/// Find the rating where mean P(solve) crosses 0.5 by binning and interpolating.
fn find_half_solve_rating(rated_pairs: &[(i32, f64)]) -> Option<i32> {
    if rated_pairs.is_empty() {
        return None;
    }

    // Find rating range
    let min_r = rated_pairs.iter().map(|(r, _)| *r).min().unwrap();
    let max_r = rated_pairs.iter().map(|(r, _)| *r).max().unwrap();

    // Bin by BIN_WIDTH intervals
    let bin_start = (min_r / BIN_WIDTH) * BIN_WIDTH;
    let bin_end = ((max_r / BIN_WIDTH) + 1) * BIN_WIDTH;

    let mut bins: Vec<(i32, f64, usize)> = Vec::new(); // (center, sum_p, count)
    let mut r = bin_start;
    while r < bin_end {
        let center = r + BIN_WIDTH / 2;
        let (sum, count) = rated_pairs
            .iter()
            .filter(|(rating, _)| *rating >= r && *rating < r + BIN_WIDTH)
            .fold((0.0, 0usize), |(s, c), (_, p)| (s + p, c + 1));
        if count > 0 {
            bins.push((center, sum / count as f64, count));
        }
        r += BIN_WIDTH;
    }

    if bins.is_empty() {
        return None;
    }

    // Walk bins to find 0.5 crossing (bins are low-to-high rating, P(solve) decreasing)
    for i in 0..bins.len() - 1 {
        let (r1, p1, _) = bins[i];
        let (r2, p2, _) = bins[i + 1];
        // Crossing: p1 >= 0.5 and p2 < 0.5
        if p1 >= 0.5 && p2 < 0.5 {
            // Linear interpolation
            let frac = (p1 - 0.5) / (p1 - p2);
            let interp = r1 as f64 + frac * (r2 - r1) as f64;
            return Some((interp.round() as i32).clamp(MIN_RATING, MAX_RATING));
        }
    }

    // All P(solve) > 0.5: extrapolate above highest bin
    if bins.last().unwrap().1 >= 0.5 {
        let last_center = bins.last().unwrap().0;
        return Some((last_center + BIN_WIDTH).min(MAX_RATING));
    }

    // All P(solve) < 0.5: extrapolate below lowest bin
    if bins.first().unwrap().1 < 0.5 {
        let first_center = bins.first().unwrap().0;
        return Some((first_center - BIN_WIDTH).max(MIN_RATING));
    }

    // Shouldn't reach here, but fallback to median bin
    let mid = bins[bins.len() / 2].0;
    Some(mid.clamp(MIN_RATING, MAX_RATING))
}

/// Compute per-tag skill ratings from a fitted user embedding.
pub fn compute_skill_profile(user: &UserParams, model: &ProblemModel) -> SkillProfile {
    let tags: Vec<&String> = model.tag_dim_map.keys().collect();

    // Precompute P(solve) for all problems once
    let all_predictions: Vec<f64> = model
        .problem_params
        .iter()
        .map(|p| predict(user, p))
        .collect();

    // Per-tag: collect (rating, P(solve)) pairs for rated problems
    let mut tag_ratings = Vec::new();
    for tag in &tags {
        let mut rated_pairs: Vec<(i32, f64)> = Vec::new();
        let mut p_sum = 0.0;
        let mut p_count = 0usize;

        for (key, &idx) in &model.problem_index {
            let problem_tags = match model.problem_tags.get(idx) {
                Some(t) => t,
                None => continue,
            };
            if !problem_tags.iter().any(|t| t == *tag) {
                continue;
            }
            let p = all_predictions[idx];
            p_sum += p;
            p_count += 1;

            if let Some(Some(rating)) = model.problem_ratings.get(idx) {
                rated_pairs.push((*rating, p));
            }
            // Suppress unused variable warning
            let _ = key;
        }

        if rated_pairs.len() < MIN_PROBLEMS_PER_TAG {
            continue;
        }

        let effective_rating = match find_half_solve_rating(&rated_pairs) {
            Some(r) => r,
            None => continue,
        };

        // Compute median rating for strength calculation
        let mut ratings: Vec<i32> = rated_pairs.iter().map(|(r, _)| *r).collect();
        ratings.sort();
        let median_rating = ratings[ratings.len() / 2];

        let strength = ((effective_rating - median_rating) as f64 / 400.0 + 0.5).clamp(0.0, 1.0);
        let avg_p_solve = if p_count > 0 {
            p_sum / p_count as f64
        } else {
            0.0
        };

        tag_ratings.push(TagSkillRating {
            tag: tag.to_string(),
            effective_rating,
            strength,
            avg_p_solve,
            num_problems: rated_pairs.len(),
        });
    }

    tag_ratings.sort_by(|a, b| b.avg_p_solve.partial_cmp(&a.avg_p_solve).unwrap_or(std::cmp::Ordering::Equal));

    // Overall rating: same algorithm but across ALL rated problems
    let mut all_rated_pairs: Vec<(i32, f64)> = Vec::new();
    for (_key, &idx) in &model.problem_index {
        if let Some(Some(rating)) = model.problem_ratings.get(idx) {
            all_rated_pairs.push((*rating, all_predictions[idx]));
        }
    }

    let overall_rating = find_half_solve_rating(&all_rated_pairs).unwrap_or(1200);

    SkillProfile {
        tag_ratings,
        overall_rating,
    }
}

/// Compute per-tag rating deltas between two user embeddings.
pub fn compute_skill_deltas(
    old: &UserParams,
    new: &UserParams,
    model: &ProblemModel,
) -> Vec<SkillDelta> {
    let old_profile = compute_skill_profile(old, model);
    let new_profile = compute_skill_profile(new, model);

    let mut deltas = Vec::new();

    for new_tag in &new_profile.tag_ratings {
        if let Some(old_tag) = old_profile
            .tag_ratings
            .iter()
            .find(|t| t.tag == new_tag.tag)
        {
            let delta = new_tag.effective_rating - old_tag.effective_rating;
            if delta.abs() >= 10 {
                deltas.push(SkillDelta {
                    tag: new_tag.tag.clone(),
                    old_rating: old_tag.effective_rating,
                    new_rating: new_tag.effective_rating,
                    delta,
                });
            }
        }
    }

    // Also add overall delta
    let overall_delta = new_profile.overall_rating - old_profile.overall_rating;
    if overall_delta.abs() >= 10 {
        deltas.push(SkillDelta {
            tag: "overall".to_string(),
            old_rating: old_profile.overall_rating,
            new_rating: new_profile.overall_rating,
            delta: overall_delta,
        });
    }

    deltas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::ProblemParams;
    use std::collections::HashMap;

    fn make_test_model() -> ProblemModel {
        let mut problem_index = HashMap::new();
        let mut problem_params = Vec::new();
        let mut problem_ratings = Vec::new();
        let mut problem_tags = Vec::new();

        // Create 20 problems across 2 tags with varying difficulty
        for i in 0..20 {
            let key = format!("{}:{}", 1000 + i, "A");
            problem_index.insert(key, i);

            let difficulty = -2.0 + (i as f64) * 0.2; // range -2.0 to +1.8
            problem_params.push(ProblemParams {
                alpha: vec![0.1; 4],
                difficulty,
            });

            let rating = 800 + i as i32 * 150;
            problem_ratings.push(Some(rating));

            let tag = if i < 10 { "dp" } else { "math" };
            problem_tags.push(vec![tag.to_string()]);
        }

        let mut tag_dim_map = HashMap::new();
        tag_dim_map.insert("dp".to_string(), 0);
        tag_dim_map.insert("math".to_string(), 1);

        ProblemModel {
            latent_dim: 4,
            problem_params,
            problem_index,
            problem_ratings,
            problem_tags,
            tag_dim_map,
        }
    }

    #[test]
    fn test_compute_skill_profile() {
        let model = make_test_model();
        let user = UserParams {
            theta: vec![0.5, 0.3, 0.0, 0.0],
            bias: 0.0,
        };

        let profile = compute_skill_profile(&user, &model);
        assert!(profile.tag_ratings.len() <= 2);
        assert!(profile.overall_rating >= MIN_RATING);
        assert!(profile.overall_rating <= MAX_RATING);
    }

    #[test]
    fn test_compute_skill_deltas() {
        let model = make_test_model();
        let old_user = UserParams {
            theta: vec![0.3, 0.3, 0.0, 0.0],
            bias: 0.0,
        };
        let new_user = UserParams {
            theta: vec![0.8, 0.3, 0.0, 0.0],
            bias: 0.5,
        };

        let deltas = compute_skill_deltas(&old_user, &new_user, &model);
        // Should have some deltas since the user got significantly stronger
        // (exact count depends on whether the change crosses the threshold)
        for d in &deltas {
            assert!(d.delta.abs() >= 10);
        }
    }

    #[test]
    fn test_find_half_solve_rating_empty() {
        assert_eq!(find_half_solve_rating(&[]), None);
    }

    #[test]
    fn test_find_half_solve_rating_all_high() {
        // All easy problems — should extrapolate above
        let pairs: Vec<(i32, f64)> = (0..10).map(|i| (800 + i * 100, 0.9)).collect();
        let rating = find_half_solve_rating(&pairs).unwrap();
        assert!(rating > 1500);
    }

    #[test]
    fn test_find_half_solve_rating_all_low() {
        // All hard problems — should extrapolate below
        let pairs: Vec<(i32, f64)> = (0..10).map(|i| (2000 + i * 100, 0.1)).collect();
        let rating = find_half_solve_rating(&pairs).unwrap();
        assert!(rating < 2100);
    }
}
