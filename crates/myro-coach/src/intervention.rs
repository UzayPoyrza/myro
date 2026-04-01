use std::time::Instant;

use crate::types::InterventionTrigger;

pub struct InterventionEngine {
    /// When the last edit was made
    last_edit_at: Instant,
    /// Previous code snapshot for rewrite detection
    last_snapshot: String,
    /// Lines in last snapshot
    last_snapshot_lines: usize,
    /// Total interventions this session (for debug)
    intervention_count: u32,
    /// Automatic interventions (test failures only)
    auto_intervention_count: u32,
    /// Max automatic interventions allowed
    max_auto_interventions: u32,
    /// Stall threshold in seconds
    stall_threshold_secs: u64,
    /// Whether we already triggered a stall (don't repeat)
    stall_triggered: bool,
}

impl InterventionEngine {
    pub fn new(stall_threshold_secs: u64, max_auto_interventions: u32) -> Self {
        Self {
            last_edit_at: Instant::now(),
            last_snapshot: String::new(),
            last_snapshot_lines: 0,
            intervention_count: 0,
            auto_intervention_count: 0,
            max_auto_interventions,
            stall_threshold_secs,
            stall_triggered: false,
        }
    }

    /// Call this every time the editor content changes.
    pub fn record_edit(&mut self, line_count: usize) {
        self.last_edit_at = Instant::now();
        self.last_snapshot_lines = line_count;
        self.stall_triggered = false;
    }

    /// Update the code snapshot (call periodically, e.g., every 5 seconds).
    pub fn update_snapshot(&mut self, code: &str) {
        self.last_snapshot_lines = code.lines().count();
        self.last_snapshot = code.to_string();
    }

    /// Check for stall trigger. Returns Some(Stall) if user has been idle.
    /// This is now used for local ghost nudge only — no LLM call.
    pub fn check_for_triggers(&mut self) -> Option<InterventionTrigger> {
        let now = Instant::now();
        let idle_secs = now.duration_since(self.last_edit_at).as_secs();

        // Stall detection — returns trigger for local ghost nudge (no budget consumed)
        if idle_secs >= self.stall_threshold_secs && !self.stall_triggered {
            self.stall_triggered = true;
            return Some(InterventionTrigger::Stall { idle_secs });
        }

        None
    }

    /// Notify of test failure (auto-trigger, uses auto budget).
    pub fn on_test_failure(&mut self) -> bool {
        if self.auto_intervention_count >= self.max_auto_interventions {
            return false;
        }
        self.auto_intervention_count += 1;
        self.intervention_count += 1;
        true
    }

    /// User explicitly requested help — no cap.
    pub fn on_user_request(&mut self) {
        self.intervention_count += 1;
    }

    pub fn intervention_count(&self) -> u32 {
        self.intervention_count
    }

    pub fn idle_secs(&self) -> u64 {
        Instant::now().duration_since(self.last_edit_at).as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Helper to create an engine with a past `last_edit_at` so we can test
    /// stall detection without sleeping.
    fn engine_with_stale_edit(stall_secs: u64, max_auto: u32) -> InterventionEngine {
        let mut engine = InterventionEngine::new(stall_secs, max_auto);
        engine.last_edit_at = Instant::now() - Duration::from_secs(stall_secs + 1);
        engine
    }

    #[test]
    fn test_stall_detection_triggers_after_threshold() {
        let mut engine = engine_with_stale_edit(5, 3);
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_some());
        match trigger.unwrap() {
            InterventionTrigger::Stall { idle_secs } => {
                assert!(idle_secs >= 5);
            }
            other => panic!("Expected Stall, got {:?}", other),
        }
    }

    #[test]
    fn test_stall_does_not_retrigger() {
        let mut engine = engine_with_stale_edit(5, 3);
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_some());
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_none());
    }

    #[test]
    fn test_stall_resets_on_edit() {
        let mut engine = engine_with_stale_edit(5, 3);
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_some());

        engine.record_edit(10);
        engine.last_edit_at = Instant::now() - Duration::from_secs(6);

        let trigger = engine.check_for_triggers();
        assert!(trigger.is_some());
    }

    #[test]
    fn test_auto_interventions_capped() {
        let mut engine = InterventionEngine::new(60, 2);
        assert!(engine.on_test_failure());
        assert!(engine.on_test_failure());
        assert!(!engine.on_test_failure()); // capped at 2
        assert_eq!(engine.intervention_count(), 2);
    }

    #[test]
    fn test_user_request_never_capped() {
        let mut engine = InterventionEngine::new(60, 0); // 0 auto budget
        // User requests should always work, even with 0 auto budget
        for _ in 0..20 {
            engine.on_user_request();
        }
        assert_eq!(engine.intervention_count(), 20);
    }

    #[test]
    fn test_stall_does_not_consume_budget() {
        let mut engine = engine_with_stale_edit(5, 3);
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_some());
        // Stall should not increment any counter
        assert_eq!(engine.intervention_count(), 0);
        assert_eq!(engine.auto_intervention_count, 0);
    }

    #[test]
    fn test_no_stall_before_threshold() {
        let mut engine = InterventionEngine::new(60, 3);
        let trigger = engine.check_for_triggers();
        assert!(trigger.is_none());
    }

    #[test]
    fn test_idle_secs() {
        let mut engine = InterventionEngine::new(60, 3);
        engine.last_edit_at = Instant::now() - Duration::from_secs(10);
        let idle = engine.idle_secs();
        assert!(idle >= 10);
    }
}
