use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for model training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub latent_dim: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub lambda: f64,
    pub tag_init: bool,
    pub negative_sample_ratio: f64,
    pub min_contests: usize,
    pub cutoff_timestamp: i64,
    pub verbose: bool,
}

/// Latent parameters for a single user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserParams {
    /// Latent skill vector θ_u ∈ ℝ^k
    pub theta: Vec<f64>,
    /// General ability bias b_u
    pub bias: f64,
}

/// Latent parameters for a single problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemParams {
    /// Skill requirement vector a_p ∈ ℝ^k
    pub alpha: Vec<f64>,
    /// Difficulty bias d_p
    pub difficulty: f64,
}

/// A trained solve-prediction model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolvePredictionModel {
    pub config: ModelConfig,
    pub user_params: Vec<UserParams>,
    pub problem_params: Vec<ProblemParams>,
    pub user_index: HashMap<String, usize>,
    pub problem_index: HashMap<String, usize>,
    pub problem_ratings: Vec<Option<i32>>,
    pub problem_tags: Vec<Vec<String>>,
    /// Tag-to-dimension mapping used during tag-informed init
    pub tag_dim_map: HashMap<String, usize>,
}

/// A problem-only model (no user parameters). Used for cold-start prediction:
/// ship problem embeddings, compute user embeddings on-the-fly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemModel {
    pub latent_dim: usize,
    pub problem_params: Vec<ProblemParams>,
    pub problem_index: HashMap<String, usize>,
    pub problem_ratings: Vec<Option<i32>>,
    pub problem_tags: Vec<Vec<String>>,
    pub tag_dim_map: HashMap<String, usize>,
}

impl From<SolvePredictionModel> for ProblemModel {
    fn from(m: SolvePredictionModel) -> Self {
        ProblemModel {
            latent_dim: m.config.latent_dim,
            problem_params: m.problem_params,
            problem_index: m.problem_index,
            problem_ratings: m.problem_ratings,
            problem_tags: m.problem_tags,
            tag_dim_map: m.tag_dim_map,
        }
    }
}

/// A weighted observation for time-weighted user fitting.
#[derive(Debug, Clone)]
pub struct WeightedObservation {
    pub problem_idx: usize,
    pub solved: bool,
    pub weight: f64,
}

/// A single (user, problem, outcome) observation.
#[derive(Debug, Clone)]
pub struct Observation {
    pub user_idx: usize,
    pub problem_idx: usize,
    pub solved: bool,
    pub user_rating: Option<i32>,
    pub problem_rating: Option<i32>,
    pub contest_timestamp: i64,
}

/// A dataset ready for training or evaluation.
#[derive(Debug)]
pub struct TrainingDataset {
    pub observations: Vec<Observation>,
    pub num_users: usize,
    pub num_problems: usize,
    pub user_index: HashMap<String, usize>,
    pub problem_index: HashMap<String, usize>,
    pub problem_ratings: Vec<Option<i32>>,
    pub problem_tags: Vec<Vec<String>>,
}
