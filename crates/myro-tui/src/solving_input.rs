use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use edtui::EditorMode;

use crate::app::{App, AppState, ConfirmPopup, SolveMode};
use crate::solving;
impl App {
    pub fn handle_solving_key(&mut self, key: KeyEvent) {
        if self.solving_input_blocked() {
            return;
        }

        if self.solving_in_command_mode() {
            self.handle_command_input(key);
            return;
        }

        if self.handle_test_panel_key(key) {
            return;
        }

        if self.solving_editor_is_normal() && key.code == KeyCode::Char('/') {
            self.open_solving_command_input();
            self.status_message = None;
            return;
        }

        if self.solving_editor_is_normal() && key.code == KeyCode::Tab {
            if let AppState::Solving {
                statement_focused, ..
            } = &mut self.state
            {
                *statement_focused = !*statement_focused;
            }
            return;
        }

        if self.handle_statement_key(key) {
            return;
        }

        self.dismiss_solving_ghost_text();
        self.forward_key_to_editor(key);
    }

    fn handle_command_input(&mut self, key: KeyEvent) {
        let Some(command) = self.update_command_input(key) else {
            return;
        };
        self.execute_command(&command);
    }

    fn execute_command(&mut self, cmd: &str) {
        if self.handle_debug_command(cmd) {
            return;
        }

        if let Some(message) = cmd.strip_prefix("coach ") {
            self.send_coach_user_message(message);
            return;
        }

        if cmd == "test" {
            self.toggle_test_panel();
            return;
        }

        if self.solving_test_panel_open() && self.handle_test_panel_command(cmd) {
            return;
        }

        let (current_mode, is_from_past) = self.current_solving_context();

        match cmd {
            "run" => self.run_all_tests(),
            "quit" | "q" => self.execute_quit_command(current_mode, is_from_past),
            "help" | "h" => {}
            "skip" => self.execute_skip_command(current_mode, is_from_past),
            "hint" => self.request_hint(current_mode),
            "coach" => self.toggle_coach_panel(current_mode),
            "submit" => self.submit_current_solution(),
            "isuck" => self.execute_isuck_command(current_mode),
            _ => {}
        }
    }

