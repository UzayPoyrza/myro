use std::path::PathBuf;

use myro_cf::types::ProblemStatement;
use myro_predict::model::skills::{SkillDelta, SkillProfile};

use crate::recommend::{self, RecommendEvent, RecommendHandle, RecommendRequest};
use crate::state::{self, SavedProblem, UserState};

pub type PendingProblem = (i64, String, f64, Option<i32>);

pub struct RecommenderState {
    handle: Option<RecommendHandle>,
    pub status: Option<String>,
    pub pending_problem: Option<PendingProblem>,
    pub skill_profile: Option<SkillProfile>,
    pub skill_deltas: Option<Vec<SkillDelta>>,
    pub skill_delta_tick: u64,
    pub deferred_problem: Option<ProblemStatement>,
    pub skip_auto_recommend: bool,
}

impl RecommenderState {
    /// Construct an empty state for testing (no filesystem access).
    pub fn empty() -> Self {
        Self {
            handle: None,
            status: None,
            pending_problem: None,
            skill_profile: None,
            skill_deltas: None,
            skill_delta_tick: 0,
            deferred_problem: None,
            skip_auto_recommend: false,
        }
    }

    pub fn new(user_state: &UserState) -> Self {
        let pending_problem = user_state
            .saved_problem
            .as_ref()
            .map(|saved| {
                (
                    saved.contest_id,
                    saved.index.clone(),
                    saved.predicted_p,
                    saved.rating,
                )
            });

        let skill_profile = std::fs::read_to_string(skill_profile_path())
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok());

        Self {
            handle: None,
            status: None,
            pending_problem,
            skill_profile,
            skill_deltas: None,
            skill_delta_tick: 0,
            deferred_problem: None,
            skip_auto_recommend: false,
        }
    }

    pub fn ensure_started(&mut self) {
        if self.handle.is_none() {
            self.handle = Some(recommend::spawn_recommender());
        }
    }

    pub fn send(&mut self, request: RecommendRequest) {
        self.ensure_started();
        if let Some(handle) = &self.handle {
            let _ = handle.request_tx.send(request);
        }
    }

    pub fn take_events(&mut self) -> Vec<RecommendEvent> {
        let mut events = Vec::new();
        if let Some(handle) = &self.handle {
            while let Ok(event) = handle.event_rx.try_recv() {
                events.push(event);
            }
        }
        events
    }

    pub fn cache_pending_problem(
        &mut self,
        user_state: &mut UserState,
        contest_id: i64,
        index: String,
        predicted_p: f64,
        rating: Option<i32>,
    ) {
        self.pending_problem = Some((contest_id, index.clone(), predicted_p, rating));
        user_state.saved_problem = Some(SavedProblem {
            contest_id,
            index,
            predicted_p,
            rating,
        });
        let _ = state::save_state(user_state);
    }

    pub fn clear_pending_problem(&mut self, user_state: &mut UserState) {
        self.pending_problem = None;
        user_state.saved_problem = None;
        let _ = state::save_state(user_state);
    }

    pub fn store_skill_profile(&mut self, profile: SkillProfile) {
        if let Ok(json) = serde_json::to_string(&profile) {
            let _ = std::fs::write(skill_profile_path(), json);
        }
        self.skill_profile = Some(profile);
    }

    pub fn clear_profile_cache(&mut self) {
        self.skill_profile = None;
        let _ = std::fs::remove_file(skill_profile_path());
    }

    pub fn clear_runtime_state(&mut self) {
        self.status = None;
        self.pending_problem = None;
        self.skill_deltas = None;
        self.skill_delta_tick = 0;
        self.deferred_problem = None;
        self.skip_auto_recommend = false;
        self.handle = None;
    }

    pub fn clear_cached_data(&mut self) {
        let _ = std::fs::remove_file(recommend::history_path());
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("myro");
        let _ = std::fs::remove_file(data_dir.join("user_params.bin"));
        self.clear_profile_cache();
        self.clear_runtime_state();
    }

    pub fn skill_popup_open(&self) -> bool {
        self.skill_deltas.is_some()
    }
}

fn skill_profile_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("skill_profile.json")
}
