use serde::{Deserialize, Serialize};

/// Profile row in Supabase `profiles` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub user_id: String,
    pub cf_handle: Option<String>,
    pub display_name: Option<String>,
    pub target_probability: Option<f64>,
    pub created_at: Option<String>,
}

/// Row in `solved_problems` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolvedProblemRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user_id: String,
    pub problem_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solved_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_taken_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Row in `past_entries` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastEntryRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user_id: String,
    pub contest_id: i64,
    pub index: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ever_accepted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ever_submitted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_seen_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_submitted_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_submitted_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_taken_secs: Option<i64>,
}

/// Row in `skill_snapshots` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshotRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user_id: String,
    pub tag: String,
    pub mu: f64,
    pub phi: f64,
    pub sigma: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

/// Row in `coaching_sessions` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachingSessionRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user_id: String,
    pub problem_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observations: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

/// Row in `solutions` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user_id: String,
    pub problem_id: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
}

/// Row in `events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub user_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}