    fn solving_input_blocked(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving {
                running: Some(_), ..
            }
        ) || matches!(
            &self.state,
            AppState::Solving {
                timer_expired: true,
                ..
            }
        )
    }

    fn solving_in_command_mode(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving {
                command_input: Some(_),
                ..
            }
        )
    }

    fn solving_editor_is_normal(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving { editor_state, .. }
                if matches!(editor_state.mode, EditorMode::Normal)
        )
    }

    fn solving_test_panel_open(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving {
                test_panel: Some(ref tp),
                ..
            } if tp.visible
        )
    }

    fn solving_test_input_is_normal(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving {
                test_panel: Some(ref tp),
                ..
            } if tp.visible && matches!(tp.input_state.mode, EditorMode::Normal)
        )
    }

    fn solving_test_output_focused(&self) -> bool {
        matches!(
            &self.state,
            AppState::Solving {
                test_panel: Some(ref tp),
                ..
            } if tp.output_focused
        )
    }

    fn open_solving_command_input(&mut self) {
        if let AppState::Solving { command_input, .. } = &mut self.state {
            *command_input = Some(String::new());
        }
    }

    fn handle_test_panel_key(&mut self, key: KeyEvent) -> bool {
        if !self.solving_test_panel_open() {
            return false;
        }

        let normal_input = self.solving_test_input_is_normal();

        if key.code == KeyCode::Esc && normal_input {
            if let AppState::Solving {
                test_panel: Some(ref mut tp),
                ..
            } = &mut self.state
            {
                tp.visible = false;
            }
            return true;
        }

        if key.code == KeyCode::Char('/') && normal_input {
            self.open_solving_command_input();
            return true;
        }

        if key.code == KeyCode::Tab && normal_input {
            if let AppState::Solving {
                test_panel: Some(ref mut tp),
                ..
            } = &mut self.state
            {
                tp.output_focused = !tp.output_focused;
            }
            return true;
        }

        if self.solving_test_output_focused() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if let AppState::Solving {
                        test_panel: Some(ref mut tp),
                        ..
                    } = &mut self.state
                    {
                        tp.output_scroll = tp.output_scroll.saturating_add(1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let AppState::Solving {
                        test_panel: Some(ref mut tp),
                        ..
                    } = &mut self.state
                    {
                        tp.output_scroll = tp.output_scroll.saturating_sub(1);
                    }
                }
                _ => {}
            }
            return true;
        }

        if let AppState::Solving {
            test_panel: Some(ref mut tp),
            ..
        } = &mut self.state
        {
            tp.input_handler.on_key_event(key, &mut tp.input_state);
        }
        true
    }

    fn handle_statement_key(&mut self, key: KeyEvent) -> bool {
        let focused = matches!(
            &self.state,
            AppState::Solving {
                statement_focused: true,
                ..
            }
        );
        if !focused {
            return false;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::Solving { statement_scroll, .. } = &mut self.state {
                    *statement_scroll = statement_scroll.saturating_add(1);
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::Solving { statement_scroll, .. } = &mut self.state {
                    *statement_scroll = statement_scroll.saturating_sub(1);
                }
                true
            }
            KeyCode::Esc | KeyCode::Tab => {
                if let AppState::Solving {
                    statement_focused, ..
                } = &mut self.state
                {
                    *statement_focused = false;
                }
                true
            }
            _ => {
                if let AppState::Solving {
                    statement_focused, ..
                } = &mut self.state
                {
                    *statement_focused = false;
                }
                false
            }
        }
    }

    fn dismiss_solving_ghost_text(&mut self) {
        if let AppState::Solving {
            coach: Some(ref mut coach_state),
            ..
        } = &mut self.state
        {
            coach_state.dismiss_ghost_text();
        }
    }

    fn forward_key_to_editor(&mut self, key: KeyEvent) {
        if let AppState::Solving {
            editor_state,
            editor_handler,
            ..
        } = &mut self.state
        {
            editor_handler.on_key_event(key, editor_state);
        }
    }

    fn update_command_input(&mut self, key: KeyEvent) -> Option<String> {
        let AppState::Solving { command_input, .. } = &mut self.state else {
            return None;
        };

        let command = command_input.as_mut()?;
        match key.code {
            KeyCode::Esc => {
                let _ = command_input.take();
                None
            }
            KeyCode::Enter => {
                let finished = command.clone();
                let _ = command_input.take();
                Some(finished)
            }
            KeyCode::Backspace => {
                command.pop();
                if command.is_empty() {
                    let _ = command_input.take();
                }
                None
            }
            KeyCode::Char(ch) => {
                command.push(ch);
                None
            }
            _ => None,
        }
    }

    fn handle_debug_command(&mut self, cmd: &str) -> bool {
        if cmd == "debug copy" {
            let log_text = self.debug_log.join("\n");
            match copy_to_clipboard(&log_text) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("copied {} log lines to clipboard", self.debug_log.len()));
                }
                Err(e) => {
                    let path = dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("myro")
                        .join("debug.log");
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    let _ = std::fs::write(&path, &log_text);
                    self.status_message =
                        Some(format!("clipboard failed ({}), saved to {}", e, path.display()));
                }
            }
            return true;
        }

        if cmd == "debug" {
            self.debug_visible = !self.debug_visible;
            self.debug_scroll = 0;
            return true;
        }

        false
    }

    fn current_solving_context(&self) -> (Option<SolveMode>, bool) {
        match &self.state {
            AppState::Solving {
                mode,
                from_past,
                ..
            } => (Some(*mode), *from_past),
            _ => (None, false),
        }
    }

    fn send_coach_user_message(&mut self, message: &str) {
        let (current_mode, _) = self.current_solving_context();
        if current_mode == Some(SolveMode::Intense) {
            self.set_status("coach not available in intense mode");
            return;
        }

        if let AppState::Solving {
            coach: Some(ref mut coach_state),
            editor_state,
            ..
        } = &mut self.state
        {
            let code = editor_state.lines.to_string();
            let elapsed = coach_state.started_at.elapsed().as_secs();
            if let Some(debug) = coach_state.send_request(crate::coach::CoachRequest::UserMessage {
                message: message.to_string(),
                code,
                elapsed_secs: elapsed,
            }) {
                self.debug_log
                    .push(format!("[{:>4}s] {}", self.tick / 10, debug));
            }
        }
    }

    fn toggle_test_panel(&mut self) {
        if let AppState::Solving { test_panel, .. } = &mut self.state {
            match test_panel {
                Some(tp) => tp.visible = !tp.visible,
                None => *test_panel = Some(Box::new(crate::test_panel::TestPanelState::new())),
            }
        }
    }

    fn handle_test_panel_command(&mut self, cmd: &str) -> bool {
        if cmd == "run" {
            self.run_custom_input();
            return true;
        }

        if cmd == "runall" {
            self.run_all_tests();
            return true;
        }

        if let Some(sample_index) = parse_sample_command(cmd) {
            let sample_error = if let AppState::Solving {
                problem,
                test_panel: Some(ref mut tp),
                ..
            } = &mut self.state
            {
                if let Some(example) = problem.examples.get(sample_index) {
                    tp.set_input(&example.input);
                    tp.output_focused = false;
                    None
                } else {
                    Some(format!(
                        "No sample {} (have {})",
                        sample_index + 1,
                        problem.examples.len()
                    ))
                }
            } else {
                None
            };

            if let Some(message) = sample_error {
                self.set_status(message);
            }
            return true;
        }

        false
    }

    fn execute_quit_command(&mut self, current_mode: Option<SolveMode>, is_from_past: bool) {
        if current_mode == Some(SolveMode::Intense) {
            self.set_status("can't quit intense mode. use /isuck to give up.");
            return;
        }

        self.save_and_finish_past("skipped");
        if let AppState::Solving {
            coach: Some(ref coach_state),
            ..
        } = &self.state
        {
            let _ = coach_state.request_tx.send(crate::coach::CoachRequest::Quit);
        }

        if self.ephemeral {
            self.should_quit = true;
            return;
        }

        self.state = if is_from_past {
            AppState::Past {
                scroll: 0,
                command_input: None,
                filter_open: false,
                filter_cursor: 0,
                order_open: false,
                order_cursor: 0,
            }
        } else {
            AppState::Home { selected: 0 }
        };
    }

    fn execute_skip_command(&mut self, current_mode: Option<SolveMode>, is_from_past: bool) {
        if current_mode == Some(SolveMode::Intense) {
            self.set_status("/skip is Chill-only. Use /isuck in Intense mode.");
            return;
        }
        if is_from_past {
            self.set_status("/skip not available for past problems. Use /quit.");
            return;
        }
        self.execute_skip();
    }

    fn request_hint(&mut self, current_mode: Option<SolveMode>) {
        if current_mode == Some(SolveMode::Intense) {
            self.set_status("hints not available in intense mode");
            return;
        }

        if let AppState::Solving {
            coach: Some(ref mut coach_state),
            editor_state,
            ..
        } = &mut self.state
        {
            coach_state.intervention_engine.on_user_request();
            let hint_count = coach_state.hint_count;
            let code = editor_state.lines.to_string();
            let elapsed = coach_state.started_at.elapsed().as_secs();
            if let Some(debug) = coach_state.send_request(crate::coach::CoachRequest::RequestHint {
                code,
                elapsed_secs: elapsed,
                hint_count: coach_state.hint_count,
            }) {
                self.debug_log
                    .push(format!("[{:>4}s] {}", self.tick / 10, debug));
            }
            coach_state.confidence = myro_coach::types::ConfidenceLevel::Intervening;
            self.track_event("hint_requested", serde_json::json!({
                "hint_count": hint_count,
            }));
        } else {
            self.set_status("coach not available — set [coach] base_url in config");
        }
    }

    fn toggle_coach_panel(&mut self, current_mode: Option<SolveMode>) {
        if current_mode == Some(SolveMode::Intense) {
            self.set_status("coach not available in intense mode");
            return;
        }

        if let AppState::Solving {
            coach: Some(ref mut coach_state),
            ..
        } = &mut self.state
        {
            coach_state.panel_visible = !coach_state.panel_visible;
        } else {
            self.set_status("coach not available — set [coach] base_url in config");
        }
    }

    fn submit_current_solution(&mut self) {
        if let AppState::Solving {
            editor_state,
            problem,
            ..
        } = &self.state
        {
            let handle = match &self.app_config.codeforces.handle {
                Some(handle) => handle.clone(),
                None => {
                    self.set_status("set cf handle in settings first");
                    return;
                }
            };

            let cookies = self.app_config.codeforces.cookies.clone();
            if cookies.is_empty() {
                self.set_status("no cf session. log into cf in firefox and re-import in settings.");
                return;
            }

            let user_agent = self
                .app_config
                .codeforces
                .user_agent
                .clone()
                .unwrap_or_else(crate::browser::detect_firefox_ua);

            self.track_event("submission_sent", serde_json::json!({
                "contest_id": problem.contest_id,
                "index": &problem.index,
            }));
            self.recommender.send(crate::recommend::RecommendRequest::Submit {
                contest_id: problem.contest_id,
                index: problem.index.clone(),
                source_code: editor_state.lines.to_string(),
                handle,
                cookies,
                user_agent,
            });
            self.recommender.status = Some("submitting...".into());
        }
    }

    fn execute_isuck_command(&mut self, current_mode: Option<SolveMode>) {
        if current_mode == Some(SolveMode::Chill) {
            self.set_status("/isuck is intense-only. use /skip in chill mode.");
            return;
        }

        if !self.user_state.isuck_explained {
            self.confirm_popup = Some(ConfirmPopup {
                title: "give up",
                message: "/isuck marks this problem as too hard. your ratings will update.",
                action: "isuck",
            });
            return;
        }

        self.execute_isuck();
    }

    fn run_custom_input(&mut self) {
        let input = match &self.state {
            AppState::Solving {
                test_panel: Some(ref tp),
                ..
            } => tp.get_input(),
            _ => return,
        };

        if let AppState::Solving {
            editor_state,
            results,
            running,
            solution_path,
            test_panel,
            ..
        } = &mut self.state
        {
            let text = editor_state.lines.to_string();
            solving::save_solution(solution_path.as_path(), &text);
            *results = None;
            if let Some(tp) = test_panel {
                tp.run_progress = None;
                tp.output_scroll = 0;
            }

            let path = solution_path.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for result in crate::runner::run_custom(&path, &input) {
                    let _ = tx.send(result);
                }
            });
            *running = Some(rx);
        }
    }

    fn run_all_tests(&mut self) {
        if let AppState::Solving {
            problem,
            editor_state,
            results,
            running,
            solution_path,
            test_panel,
            ..
        } = &mut self.state
        {
            let text = editor_state.lines.to_string();
            solving::save_solution(solution_path.as_path(), &text);

            *results = None;
            let total = problem.examples.len();
            if let Some(tp) = test_panel {
                tp.run_progress = Some((0, total));
                tp.output_scroll = 0;
            }

            let path = solution_path.clone();
            let examples = problem.examples.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                crate::runner::run_tests_incremental(&path, &examples, "python3", &tx);
            });
            *running = Some(rx);
        }
    }
}

fn parse_sample_command(cmd: &str) -> Option<usize> {
    let n_str = cmd.strip_prefix("sample")?.trim();
    let n = n_str.parse::<usize>().ok()?;
    n.checked_sub(1)
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use base64::Engine;
    use std::io::Write;
    use std::process::{Command, Stdio};

    let commands: &[&[&str]] = &[
        &["wl-copy"],
        &["xclip", "-selection", "clipboard"],
        &["xsel", "--clipboard", "--input"],
    ];

    for args in commands {
        if let Ok(mut child) = Command::new(args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    return Ok(());
                }
            }
        }
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(text);
    let osc = format!("\x1b]52;c;{}\x07", b64);
    let mut stdout = std::io::stdout();
    if crossterm::execute!(stdout, crossterm::style::Print(&osc)).is_ok() {
        return Ok(());
    }

    Err("no clipboard tool available".to_string())
}
