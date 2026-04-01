use anyhow::Result;
use myro_api::SupabaseClient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserState {
    pub name: Option<String>,
    pub solved: Vec<String>,
    #[serde(default)]
    pub isuck_explained: bool,
    /// Saved recommended problem: (contest_id, index, predicted_p, rating).
    /// Persists across restarts; cleared by /isuck or new recommendation.
    #[serde(default)]
    pub saved_problem: Option<SavedProblem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedProblem {
    pub contest_id: i64,
    pub index: String,
    pub predicted_p: f64,
    pub rating: Option<i32>,
}

impl UserState {
    pub fn is_solved(&self, problem_id: &str) -> bool {
        self.solved.iter().any(|s| s == problem_id)
    }
}

fn state_file_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("state.json")
}

pub fn load_state() -> UserState {
    let path = state_file_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        UserState::default()
    }
}

pub fn save_state(state: &UserState) -> Result<()> {
    if std::env::var("MYRO_EPHEMERAL").is_ok() {
        return Ok(());
    }
    let path = state_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// --- Past entries (separate file) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastEntry {
    pub contest_id: i64,
    pub index: String,
    pub title: String,
    pub rating: Option<i32>,
    pub tags: Vec<String>,
    pub mode: String,
    pub outcome: String,
    pub last_verdict: Option<String>,
    #[serde(default)]
    pub ever_accepted: bool,
    #[serde(default)]
    pub ever_submitted: bool,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub first_submitted_at: Option<i64>,
    pub last_submitted_at: Option<i64>,
    pub finished_at: Option<i64>,
    /// Seconds spent solving (wall clock, excludes paused time like rating popups).
    #[serde(default)]
    pub time_taken_secs: Option<u64>,
}

fn past_file_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("past.json")
}

pub fn load_past() -> Vec<PastEntry> {
    let path = past_file_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_past(entries: &[PastEntry]) -> Result<()> {
    if std::env::var("MYRO_EPHEMERAL").is_ok() {
        return Ok(());
    }
    let path = past_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Push solved problem list to Supabase (best-effort, non-blocking).
pub fn sync_solved_to_remote(client: &SupabaseClient, solved: &[String]) {
    let rows: Vec<myro_api::types::SolvedProblemRow> = solved
        .iter()
        .map(|problem_id| myro_api::types::SolvedProblemRow {
            id: None,
            user_id: client.user_id.clone(),
            problem_id: problem_id.clone(),
            solved_at: None,
            time_taken_secs: None,
            rating: None,
            tags: None,
        })
        .collect();
    let _ = myro_api::sync::push_solved(client, &rows);
}

/// Push past entries to Supabase (best-effort, non-blocking).
pub fn sync_past_to_remote(client: &SupabaseClient, entries: &[PastEntry]) {
    let rows: Vec<myro_api::types::PastEntryRow> = entries
        .iter()
        .map(|e| myro_api::types::PastEntryRow {
            id: None,
            user_id: client.user_id.clone(),
            contest_id: e.contest_id,
            index: e.index.clone(),
            title: Some(e.title.clone()),
            rating: e.rating,
            tags: Some(e.tags.clone()),
            mode: Some(e.mode.clone()),
            outcome: Some(e.outcome.clone()),
            last_verdict: e.last_verdict.clone(),
            ever_accepted: Some(e.ever_accepted),
            ever_submitted: Some(e.ever_submitted),
            first_seen_at: Some(e.first_seen_at),
            last_seen_at: Some(e.last_seen_at),
            first_submitted_at: e.first_submitted_at,
            last_submitted_at: e.last_submitted_at,
            finished_at: e.finished_at,
            time_taken_secs: e.time_taken_secs.map(|s| s as i64),
        })
        .collect();
    let _ = myro_api::sync::push_past_entries(client, &rows);
}
