//! Test harness for myro-tui integration tests.
//!
//! Provides `TestApp` (builder for `App` + `Terminal<TestBackend>`),
//! `CapturedFrame` (text snapshot), and `Scenario` (fluent scripting).

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::app::{App, AppState, PastFilter, PastOrder, SolveMode};
use crate::recommender_state::RecommenderState;
use crate::state::UserState;
use crate::ui;

/// A captured frame from the test terminal.
pub struct CapturedFrame {
    rows: Vec<String>,
}

impl CapturedFrame {
    fn from_terminal(terminal: &Terminal<TestBackend>) -> Self {
        let buf = terminal.backend().buffer();
        let area = buf.area();
        let mut rows = Vec::with_capacity(area.height as usize);
        for y in area.y..area.y + area.height {
            let mut row = String::new();
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    let sym = cell.symbol();
                    // Skip wide-char continuation cells (empty string)
                    if !sym.is_empty() {
                        row.push_str(sym);
                    }
                }
            }
            rows.push(row);
        }
        CapturedFrame { rows }
    }

    /// Check if any row contains the given text.
    pub fn contains_text(&self, needle: &str) -> bool {
        self.rows.iter().any(|r| r.contains(needle))
    }

    /// Get text content of a specific row (0-indexed).
    pub fn text_at_row(&self, row: u16) -> &str {
        self.rows
            .get(row as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Full frame as a multiline string (for debugging).
    pub fn dump(&self) -> String {
        self.rows.join("\n")
    }

    /// Write frame to a file (for Claude to Read).
    pub fn dump_to_file(&self, path: &str) {
        let _ = std::fs::write(path, self.dump());
    }
}

/// Builder for constructing a testable `App` without filesystem/network.
pub struct TestAppBuilder {
    state: AppState,
    width: u16,
    height: u16,
    user_state: UserState,
}

impl TestAppBuilder {
    fn new(state: AppState) -> Self {
        Self {
            state,
            width: 120,
            height: 40,
            user_state: UserState::default(),
        }
    }

    /// Set terminal dimensions (default 120x40).
    pub fn size(mut self, w: u16, h: u16) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    /// Set custom user state.
    pub fn user_state(mut self, state: UserState) -> Self {
        self.user_state = state;
        self
    }

    /// Build the TestApp.
    pub fn build(self) -> TestApp {
        let app = App {
            state: self.state,
            should_quit: false,
            quit_pending: false,
            status_message: None,
            terminal_width: self.width,
            terminal_height: self.height,
            tick: 0,
            coach_config: myro_coach::config::CoachConfig::default(),
            app_config: crate::config::AppConfig::default(),
            user_state: self.user_state,
            recommender: RecommenderState::empty(),
            debug_log: Vec::new(),
            debug_visible: false,
            debug_scroll: 0,
            confirm_popup: None,
            status_clear_tick: 0,
            ephemeral: false,
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
        };

        let backend = TestBackend::new(self.width, self.height);
        let terminal = Terminal::new(backend).expect("failed to create test terminal");

        TestApp { app, terminal }
    }
}

/// Wraps `App` + `Terminal<TestBackend>` for testing.
pub struct TestApp {
    pub app: App,
    pub terminal: Terminal<TestBackend>,
}

impl TestApp {
    /// Start at the Home screen.
    pub fn home() -> TestAppBuilder {
        TestAppBuilder::new(AppState::Home { selected: 0 })
    }

    /// Start at the Settings screen.
    pub fn settings() -> TestAppBuilder {
        TestAppBuilder::new(AppState::Settings {
            selected: 1,
            editing: None,
            dropdown: None,
        })
    }

    /// Start at the Past screen.
    pub fn past() -> TestAppBuilder {
        TestAppBuilder::new(AppState::Past {
            scroll: 0,
            command_input: None,
            filter_open: false,
            filter_cursor: 0,
            order_open: false,
            order_cursor: 0,
        })
    }

    /// Start at the Solving screen with a given problem.
    pub fn solving(
        problem: myro_cf::types::ProblemStatement,
        problem_file: myro_coach::seed::ProblemFile,
    ) -> TestAppBuilder {
        use edtui::{EditorEventHandler, EditorState, Lines};

        let solution_path = std::path::PathBuf::from("/tmp/myro-test-solution.py");
        let state = AppState::Solving {
            problem: Box::new(problem),
            problem_file: Box::new(problem_file),
            editor_state: Box::new(EditorState::new(Lines::from(""))),
            editor_handler: EditorEventHandler::default(),
            results: None,
            running: None,
            show_statement: true,
            statement_scroll: 0,
            statement_focused: false,
            command_input: None,
            solution_path,
            coach: None,
            test_panel: None,
            mode: SolveMode::Chill,
            timer_started: None,
            timer_paused_secs: 0,
            timer_expired: false,
            from_past: false,
        };
        TestAppBuilder::new(state)
    }

    /// Send a key press and run one tick.
    pub fn press(&mut self, code: KeyCode) {
        self.app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        self.app.tick();
    }

    /// Send a key press with modifiers and run one tick.
    pub fn press_mod(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.app.handle_key(KeyEvent::new(code, modifiers));
        self.app.tick();
    }

    /// Type a string, sending each char as a key event.
    pub fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.app
                .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        self.app.tick();
    }

    /// Run N ticks without key input.
    pub fn tick_n(&mut self, n: u64) {
        for _ in 0..n {
            self.app.tick();
        }
    }

    /// Render the current state and return a captured frame.
    pub fn render(&mut self) -> CapturedFrame {
        self.terminal
            .draw(|frame| ui::render(frame, &mut self.app))
            .expect("render failed");
        CapturedFrame::from_terminal(&self.terminal)
    }
}

/// Fluent scripting API for test scenarios.
pub struct Scenario {
    steps: Vec<ScenarioStep>,
}

enum ScenarioStep {
    Press(KeyCode),
    PressN(KeyCode, usize),
    PressMod(KeyCode, KeyModifiers),
    TypeStr(String),
    TickN(u64),
    Capture(String),
}

impl Scenario {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn press(mut self, code: KeyCode) -> Self {
        self.steps.push(ScenarioStep::Press(code));
        self
    }

    pub fn press_n(mut self, code: KeyCode, n: usize) -> Self {
        self.steps.push(ScenarioStep::PressN(code, n));
        self
    }

    pub fn press_mod(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.steps.push(ScenarioStep::PressMod(code, modifiers));
        self
    }

    pub fn type_str(mut self, s: &str) -> Self {
        self.steps.push(ScenarioStep::TypeStr(s.to_string()));
        self
    }

    pub fn tick_n(mut self, n: u64) -> Self {
        self.steps.push(ScenarioStep::TickN(n));
        self
    }

    pub fn capture(mut self, name: &str) -> Self {
        self.steps.push(ScenarioStep::Capture(name.to_string()));
        self
    }

    /// Execute all steps and return named captures.
    pub fn run(self, test_app: &mut TestApp) -> HashMap<String, CapturedFrame> {
        let mut captures = HashMap::new();
        for step in self.steps {
            match step {
                ScenarioStep::Press(code) => test_app.press(code),
                ScenarioStep::PressN(code, n) => {
                    for _ in 0..n {
                        test_app.press(code);
                    }
                }
                ScenarioStep::PressMod(code, mods) => test_app.press_mod(code, mods),
                ScenarioStep::TypeStr(s) => test_app.type_str(&s),
                ScenarioStep::TickN(n) => test_app.tick_n(n),
                ScenarioStep::Capture(name) => {
                    captures.insert(name, test_app.render());
                }
            }
        }
        captures
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new()
    }
}
