use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorEventHandler, EditorState};
use myro_cf::types::ProblemStatement;
use myro_coach::seed::ProblemFile;
use std::sync::mpsc;
use std::time::Instant;

use crate::onboarding;
use crate::recommender_state::RecommenderState;
use crate::runner::TestResult;
use crate::solving;
use crate::state::{self, PastEntry, UserState};

pub use myro_api::{EventBatch, SupabaseClient};

const CHECK: &str = "\u{2713}";
const CROSS: &str = "\u{2717}";

/// Intense mode timer duration in seconds.
pub const INTENSE_TIMER_SECS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveMode {
    Chill,
    Intense,
}

pub struct ConfirmPopup {
    pub title: &'static str,
    pub message: &'static str,
    pub action: &'static str,
}

#[derive(Debug, Clone)]
pub struct PastFilter {
    pub chill: bool,
    pub intense: bool,
    pub solved: bool,
    pub unsolved: bool,
    pub submitted: bool,
    pub unsubmitted: bool,
}

impl Default for PastFilter {
    fn default() -> Self {
        Self {
            chill: true,
            intense: true,
            solved: true,
            unsolved: true,
            submitted: true,
            unsubmitted: true,
        }
    }
}

impl PastFilter {
    pub fn matches(&self, entry: &PastEntry) -> bool {
        let mode_ok = match entry.mode.as_str() {
            "chill" => self.chill,
            "intense" => self.intense,
            _ => true,
        };
        let verdict_ok = if entry.ever_accepted { self.solved } else { self.unsolved };
        let sub_ok = if entry.ever_submitted { self.submitted } else { self.unsubmitted };
        mode_ok && verdict_ok && sub_ok
    }

    pub fn label(index: usize) -> &'static str {
        match index {
            0 => "chill",
            1 => "intense",
            2 => "solved",
            3 => "unsolved",
            4 => "submitted",
            5 => "unsubmitted",
            _ => "",
        }
    }

    pub fn section_header(index: usize) -> Option<&'static str> {
        match index {
            0 => Some("mode"),
            2 => Some("status"),
            4 => Some("submission"),
            _ => None,
        }
    }

    pub fn get(&self, index: usize) -> bool {
        match index {
            0 => self.chill,
            1 => self.intense,
            2 => self.solved,
            3 => self.unsolved,
            4 => self.submitted,
            5 => self.unsubmitted,
            _ => true,
        }
    }

    pub fn toggle(&mut self, index: usize) {
        match index {
            0 => self.chill = !self.chill,
            1 => self.intense = !self.intense,
            2 => self.solved = !self.solved,
            3 => self.unsolved = !self.unsolved,
            4 => self.submitted = !self.submitted,
            5 => self.unsubmitted = !self.unsubmitted,
            _ => {}
        }
    }

    pub const COUNT: usize = 6;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderSortBy {
    #[default]
    FirstSeen,
    LastSeen,
    FirstSubmission,
    LastSubmission,
    Rating,
}

impl OrderSortBy {
    pub const ALL: &[OrderSortBy] = &[
        OrderSortBy::FirstSeen,
        OrderSortBy::LastSeen,
        OrderSortBy::FirstSubmission,
        OrderSortBy::LastSubmission,
        OrderSortBy::Rating,
    ];

