use std::sync::mpsc;
use std::time::Instant;

use crate::app::{App, AppState, LoginPhase, SolveMode, INTENSE_TIMER_SECS};
use crate::onboarding;
use crate::solving;
use crate::state;

impl App {
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let current_tick = self.tick;
        let mut debug_msgs = Vec::new();

        self.tick_status_message(current_tick);
        self.tick_update_check();
        self.tick_auth();
        self.tick_onboarding();
        self.tick_solving_runtime(current_tick, &mut debug_msgs);
        self.tick_intense_timer(&mut debug_msgs);
        self.tick_recommender(current_tick, &mut debug_msgs);
        self.tick_verdict_polling(current_tick);
        self.flush_debug_messages(debug_msgs);
    }

    fn tick_status_message(&mut self, current_tick: u64) {
        if self.status_message.is_some() {
            if self.status_clear_tick == 0 {
                self.status_clear_tick = current_tick;
            } else if current_tick.wrapping_sub(self.status_clear_tick) > 50 {
                self.status_message = None;
                self.status_clear_tick = 0;
            }
        } else {
            self.status_clear_tick = 0;
        }
    }

    fn tick_update_check(&mut self) {
        if let Some(ref rx) = self.update_rx {
            match rx.try_recv() {
                Ok(crate::updater::UpdateEvent::Available { version }) => {
                    self.log_debug(format!("update available: v{}", version));
                    self.update_available = Some(version);
                    self.update_rx = None;
                }
                Ok(crate::updater::UpdateEvent::UpToDate) => {
                    self.update_rx = None;
                }
                Ok(crate::updater::UpdateEvent::Error(e)) => {
                    self.log_debug(format!("update check failed: {}", e));
                    self.update_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.update_rx = None;
                }
            }
        }
    }

    fn tick_auth(&mut self) {
        if let AppState::Login { auth_rx, phase, .. } = &mut self.state {
            if let Some(rx) = auth_rx {
                match rx.try_recv() {
                    Ok(Ok(tokens)) => {
                        let tokens_clone = tokens;
                        // Take ownership of auth_rx by setting to None
                        *auth_rx = None;
                        *phase = LoginPhase::OAuthSuccess;
                        self.complete_auth(tokens_clone);
                    }
                    Ok(Err(msg)) => {
                        *auth_rx = None;
                        *phase = LoginPhase::EmailInput {
                            email: String::new(),
                            password: String::new(),
                            is_signup: false,
                            field_focus: 0,
                            error: Some(msg),
                        };
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        *auth_rx = None;
                        *phase = LoginPhase::EmailInput {
                            email: String::new(),
                            password: String::new(),
                            is_signup: false,
                            field_focus: 0,
                            error: Some("auth connection lost".into()),
                        };
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
        }
    }

    fn tick_onboarding(&mut self) {
        onboarding::poll_handle_validation(&mut self.state);
    }

    fn tick_solving_runtime(&mut self, current_tick: u64, debug_msgs: &mut Vec<String>) {
        if let AppState::Solving {
            running,
            results,
            coach,
            editor_state,
            test_panel,
            problem,
            ..
        } = &mut self.state
        {
            if let Some(rx) = running {
                loop {
                    match rx.try_recv() {
                        Ok(result) => {
                            let vec = results.get_or_insert_with(Vec::new);
                            vec.push(result);
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            let was_runall = test_panel
                                .as_ref()
                                .is_some_and(|tp| tp.run_progress.is_some());
                            if let Some(ref mut tp) = test_panel {
                                tp.run_progress = None;
                            }
                            if was_runall {
                                if let Some(ref r) = results {
                                    if let Some(first_fail) = r.iter().find(|tr| !tr.passed) {
                                        let idx = first_fail.test_num.saturating_sub(1);
                                        if let Some(ex) = problem.examples.get(idx) {
                                            if let Some(ref mut tp) = test_panel {
                                                tp.set_input(&ex.input);
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(ref r) = results {
                                if let Some(ref mut coach_state) = coach {
                                    let passed = r.iter().filter(|tr| tr.passed).count();
                                    let total = r.len();
                                    if passed == total {
                                        // All pass — instant local message, no LLM
                                        coach_state.panel_lines = vec![crate::coach::CoachLine {
                                            text: "all tests pass — /submit when ready".to_string(),
                                            is_header: false,
                                        }];
                                        debug_msgs.push("coach: all tests pass (local)".into());
                                    } else if coach_state.intervention_engine.on_test_failure() {
                                        // Debounced: one analyze per /run with rich context
                                        let mut summary = format!("test run: {}/{} passed.", passed, total);
                                        for tr in r.iter().filter(|tr| !tr.passed) {
                                            summary.push_str(&format!(
                                                " test {}: expected '{}', got '{}'.",
                                                tr.test_num,
                                                tr.expected.lines().next().unwrap_or(""),
                                                tr.actual.lines().next().unwrap_or(""),
                                            ));
                                        }
                                        let code = editor_state.lines.to_string();
                                        let elapsed = coach_state.started_at.elapsed().as_secs();
                                        if let Some(dbg) = coach_state.send_request(
                                            crate::coach::CoachRequest::Analyze {
                                                code,
                                                trigger: summary,
                                                elapsed_secs: elapsed,
                                            },
                                        ) {
                                            debug_msgs.push(dbg);
                                        }
                                    }
                                }
                            }
                            *running = None;
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            if let Some(ref mut tp) = test_panel {
                                if let Some(ref r) = results {
                                    if let Some((ref mut done, _)) = tp.run_progress {
                                        *done = r.len();
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }

            if let Some(ref mut coach_state) = coach {
                match coach_state.event_rx.try_recv() {
                    Ok(crate::coach::CoachEvent::CoachMessage { response }) => {
                        debug_msgs.push(format!(
                            "coach response: state={} conf={:.2} obs={}",
                            response.state,
                            response.confidence,
                            response.matched_observation_id.as_deref().unwrap_or("-"),
                        ));
                        coach_state.apply_response(&response, current_tick);
                    }
                    Ok(crate::coach::CoachEvent::Error { message }) => {
                        debug_msgs.push(format!("coach error: {}", message));
                        coach_state.apply_error(&message);
                    }
                    Ok(crate::coach::CoachEvent::Debug { message }) => {
                        debug_msgs.push(message);
                    }
                    Err(_) => {}
                }

                let line_count = editor_state.lines.iter_row().count();
                if line_count != coach_state.last_line_count {
                    coach_state.intervention_engine.record_edit(line_count);
                    coach_state.last_line_count = line_count;
                    coach_state.dismiss_ghost_text();
                }

                if current_tick.wrapping_sub(coach_state.last_snapshot_tick) >= 50 {
                    let code = editor_state.lines.to_string();
                    coach_state.intervention_engine.update_snapshot(&code);
                    coach_state.last_snapshot_tick = current_tick;
                }

                // Stall → local ghost nudge, no LLM call
                if !coach_state.thinking {
                    if let Some(trigger) = coach_state.intervention_engine.check_for_triggers() {
                        debug_msgs.push(format!(
                            "stall nudge (local): {}",
                            trigger.description()
                        ));
                        coach_state.ghost_text = Some(crate::coach::GhostTextState {
                            text: "stuck? try /hint or /coach <question>".to_string(),
                            format: myro_coach::types::GhostFormat::Natural,
                            appeared_at: current_tick,
                        });
                    }
                }
            }
        }
    }

    fn tick_intense_timer(&mut self, debug_msgs: &mut Vec<String>) {
        if self.recommender.skill_popup_open() {
            return;
        }

        if let AppState::Solving {
            mode: SolveMode::Intense,
            timer_started: Some(started),
            timer_paused_secs,
            timer_expired,
            problem,
            editor_state,
            solution_path,
            ..
        } = &mut self.state
        {
            let effective_elapsed = started
                .elapsed()
                .as_secs()
                .saturating_sub(*timer_paused_secs);
            if !*timer_expired && effective_elapsed >= INTENSE_TIMER_SECS {
                *timer_expired = true;
                let text = editor_state.lines.to_string();
                solving::save_solution(solution_path.as_path(), &text);

                let key = format!("{}:{}", problem.contest_id, problem.index);
                solving::mark_outcome(
                    &mut self.past_entries,
                    problem.contest_id,
                    &problem.index,
                    "timed_out",
                    Some(effective_elapsed),
                );
                let _ = state::save_past(&self.past_entries);

                self.recommender
                    .send(crate::recommend::RecommendRequest::RecordAndRefit {
                        problem_key: key,
                        solved: false,
                    });
                debug_msgs.push("intense timer expired".into());
                self.set_status("time's up!");
            }
        }
    }

    fn tick_recommender(&mut self, current_tick: u64, debug_msgs: &mut Vec<String>) {
        for event in self.recommender.take_events() {
            self.handle_recommender_event(event, current_tick, debug_msgs);
        }
    }

    fn handle_recommender_event(
        &mut self,
        event: crate::recommend::RecommendEvent,
        current_tick: u64,
        debug_msgs: &mut Vec<String>,
    ) {
        match event {
            crate::recommend::RecommendEvent::EmbeddingReady {
                num_observations,
                user_rating,
            } => {
                debug_msgs.push(format!(
                    "embedding ready: {} obs, rating={:?}",
                    num_observations, user_rating
                ));
                if self.recommender.pending_problem.is_none()
                    && !self.recommender.skip_auto_recommend
                {
                    self.recommender.status = Some(format!(
                        "fitted on {} observations. picking problem...",
                        num_observations
                    ));
                    self.recommender
                        .send(crate::recommend::RecommendRequest::Recommend {
                            target_p: self.app_config.recommender.target_probability,
                            solved_keys: self.user_state.solved.clone(),
                        });
                } else {
                    self.recommender.skip_auto_recommend = false;
                    self.recommender.status = None;
                }
            }
            crate::recommend::RecommendEvent::ProblemRecommended {
                contest_id,
                index,
                predicted_p,
                rating,
                ..
            } => {
                debug_msgs.push(format!(
                    "recommended: {}{} P={:.2} rating={:?}",
                    contest_id, index, predicted_p, rating
                ));
                self.recommender.cache_pending_problem(
                    &mut self.user_state,
                    contest_id,
                    index.clone(),
                    predicted_p,
                    rating,
                );
                if self.last_solve_mode.is_some() {
                    self.recommender.status =
                        Some(format!("fetching problem {}{}...", contest_id, index));
                    self.recommender
                        .send(crate::recommend::RecommendRequest::FetchProblem {
                            contest_id,
                            index,
                        });
                } else {
                    self.recommender.status = None;
                }
            }
            crate::recommend::RecommendEvent::ProblemFetched { statement } => {
                self.recommender.status = None;
                if self.last_solve_mode.is_some() {
                    if self.recommender.skill_popup_open() {
                        self.recommender.deferred_problem = Some(statement);
                    } else {
                        self.start_solving_recommended(statement);
                    }
                }
            }
            crate::recommend::RecommendEvent::Submitted => {
                self.recommender.status = Some("submitted! waiting for verdict...".into());
            }
            crate::recommend::RecommendEvent::Verdict { verdict, .. } => {
                self.recommender.status = None;
                self.handle_verdict(&verdict);
            }
            crate::recommend::RecommendEvent::Refitted => {
                debug_msgs.push("embedding refitted".into());
                self.recommender.status = None;
            }
            crate::recommend::RecommendEvent::SkillProfile { profile, deltas } => {
                debug_msgs.push(format!(
                    "skill profile: overall={}, {} tags",
                    profile.overall_rating,
                    profile.tag_ratings.len(),
                ));
                self.recommender.status = None;
                self.recommender.store_skill_profile(profile);
                if self.last_solve_mode == Some(SolveMode::Intense) && !deltas.is_empty() {
                    self.recommender.skill_deltas = Some(deltas);
                    self.recommender.skill_delta_tick = current_tick;
                    self.timer_pause_start = Some(Instant::now());
                }
            }
            crate::recommend::RecommendEvent::Error { message } => {
                self.set_status(format!("error: {}", message));
                self.recommender.status = None;
            }
            crate::recommend::RecommendEvent::Status { message } => {
                self.recommender.status = Some(message);
            }
        }
    }

    fn tick_verdict_polling(&mut self, current_tick: u64) {
        if self.recommender.status.as_deref() == Some("submitted! waiting for verdict...")
            && current_tick.is_multiple_of(30)
        {
            if let AppState::Solving { problem, .. } = &self.state {
                self.recommender
                    .send(crate::recommend::RecommendRequest::PollVerdict {
                        contest_id: problem.contest_id,
                    });
            }
        }
    }

    fn flush_debug_messages(&mut self, debug_msgs: Vec<String>) {
        for msg in debug_msgs {
            self.log_debug(msg);
        }
    }
}