    pub fn label(self) -> &'static str {
        match self {
            OrderSortBy::FirstSeen => "first seen",
            OrderSortBy::LastSeen => "last seen",
            OrderSortBy::FirstSubmission => "first submission",
            OrderSortBy::LastSubmission => "last submission",
            OrderSortBy::Rating => "rating",
        }
    }

    pub fn is_time_based(self) -> bool {
        !matches!(self, OrderSortBy::Rating)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PastOrder {
    pub sort_by: OrderSortBy,
    pub ascending: bool,
}

impl PastOrder {
    /// Total items: 5 sort keys + 2 direction options
    pub const TOTAL_ITEMS: usize = 7;

    pub fn direction_labels(&self) -> (&'static str, &'static str) {
        if self.sort_by.is_time_based() {
            ("most recent first", "oldest first")
        } else {
            ("highest first", "lowest first")
        }
    }
}

pub struct App {
    pub state: AppState,
    pub should_quit: bool,
    pub quit_pending: bool,
    pub status_message: Option<String>,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub tick: u64,
    pub coach_config: myro_coach::config::CoachConfig,
    pub app_config: crate::config::AppConfig,
    pub user_state: UserState,
    pub recommender: RecommenderState,
    pub debug_log: Vec<String>,
    pub debug_visible: bool,
    pub debug_scroll: usize,
    pub confirm_popup: Option<ConfirmPopup>,
    pub(crate) status_clear_tick: u64,
    pub ephemeral: bool,
    // --- Chill/Intense mode fields ---
    pub feed_me_menu: Option<usize>,
    pub last_solve_mode: Option<SolveMode>,
    pub past_entries: Vec<PastEntry>,
    pub past_filter: PastFilter,
    pub past_order: PastOrder,
    /// Index into past_entries of a problem being re-opened from My Past.
    /// When set, start_solving_recommended skips creating a new PastEntry.
    pub reopening_past_entry: Option<usize>,
    /// Instant when timer was paused (skill deltas popup open).
    pub timer_pause_start: Option<Instant>,
    // --- Supabase sync ---
    pub api: Option<SupabaseClient>,
    pub events: Option<std::sync::Arc<EventBatch>>,
    // --- Self-update ---
    pub update_rx: Option<mpsc::Receiver<crate::updater::UpdateEvent>>,
    pub update_available: Option<String>,
}

pub enum OnboardingPhase {
    Handle,
    CookieImport,
}

#[derive(Debug)]
pub enum LoginPhase {
    ChooseMethod,
    EmailInput {
        email: String,
        password: String,
        is_signup: bool,
        field_focus: u8, // 0 = email, 1 = password
        error: Option<String>,
    },
    OAuthWaiting,
    OAuthSuccess,
}

pub enum AppState {
    Login {
        phase: LoginPhase,
        selected: usize,
        auth_rx: Option<mpsc::Receiver<Result<myro_api::AuthTokens, String>>>,
    },
    HandlePrompt {
        phase: OnboardingPhase,
        handle_input: String,
        error: Option<String>,
        validating: bool,
        validate_rx: Option<mpsc::Receiver<Result<myro_cf::CfUser, String>>>,
    },
    Home {
        selected: usize,
    },
    // TODO: bring back when "Start training" menu item returns
    #[allow(dead_code)]
    ProblemSelect {
        problems: Vec<ProblemFile>,
        selected: usize,
        scroll_offset: usize,
    },
    Stats {
        scroll: usize,
    },
    Settings {
        selected: usize,
        editing: Option<String>,
    },
    Past {
        scroll: usize,
        command_input: Option<String>,
        filter_open: bool,
        filter_cursor: usize,
        order_open: bool,
        order_cursor: usize,
    },
    Solving {
        problem: Box<ProblemStatement>,
        #[allow(dead_code)]
        problem_file: Box<ProblemFile>,
        editor_state: Box<EditorState>,
        editor_handler: EditorEventHandler,
        results: Option<Vec<TestResult>>,
        running: Option<mpsc::Receiver<TestResult>>,
        show_statement: bool,
        statement_scroll: u16,
        statement_focused: bool,
        command_input: Option<String>,
        solution_path: std::path::PathBuf,
        coach: Option<Box<crate::coach::CoachState>>,
        test_panel: Option<Box<crate::test_panel::TestPanelState>>,
        mode: SolveMode,
        timer_started: Option<Instant>,
        timer_paused_secs: u64,
        timer_expired: bool,
        from_past: bool,
    },
}

pub const MENU_ITEMS: &[&str] = &[
    "feed me",
    "rate me",
    "my past",
    "settings",
];

pub enum SettingsItem {
    Section { label: &'static str },
    Display { label: &'static str, field: &'static str },
    Editable { label: &'static str, field: &'static str },
    EditableSensitive { label: &'static str, field: &'static str },
    Action { label: &'static str, action: &'static str },
}

impl SettingsItem {
    pub fn is_selectable(&self) -> bool {
        !matches!(self, SettingsItem::Section { .. })
    }
}

pub const SETTINGS_ITEMS: &[SettingsItem] = &[
    // LLM / Coach
    SettingsItem::Section { label: "ai coach" },
    SettingsItem::Editable { label: "llm endpoint", field: "coach.base_url" },
    SettingsItem::EditableSensitive { label: "api key", field: "coach.api_key" },
    SettingsItem::Editable { label: "model", field: "coach.model" },
    // Codeforces
    SettingsItem::Section { label: "codeforces" },
    SettingsItem::Display { label: "cf handle", field: "codeforces.handle" },
    SettingsItem::Editable { label: "target p(solve)", field: "recommender.target_probability" },
    SettingsItem::Action { label: "re-import cookies", action: "reimport_cookies" },
    // Account
    SettingsItem::Section { label: "account" },
    SettingsItem::Action { label: "sign in (sync)", action: "sign_in" },
    SettingsItem::Action { label: "reset history", action: "reset_history" },
    SettingsItem::Action { label: "logout", action: "logout" },
];

impl App {
    pub fn new() -> Result<Self> {
        let coach_config = myro_coach::config::CoachConfig::load().unwrap_or_default();
        let app_config = crate::config::AppConfig::load();
        let user_state = state::load_state();
        let mut recommender = RecommenderState::new(&user_state);

        // Try to restore auth session
        let (api, events) = match myro_api::auth::load_tokens() {
            Some(tokens) => {
                if tokens.is_expired() {
                    match myro_api::auth::refresh_token(&tokens) {
                        Ok(refreshed) => {
                            let client = refreshed.to_client();
                            let events =
                                std::sync::Arc::new(EventBatch::new(client.clone()));
                            (Some(client), Some(events))
                        }
                        Err(_) => (None, None),
                    }
                } else {
                    let client = tokens.to_client();
                    let events = std::sync::Arc::new(EventBatch::new(client.clone()));
                    (Some(client), Some(events))
                }
            }
            None => (None, None),
        };

        // Login is optional — go straight to normal onboarding
        let initial_state = onboarding::initial_app_state(&app_config);

        // Pre-load inference if user is set up (handle + cookies present)
        if matches!(initial_state, AppState::Home { .. }) {
            if let Some(handle) = &app_config.codeforces.handle {
                recommender.send(crate::recommend::RecommendRequest::FetchAndFit {
                    handle: handle.clone(),
                    model_path: app_config.recommender.model_path.clone(),
                });
            }
        }

        // Track session start
        if let Some(ref ev) = events {
            ev.track("session_start", serde_json::json!({}));
        }

        let past_entries = state::load_past();
        let update_rx = crate::updater::spawn_update_check(&app_config.update);

        Ok(Self {
            state: initial_state,
            should_quit: false,
            quit_pending: false,
            status_message: None,
            terminal_width: 80,
            terminal_height: 24,
            tick: 0,
            coach_config,
            app_config,
            user_state,
            recommender,
            debug_log: Vec::new(),
            debug_visible: false,
            debug_scroll: 0,
            confirm_popup: None,
            status_clear_tick: 0,
            ephemeral: false,
            feed_me_menu: None,
            last_solve_mode: None,
            past_entries,
            past_filter: PastFilter::default(),
            past_order: PastOrder::default(),
            reopening_past_entry: None,
            timer_pause_start: None,
            api,
            events,
            update_rx,
            update_available: None,
        })
    }

    /// Create an ephemeral app that goes straight to Solving with a mock coach.
    /// No filesystem state is read or written — ideal for testing coach interactions.
    pub fn new_ephemeral() -> Result<Self> {
        let problem_set_dir = myro_coach::seed::default_problem_set_dir();
        let problem_file = myro_coach::seed::load_problem_file(
            &problem_set_dir.join("cf-112A.json"),
        )
        .unwrap_or_else(|_| solving::test_problem_file());

        let problem = solving::problem_file_to_statement(&problem_file);

        let mut coach_config = myro_coach::config::CoachConfig::load().unwrap_or_default();
        coach_config.mock = true;

        let (state, _) = solving::create_solving_state(
            problem,
            problem_file,
            String::new(),
            &coach_config,
            "ephemeral",
            SolveMode::Chill,
            false,
        );

        Ok(Self {
            state,
            should_quit: false,
            quit_pending: false,
            status_message: None,
            terminal_width: 80,
            terminal_height: 24,
            tick: 0,
            coach_config,
            app_config: crate::config::AppConfig::default(),
            user_state: UserState::default(),
            recommender: RecommenderState::empty(),
            debug_log: Vec::new(),
            debug_visible: false,
            debug_scroll: 0,
            confirm_popup: None,
            status_clear_tick: 0,
            ephemeral: true,
            feed_me_menu: None,
            last_solve_mode: None,
            past_entries: Vec::new(),
            past_filter: PastFilter::default(),
            past_order: PastOrder::default(),
            reopening_past_entry: None,
            timer_pause_start: None,
            api: None,
            events: None,
            update_rx: None,
            update_available: None,
        })
    }

    /// Best-effort sync of solved problems and past entries to Supabase.
    pub(crate) fn sync_to_remote(&self) {
        if let Some(ref api) = self.api {
            state::sync_solved_to_remote(api, &self.user_state.solved);
            state::sync_past_to_remote(api, &self.past_entries);
        }
    }

    /// Track an analytics event (no-op if not authenticated).
    pub(crate) fn track_event(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(ref events) = self.events {
            events.track(event_type, payload);
        }
    }

    /// Set a status message and reset the auto-clear timer.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_clear_tick = 0;
    }

    pub fn log_debug(&mut self, msg: impl Into<String>) {
        let elapsed = self.tick / 10; // approximate seconds
        self.debug_log
            .push(format!("[{:>4}s] {}", elapsed, msg.into()));
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl+C: first press warns, second quits
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.quit_pending {
                self.should_quit = true;
            } else {
                self.quit_pending = true;
                self.status_message = Some("press ctrl+c again to quit".to_string());
            }
            return;
        }

        // Any other key clears the quit prompt
        if self.quit_pending {
            self.quit_pending = false;
            self.status_message = None;
        }

        // Confirm popup intercepts all keys
        if self.confirm_popup.is_some() {
            match key.code {
                KeyCode::Enter => {
                    let popup = self.confirm_popup.take().unwrap();
                    self.execute_confirmed_action(popup.action);
                }
                KeyCode::Esc => {
                    self.confirm_popup = None;
                }
                _ => {} // swallow
            }
            return;
        }

        // Skill delta popup: Enter = next problem, Esc = back to menu
        if self.recommender.skill_popup_open() {
            if key.code == KeyCode::Enter {
                // Accumulate paused time before dismissing
                if let Some(pause_start) = self.timer_pause_start.take() {
                    if let AppState::Solving { timer_paused_secs, .. } = &mut self.state {
                        *timer_paused_secs += pause_start.elapsed().as_secs();
                    }
                }
                self.recommender.skill_deltas = None;
                if self.last_solve_mode == Some(SolveMode::Intense) {
                    self.start_suggested_problem(SolveMode::Intense);
                } else if let Some(statement) = self.recommender.deferred_problem.take() {
                    self.start_solving_recommended(statement);
                }
            } else if key.code == KeyCode::Esc {
                self.timer_pause_start = None;
                self.recommender.skill_deltas = None;
                self.state = AppState::Home { selected: 0 };
            }
            return;
        }

        // Debug overlay intercepts keys when visible
        if self.debug_visible {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.debug_visible = false;
                    return;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.debug_scroll = self.debug_scroll.saturating_add(1);
                    return;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.debug_scroll = self.debug_scroll.saturating_sub(1);
                    return;
                }
                KeyCode::Char('G') => {
                    // Jump to bottom
                    self.debug_scroll = 0;
                    return;
                }
                KeyCode::Char('g') => {
                    // Jump to top
                    self.debug_scroll = 1; // 1 = first entry (0 means "pin to bottom")
                    return;
                }
                _ => return, // swallow all other keys
            }
        }

        // Feed me sub-menu popup
        if let Some(sel) = self.feed_me_menu {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.feed_me_menu = Some((sel + 1).min(1));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.feed_me_menu = Some(sel.saturating_sub(1));
                }
                KeyCode::Enter => {
                    let mode = if sel == 0 { SolveMode::Chill } else { SolveMode::Intense };
                    self.feed_me_menu = None;
                    self.start_suggested_problem(mode);
                }
                KeyCode::Esc => {
                    self.feed_me_menu = None;
                }
                _ => {}
            }
            return;
        }

        match &self.state {
            AppState::Login { .. } => self.handle_login_key(key),
            AppState::HandlePrompt { .. } => self.handle_handle_prompt_key(key),
            AppState::Home { .. } => self.handle_home_key(key),
            AppState::Stats { .. } => self.handle_stats_key(key),
            AppState::Settings { .. } => self.handle_settings_key(key),
            AppState::ProblemSelect { .. } => self.handle_problem_select_key(key),
            AppState::Past { .. } => self.handle_past_key(key),
            AppState::Solving { .. } => self.handle_solving_key(key),
        }
    }

    fn handle_login_key(&mut self, key: KeyEvent) {
        let (phase, selected) = match &mut self.state {
            AppState::Login { phase, selected, .. } => (phase, selected),
            _ => return,
        };

        match phase {
            LoginPhase::ChooseMethod => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    *selected = (*selected + 1).min(3);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Enter => match *selected {
                    0 => {
                        // GitHub OAuth
                        self.start_github_oauth();
                    }
                    1 => {
                        // Email sign in
                        *phase = LoginPhase::EmailInput {
                            email: String::new(),
                            password: String::new(),
                            is_signup: false,
                            field_focus: 0,
                            error: None,
                        };
                    }
                    2 => {
                        // Create account
                        *phase = LoginPhase::EmailInput {
                            email: String::new(),
                            password: String::new(),
                            is_signup: true,
                            field_focus: 0,
                            error: None,
                        };
                    }
                    3 => {
                        // Cancel — back to settings
                        self.state = AppState::Settings {
                            selected: 1,
                            editing: None,
                        };
                    }
                    _ => {}
                },
                KeyCode::Esc => {
                    self.state = AppState::Settings {
                        selected: 1,
                        editing: None,
                    };
                }
                _ => {}
            },
            LoginPhase::EmailInput {
                email,
                password,
                is_signup,
                field_focus,
                error,
            } => match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    *field_focus = if *field_focus == 0 { 1 } else { 0 };
                }
                KeyCode::Char(c) => {
                    *error = None;
                    if *field_focus == 0 {
                        email.push(c);
                    } else {
                        password.push(c);
                    }
                }
                KeyCode::Backspace => {
                    *error = None;
                    if *field_focus == 0 {
                        email.pop();
                    } else {
                        password.pop();
                    }
                }
                KeyCode::Enter => {
                    let email_val = email.clone();
                    let password_val = password.clone();
                    let signup = *is_signup;

                    if email_val.is_empty() || password_val.is_empty() {
                        *error = Some("email and password are required".into());
                        return;
                    }

                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let result = if signup {
                            myro_api::auth::sign_up_email(&email_val, &password_val)
                        } else {
                            myro_api::auth::sign_in_email(&email_val, &password_val)
                        };
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    });
                    self.state = AppState::Login {
                        phase: LoginPhase::OAuthWaiting,
                        selected: 0,
                        auth_rx: Some(rx),
                    };
                }
                KeyCode::Esc => {
                    *phase = LoginPhase::ChooseMethod;
                }
                _ => {}
            },
            LoginPhase::OAuthWaiting => {
                if key.code == KeyCode::Esc {
                    *phase = LoginPhase::ChooseMethod;
                }
            }
            LoginPhase::OAuthSuccess => {}
        }
    }

    fn start_github_oauth(&mut self) {
        match myro_api::auth::start_github_oauth() {
            Ok((url, listener)) => {
                // Open browser (suppress stdout/stderr to prevent GTK warnings corrupting TUI)
                let _ = std::process::Command::new("xdg-open")
                    .arg(&url)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .or_else(|_| {
                        std::process::Command::new("open")
                            .arg(&url)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                    });

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let result = listener.wait_for_callback();
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });

                self.state = AppState::Login {
                    phase: LoginPhase::OAuthWaiting,
                    selected: 0,
                    auth_rx: Some(rx),
                };
            }
            Err(e) => {
                self.state = AppState::Login {
                    phase: LoginPhase::EmailInput {
                        email: String::new(),
                        password: String::new(),
                        is_signup: false,
                        field_focus: 0,
                        error: Some(format!("oauth failed: {}", e)),
                    },
                    selected: 0,
                    auth_rx: None,
                };
            }
        }
    }

    pub(crate) fn complete_auth(&mut self, tokens: myro_api::AuthTokens) {
        let client = tokens.to_client();
        let events = std::sync::Arc::new(EventBatch::new(client.clone()));
        events.track("session_start", serde_json::json!({}));
        self.api = Some(client);
        self.events = Some(events);
        self.log_debug("authenticated with supabase");

        // Sync profile if we have a CF handle
        if let Some(ref api) = self.api {
            let cf_handle = self.app_config.codeforces.handle.as_deref();
            let display_name = self.user_state.name.as_deref();
            let target_p = Some(self.app_config.recommender.target_probability);
            let _ = myro_api::sync::sync_profile(api, cf_handle, display_name, target_p);
        }

        // Return to settings with success
        self.set_status("\u{2713} signed in — progress will sync");
        self.state = AppState::Settings {
            selected: 1,
            editing: None,
        };

        // Background sync of local data
        self.sync_to_remote();
    }

    fn handle_handle_prompt_key(&mut self, key: KeyEvent) {
        let is_cookie_phase = matches!(
            &self.state,
            AppState::HandlePrompt { phase: OnboardingPhase::CookieImport, .. }
        );
        let is_validating = matches!(
            &self.state,
            AppState::HandlePrompt { validating: true, .. }
        );

        if is_validating {
            return;
        }

        if is_cookie_phase {
            self.handle_cookie_import_key(key);
        } else {
            self.handle_handle_phase_key(key);
        }
    }

    fn handle_handle_phase_key(&mut self, key: KeyEvent) {
        let (input, error) = match &mut self.state {
            AppState::HandlePrompt {
                handle_input,
                error,
                ..
            } => (handle_input, error),
            _ => return,
        };

        match key.code {
            KeyCode::Char(c) => {
                input.push(c);
                *error = None;
            }
            KeyCode::Backspace => {
                input.pop();
                *error = None;
            }
            KeyCode::Enter => {
                onboarding::start_handle_validation(&mut self.state);
            }
            _ => {}
        }
    }

    fn handle_cookie_import_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::HandlePrompt { phase, error, .. } = &mut self.state {
                    *phase = OnboardingPhase::Handle;
                    *error = None;
                }
            }
            KeyCode::Enter => {
                let handle = match &self.state {
                    AppState::HandlePrompt { handle_input, .. } => handle_input.clone(),
                    _ => return,
                };

                match onboarding::import_cookies(
                    &mut self.app_config,
                    &mut self.user_state,
                    &mut self.recommender,
                    handle,
                ) {
                    Ok(result) => {
                        self.status_message = Some(result.status_message);
                        self.state = AppState::Home { selected: 0 };
                    }
                    Err(e) => {
                        if let AppState::HandlePrompt { error, .. } = &mut self.state {
                            *error = Some(e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_home_key(&mut self, key: KeyEvent) {
        let selected = match &mut self.state {
            AppState::Home { selected } => selected,
            _ => return,
        };
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                *selected = (*selected + 1).min(MENU_ITEMS.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *selected = selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                match *selected {
                    0 => {
                        // Feed me → open sub-menu
                        self.feed_me_menu = Some(0);
                    }
                    1 => {
                        // Rate me
                        self.state = AppState::Stats { scroll: 0 };
                    }
                    2 => {
                        // My past
                        self.state = AppState::Past {
                            scroll: 0,
                            command_input: None,
                            filter_open: false,
                            filter_cursor: 0,
                            order_open: false,
                            order_cursor: 0,
                        };
                    }
                    3 => {
                        self.state = AppState::Settings {
                            selected: 1,
                            editing: None,
                        };
                    }
                    _ => {}
                }
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_stats_key(&mut self, key: KeyEvent) {
        let scroll = match &mut self.state {
            AppState::Stats { scroll } => scroll,
            _ => return,
        };
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                *scroll = scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *scroll = scroll.saturating_sub(1);
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::Home { selected: 1 };
            }
            _ => {}
        }
    }

    fn execute_confirmed_action(&mut self, action: &str) {
        match action {
            "isuck" => {
                self.user_state.isuck_explained = true;
                let _ = state::save_state(&self.user_state);
                self.execute_isuck();
            }
            "reset_history" => {
                self.user_state.solved.clear();
                self.user_state.isuck_explained = false;
                let _ = state::save_state(&self.user_state);

                // Clear past entries
                self.past_entries.clear();
                let _ = state::save_past(&self.past_entries);

                self.recommender.clear_cached_data();
                self.recommender.clear_pending_problem(&mut self.user_state);
                self.recommender.skip_auto_recommend = true;
                if let Some(handle) = &self.app_config.codeforces.handle {
                    self.recommender
                        .send(crate::recommend::RecommendRequest::FetchAndFit {
                            handle: handle.clone(),
                            model_path: self.app_config.recommender.model_path.clone(),
                        });
                }

                self.set_status("\u{2713} History reset");
            }
            _ => {}
        }
    }

    pub(crate) fn execute_isuck(&mut self) {
        if let AppState::Solving { problem, editor_state, solution_path, timer_started, timer_paused_secs, .. } = &self.state {
            let key = format!("{}:{}", problem.contest_id, problem.index);
            self.track_event("problem_abandoned", serde_json::json!({
                "problem_id": format!("cf:{}{}", problem.contest_id, problem.index),
            }));

            // Save editor content
            let text = editor_state.lines.to_string();
            solving::save_solution(solution_path.as_path(), &text);

            // Update past entry
            let time_taken = timer_started
                .map(|s| s.elapsed().as_secs().saturating_sub(*timer_paused_secs));
            solving::mark_outcome(
                &mut self.past_entries,
                problem.contest_id,
                &problem.index,
                "gave_up",
                time_taken,
            );
            let _ = state::save_past(&self.past_entries);

            // Clear saved problem
            self.recommender.clear_pending_problem(&mut self.user_state);
            self.recommender
                .send(crate::recommend::RecommendRequest::RecordAndRefit {
                    problem_key: key,
                    solved: false,
                });

            self.recommender.status = Some("updating ratings...".into());
            self.sync_to_remote();
        }
    }

    pub(crate) fn execute_skip(&mut self) {
        if self.ephemeral {
            self.should_quit = true;
            return;
        }

        if let AppState::Solving { problem, editor_state, solution_path, .. } = &self.state {
            // Save editor content
            let text = editor_state.lines.to_string();
            solving::save_solution(solution_path.as_path(), &text);

            // Update past entry
            solving::mark_outcome(
                &mut self.past_entries,
                problem.contest_id,
                &problem.index,
                "skipped",
                None,
            );
            let _ = state::save_past(&self.past_entries);

            // Clear saved problem
            self.recommender.clear_pending_problem(&mut self.user_state);

            // Get next problem (no RecordAndRefit for Chill)
            self.recommender
                .send(crate::recommend::RecommendRequest::Recommend {
                    target_p: self.app_config.recommender.target_probability,
                    solved_keys: self.user_state.solved.clone(),
                });
            self.recommender.status = Some("finding next problem...".into());
        }
    }

    pub(crate) fn save_and_finish_past(&mut self, outcome: &str) {
        if let AppState::Solving { problem, editor_state, solution_path, timer_started, timer_paused_secs, .. } = &self.state {
            let text = editor_state.lines.to_string();
            solving::save_solution(solution_path.as_path(), &text);

            let time_taken = timer_started
                .map(|s| s.elapsed().as_secs().saturating_sub(*timer_paused_secs));

            solving::mark_outcome(
                &mut self.past_entries,
                problem.contest_id,
                &problem.index,
                outcome,
                time_taken,
            );
            let _ = state::save_past(&self.past_entries);
        }
    }

    fn start_suggested_problem(&mut self, mode: SolveMode) {
        self.last_solve_mode = Some(mode);

        let handle = match &self.app_config.codeforces.handle {
            Some(h) => h.clone(),
            None => {
                self.set_status("set your cf handle in settings first");
                return;
            }
        };

        // If we already have a saved problem, just fetch its statement
        if let Some((contest_id, ref index, ..)) = self.recommender.pending_problem {
            self.recommender.status =
                Some(format!("fetching problem {}{}...", contest_id, index));
            self.recommender
                .send(crate::recommend::RecommendRequest::FetchProblem {
                    contest_id,
                    index: index.clone(),
                });
            return;
        }

        self.recommender
            .send(crate::recommend::RecommendRequest::FetchAndFit {
                handle,
                model_path: self.app_config.recommender.model_path.clone(),
            });

        self.recommender.status = Some("loading model and fetching history...".into());
    }

    // TODO: bring back when "Start training" menu item returns
    #[allow(dead_code)]
    fn start_training(&mut self) {
        let problem_set_dir = myro_coach::seed::default_problem_set_dir();
        let problems = myro_coach::seed::load_problem_set(&problem_set_dir)
            .unwrap_or_default();

        if problems.is_empty() {
            // Fallback to test problem if no problem files found
            let pf = solving::test_problem_file();
            let ps = solving::problem_file_to_statement(&pf);
            self.start_solving_problem(ps, pf);
        } else {
            self.state = AppState::ProblemSelect {
                problems,
                selected: 0,
                scroll_offset: 0,
            };
        }
    }

    fn start_solving_problem(&mut self, problem: ProblemStatement, problem_file: ProblemFile) {
        let solution_path = solving::solution_file_path(&problem);
        let initial_code = solving::load_initial_code(&solution_path);
        let user_name = self.user_state.name.as_deref().unwrap_or("User");
        let (state, coach_enabled) = solving::create_solving_state(
            problem,
            problem_file,
            initial_code,
            &self.coach_config,
            user_name,
            SolveMode::Chill,
            false,
        );

        if coach_enabled {
            let mode = if self.coach_config.mock { "mock" } else { "llm" };
            self.log_debug(format!(
                "coach spawned ({})",
                mode
            ));
            if let Some(ref url) = self.coach_config.base_url {
                self.log_debug(format!("llm endpoint: {}", url));
            }
        } else {
            self.log_debug("coach not available (no base_url configured)");
        }

        self.state = state;
    }

    pub(crate) fn start_solving_recommended(&mut self, problem: ProblemStatement) {
        let mode = self.last_solve_mode.unwrap_or(SolveMode::Chill);
        let solution_path = solving::solution_file_path(&problem);
        let initial_code = solving::load_initial_code(&solution_path);
        let loaded = solving::load_recommended_problem_file(
            &problem,
            self.recommender
                .pending_problem
                .as_ref()
                .and_then(|(_, _, _, r)| *r),
        );

        if loaded.loaded_from_set {
            self.log_debug(format!(
                "loaded problem file with {} observations",
                loaded.problem_file.total_observations()
            ));
        }
        let problem_file = loaded.problem_file;

        self.log_debug(format!(
            "solving recommended ({}): {} [{}{}]",
            if mode == SolveMode::Chill { "chill" } else { "intense" },
            problem.title, problem.contest_id, problem.index
        ));
        self.track_event("problem_started", serde_json::json!({
            "problem_id": format!("cf:{}{}", problem.contest_id, problem.index),
            "mode": if mode == SolveMode::Chill { "chill" } else { "intense" },
        }));

        let from_past = solving::sync_past_entry_on_start(
            &mut self.past_entries,
            &mut self.reopening_past_entry,
            &problem,
            &problem_file,
            mode,
            self.recommender
                .pending_problem
                .as_ref()
                .and_then(|(_, _, _, r)| *r),
        );

        let user_name = self.user_state.name.as_deref().unwrap_or("User");
        let (state, coach_enabled) = solving::create_solving_state(
            problem,
            problem_file,
            initial_code,
            &self.coach_config,
            user_name,
            mode,
            from_past,
        );
        if mode == SolveMode::Chill && !coach_enabled {
            self.log_debug("coach not available (no base_url configured)");
        }
        self.state = state;
    }

    pub(crate) fn handle_verdict(&mut self, verdict: &str) {
        let is_ac = verdict == "OK";
        let verdict_display = match verdict {
            "OK" => "accepted!",
            "WRONG_ANSWER" => "wrong answer",
            "TIME_LIMIT_EXCEEDED" => "time limit exceeded",
            "RUNTIME_ERROR" => "runtime error",
            "MEMORY_LIMIT_EXCEEDED" => "memory limit exceeded",
            "COMPILATION_ERROR" => "compilation error",
            other => other,
        };

        let current_mode = match &self.state {
            AppState::Solving { mode, .. } => *mode,
            _ => SolveMode::Chill,
        };

        // Update past entry submission tracking
        if let AppState::Solving {
            problem,
            timer_started,
            timer_paused_secs,
            ..
        } = &self.state
        {
            solving::record_submission_verdict(
                &mut self.past_entries,
                problem.contest_id,
                &problem.index,
                verdict,
                is_ac,
                timer_started.map(|s| s.elapsed().as_secs().saturating_sub(*timer_paused_secs)),
            );
            let _ = state::save_past(&self.past_entries);
        }

        if is_ac {
            self.set_status(format!("{} {}", CHECK, verdict_display));
            if let AppState::Solving { problem, .. } = &self.state {
                let key = format!("{}:{}", problem.contest_id, problem.index);
                let problem_id = format!("cf:{}{}", problem.contest_id, problem.index);
                self.track_event("problem_solved", serde_json::json!({
                    "problem_id": &problem_id,
                    "contest_id": problem.contest_id,
                    "index": &problem.index,
                }));
                self.user_state.solved.push(problem_id);
                self.recommender.clear_pending_problem(&mut self.user_state);

                match current_mode {
                    SolveMode::Chill => {
                        // No rating effects, auto-recommend next
                        self.recommender
                            .send(crate::recommend::RecommendRequest::Recommend {
                                target_p: self.app_config.recommender.target_probability,
                                solved_keys: self.user_state.solved.clone(),
                            });
                    }
                    SolveMode::Intense => {
                        // RecordAndRefit, rating popup, then post-verdict choice
                        self.recommender
                            .send(crate::recommend::RecommendRequest::RecordAndRefit {
                                problem_key: key,
                                solved: true,
                            });
                    }
                }
            }
        } else {
            self.set_status(format!("{} {}", CROSS, verdict_display));
            self.track_event("verdict_received", serde_json::json!({
                "verdict": verdict,
                "accepted": false,
            }));
        }

        // Push solution to Supabase
        if let Some(ref api) = self.api {
            if let AppState::Solving { problem, editor_state, .. } = &self.state {
                let solution = myro_api::types::SolutionRow {
                    id: None,
                    user_id: api.user_id.clone(),
                    problem_id: format!("cf:{}{}", problem.contest_id, problem.index),
                    code: editor_state.lines.to_string(),
                    language: Some("python".into()),
                    verdict: Some(verdict.to_string()),
                    submitted_at: None,
                };
                let _ = myro_api::sync::push_solution(api, &solution);
            }
        }

        // Sync after any verdict
        self.sync_to_remote();
    }

    fn handle_problem_select_key(&mut self, key: KeyEvent) {
        let (problems_len, selected, scroll_offset) = match &mut self.state {
            AppState::ProblemSelect {
                problems,
                selected,
                scroll_offset,
            } => (problems.len(), selected, scroll_offset),
            _ => return,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                *selected = (*selected + 1).min(problems_len.saturating_sub(1));
                // Scroll down if selected goes below visible area
                let visible = self.terminal_height.saturating_sub(7) as usize;
                if *selected >= *scroll_offset + visible {
                    *scroll_offset = selected.saturating_sub(visible.saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *selected = selected.saturating_sub(1);
                if *selected < *scroll_offset {
                    *scroll_offset = *selected;
                }
            }
            KeyCode::Enter => {
                if let AppState::ProblemSelect { problems, selected, .. } = &self.state {
                    if let Some(pf) = problems.get(*selected) {
                        let ps = solving::problem_file_to_statement(pf);
                        let pf = pf.clone();
                        self.start_solving_problem(ps, pf);
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::Home { selected: 0 };
            }
            _ => {}
        }
    }

}
