# TUI Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the overengineered multi-screen TUI with a minimal, Claude-Code-inspired linear experience using edtui for the built-in vim editor.

**Architecture:** Two-state app (Home picker, Solving editor). Single `ui.rs` render function. `edtui` crate handles vim editing + syntax highlighting. Backslash commands (`\run`, `\quit`, `\help`) for actions.

**Tech Stack:** ratatui 0.30, crossterm 0.29, edtui (syntax-highlighting feature), rusqlite, tokio, myro-cf

---

### Task 1: Update dependencies

**Files:**
- Modify: `crates/myro-tui/Cargo.toml`

**Step 1: Update Cargo.toml**

Replace the full `[dependencies]` section:

```toml
[dependencies]
myro-cf = { path = "../myro-cf" }
ratatui = "0.30"
crossterm = "0.29"
edtui = { version = "0.11", features = ["syntax-highlighting"] }
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
dirs = "6"
```

Removed: `clap`, `chrono`, `open`. Added: `edtui`. Upgraded: `ratatui` 0.29→0.30, `crossterm` 0.28→0.29.

**Step 2: Verify dependencies resolve**

Run: `cargo check -p myro-tui 2>&1 | head -20`
Expected: May have compilation errors (existing code references removed crates), but dependencies should resolve. Look for "Compiling edtui" in output.

**Step 3: Commit**

```bash
git add crates/myro-tui/Cargo.toml
git commit -m "chore: update myro-tui deps for redesign (ratatui 0.30, edtui)"
```

---

### Task 2: Delete old TUI code, create skeleton files

**Files:**
- Delete: `crates/myro-tui/src/ui/` (entire directory)
- Delete: `crates/myro-tui/src/theme.rs` (will recreate)
- Delete: `crates/myro-tui/src/app.rs` (will recreate)
- Create: `crates/myro-tui/src/theme.rs` (new, minimal)
- Create: `crates/myro-tui/src/app.rs` (new, minimal)
- Create: `crates/myro-tui/src/editor.rs` (new)
- Create: `crates/myro-tui/src/ui.rs` (new, replaces ui/ directory)
- Modify: `crates/myro-tui/src/main.rs` (minimal skeleton)
- Keep: `crates/myro-tui/src/event.rs` (as-is)
- Keep: `crates/myro-tui/src/runner.rs` (as-is)

**Step 1: Delete old ui/ directory and old modules**

```bash
rm -rf crates/myro-tui/src/ui/
```

**Step 2: Create minimal theme.rs**

```rust
use ratatui::style::{Color, Modifier, Style};

// "Quiet terminal" palette — monochrome with one accent
pub const ACCENT: Color = Color::Rgb(100, 200, 200); // soft teal
pub const DIM: Color = Color::DarkGray;
pub const TEXT: Color = Color::White;
pub const MUTED: Color = Color::Gray;
pub const SUCCESS: Color = Color::Rgb(120, 200, 120);
pub const FAIL: Color = Color::Rgb(220, 100, 100);

pub fn accent_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn dim_style() -> Style {
    Style::default().fg(DIM)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn bold_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn fail_style() -> Style {
    Style::default().fg(FAIL)
}
```

**Step 3: Create minimal app.rs skeleton**

```rust
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};
use myro_cf::types::{ProblemStatement, TestExample};

use crate::runner::TestResult;

pub struct App {
    pub state: AppState,
    pub should_quit: bool,
    pub terminal_width: u16,
    pub terminal_height: u16,
}

pub enum AppState {
    Home {
        selected: usize,
    },
    Solving {
        problem: ProblemStatement,
        editor_state: EditorState,
        editor_handler: EditorEventHandler,
        results: Option<Vec<TestResult>>,
        show_statement: bool,
        command_input: Option<String>,
        solution_path: std::path::PathBuf,
    },
}

const MENU_ITEMS: &[&str] = &[
    "Start training",
    "Browse problems (coming soon)",
    "Settings (coming soon)",
];

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            state: AppState::Home { selected: 0 },
            should_quit: false,
            terminal_width: 80,
            terminal_height: 24,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match &mut self.state {
            AppState::Home { selected } => self.handle_home_key(key),
            AppState::Solving { .. } => self.handle_solving_key(key),
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
                if *selected == 0 {
                    self.start_training();
                }
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_solving_key(&mut self, key: KeyEvent) {
        // Extract command_input state to check if we're in command mode
        let in_command_mode = matches!(
            &self.state,
            AppState::Solving { command_input: Some(_), .. }
        );

        if in_command_mode {
            self.handle_command_input(key);
            return;
        }

        // Check if `\` pressed in Normal mode to enter command mode
        let editor_mode = match &self.state {
            AppState::Solving { editor_state, .. } => editor_state.mode.clone(),
            _ => return,
        };

        if key.code == KeyCode::Char('\\') && editor_mode == EditorMode::Normal {
            if let AppState::Solving { command_input, .. } = &mut self.state {
                *command_input = Some(String::new());
            }
            return;
        }

        // Pass key to edtui
        if let AppState::Solving {
            editor_state,
            editor_handler,
            ..
        } = &mut self.state
        {
            editor_handler.on_key_event(key, editor_state);
        }
    }

    fn handle_command_input(&mut self, key: KeyEvent) {
        let (cmd, should_exec) = match &mut self.state {
            AppState::Solving { command_input, .. } => {
                let cmd = command_input.as_mut().unwrap();
                match key.code {
                    KeyCode::Esc => {
                        let _ = command_input.take();
                        return;
                    }
                    KeyCode::Enter => {
                        let c = cmd.clone();
                        let _ = command_input.take();
                        (c, true)
                    }
                    KeyCode::Backspace => {
                        cmd.pop();
                        if cmd.is_empty() {
                            let _ = command_input.take();
                        }
                        return;
                    }
                    KeyCode::Char(c) => {
                        cmd.push(c);
                        return;
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        if should_exec {
            self.execute_command(&cmd);
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "run" => self.run_tests(),
            "quit" | "q" => {
                self.state = AppState::Home { selected: 0 };
            }
            "help" | "h" => {
                // TODO: show help inline
            }
            _ => {}
        }
    }

    fn start_training(&mut self) {
        let problem = test_problem();
        let solution_path = solution_file_path(&problem);

        // Load existing solution or use template
        let initial_code = if solution_path.exists() {
            std::fs::read_to_string(&solution_path).unwrap_or_default()
        } else {
            format!(
                "# {} - {}\nimport sys\ninput = sys.stdin.readline\n\n",
                problem.index, problem.title
            )
        };

        self.state = AppState::Solving {
            problem,
            editor_state: EditorState::new(Lines::from(initial_code.as_str())),
            editor_handler: EditorEventHandler::default(),
            results: None,
            show_statement: true,
            command_input: None,
            solution_path,
        };
    }

    fn run_tests(&mut self) {
        if let AppState::Solving {
            problem,
            editor_state,
            results,
            solution_path,
            ..
        } = &mut self.state
        {
            // Save editor content to file
            let text = lines_to_string(&editor_state.lines);
            let _ = std::fs::create_dir_all(solution_path.parent().unwrap());
            let _ = std::fs::write(&solution_path, &text);

            // Run tests
            let test_results =
                crate::runner::run_tests(solution_path, &problem.examples, "python3");
            *results = Some(test_results);
        }
    }
}

fn test_problem() -> ProblemStatement {
    ProblemStatement {
        contest_id: 1,
        index: "A".to_string(),
        title: "Sum of Two Numbers".to_string(),
        time_limit: "2 seconds".to_string(),
        memory_limit: "256 megabytes".to_string(),
        description: "You are given two integers a and b. Print their sum.".to_string(),
        input_spec: "The first line contains two integers a and b (1 ≤ a, b ≤ 10^9).".to_string(),
        output_spec: "Print the sum a + b.".to_string(),
        examples: vec![
            TestExample {
                input: "3 5".to_string(),
                output: "8".to_string(),
            },
            TestExample {
                input: "100 200".to_string(),
                output: "300".to_string(),
            },
        ],
        note: None,
    }
}

fn solution_file_path(problem: &ProblemStatement) -> std::path::PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("myro")
        .join("solutions");
    data_dir.join(format!("{}{}.py", problem.contest_id, problem.index))
}

/// Convert edtui Lines (Jagged2<char>) to a String
fn lines_to_string(lines: &Lines) -> String {
    let rows: Vec<String> = lines.iter().map(|row| row.iter().collect()).collect();
    rows.join("\n")
}
```

**Step 4: Create minimal editor.rs**

```rust
use edtui::{EditorTheme, EditorView, EditorState, SyntaxHighlighter, LineNumbers};
use ratatui::{layout::Rect, style::Style, Frame};

use crate::theme;

pub fn editor_theme() -> EditorTheme<'static> {
    EditorTheme::default()
        .base(Style::default().fg(theme::TEXT))
        .cursor_style(Style::default().fg(ratatui::style::Color::Black).bg(theme::TEXT))
        .selection_style(Style::default().bg(ratatui::style::Color::Rgb(60, 60, 80)))
        .line_numbers_style(Style::default().fg(theme::DIM))
        .hide_status_line()
}

pub fn python_highlighter() -> Option<SyntaxHighlighter> {
    SyntaxHighlighter::new("base16-ocean.dark", "py").ok()
}

pub fn render_editor(frame: &mut Frame, area: Rect, state: &mut EditorState) {
    let highlighter = python_highlighter();
    let view = EditorView::new(state)
        .theme(editor_theme())
        .syntax_highlighter(highlighter)
        .line_numbers(LineNumbers::Absolute)
        .tab_width(4);
    frame.render_widget(view, area);
}
```

**Step 5: Create minimal ui.rs**

```rust
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, AppState, MENU_ITEMS};
use crate::editor;
use crate::theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.terminal_width = area.width;
    app.terminal_height = area.height;

    match &app.state {
        AppState::Home { .. } => render_home(frame, app, area),
        AppState::Solving { .. } => render_solving(frame, app, area),
    }
}

fn render_home(frame: &mut Frame, app: &App, area: Rect) {
    let selected = match &app.state {
        AppState::Home { selected } => *selected,
        _ => 0,
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Length(2), // spacer + prompt
        Constraint::Min(0),   // menu
    ])
    .split(area);

    // Header
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled("myro", theme::accent_style()),
    ]);
    frame.render_widget(Paragraph::new(vec![header, Line::raw("")]), chunks[0]);

    // Separator + prompt
    let sep = "─".repeat(area.width as usize);
    let prompt = Line::from(vec![
        Span::raw("  "),
        Span::styled("What would you like to do?", theme::muted_style()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::styled(&sep, theme::dim_style()), prompt]),
        chunks[1],
    );

    // Menu items
    let mut lines = vec![Line::raw("")];
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let (prefix, style) = if i == selected {
            ("  > ", theme::bold_style())
        } else {
            ("    ", theme::muted_style())
        };
        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(*item, style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);
}

fn render_solving(frame: &mut Frame, app: &mut App, area: Rect) {
    // We need mutable access to editor_state for EditorView
    // Split the layout first, then access app state

    let show_statement = match &app.state {
        AppState::Solving { show_statement, .. } => *show_statement,
        _ => return,
    };

    let has_results = match &app.state {
        AppState::Solving { results, .. } => results.is_some(),
        _ => false,
    };

    // Compute layout based on what's visible
    let statement_height = if show_statement {
        // Roughly: header(2) + desc(4) + io_spec(4) + examples(6) + padding
        16u16.min(area.height / 2)
    } else {
        0
    };

    let results_height = if has_results { 4 } else { 0 };
    let status_height = 1;

    let chunks = Layout::vertical([
        Constraint::Length(statement_height), // statement
        Constraint::Length(if statement_height > 0 { 1 } else { 0 }), // separator
        Constraint::Length(results_height),   // test results
        Constraint::Min(3),                   // editor
        Constraint::Length(1),                // separator
        Constraint::Length(status_height),    // status line
    ])
    .split(area);

    // Render each section — we need to destructure carefully for borrow checker
    if let AppState::Solving {
        problem,
        editor_state,
        results,
        command_input,
        ..
    } = &mut app.state
    {
        // Statement
        if show_statement {
            render_statement(frame, chunks[0], problem);
            let sep = "─".repeat(area.width as usize);
            frame.render_widget(
                Paragraph::new(Line::styled(sep, theme::dim_style())),
                chunks[1],
            );
        }

        // Test results
        if let Some(results) = results {
            render_results(frame, chunks[2], results);
        }

        // Editor
        editor::render_editor(frame, chunks[3], editor_state);

        // Bottom separator
        let sep = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::styled(sep, theme::dim_style())),
            chunks[4],
        );

        // Status line
        render_status_line(frame, chunks[5], &editor_state.mode, command_input);
    }
}

fn render_statement(frame: &mut Frame, area: Rect, problem: &myro_cf::ProblemStatement) {
    let mut lines = vec![];

    // Title
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}. {}", problem.index, problem.title),
            theme::accent_style(),
        ),
        Span::raw("    "),
        Span::styled(
            format!("time: {} | mem: {}", problem.time_limit, problem.memory_limit),
            theme::muted_style(),
        ),
    ]));
    lines.push(Line::raw(""));

    // Description
    for desc_line in problem.description.lines() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(desc_line.to_string()),
        ]));
    }
    lines.push(Line::raw(""));

    // Input spec
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Input   ", theme::muted_style()),
        Span::raw(problem.input_spec.lines().next().unwrap_or("")),
    ]));

    // Output spec
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Output  ", theme::muted_style()),
        Span::raw(problem.output_spec.lines().next().unwrap_or("")),
    ]));
    lines.push(Line::raw(""));

    // Examples
    for (i, ex) in problem.examples.iter().enumerate() {
        let label = format!("  -- Example {} ", i + 1);
        let dashes = "─".repeat((area.width as usize).saturating_sub(label.len() + 2));
        lines.push(Line::styled(
            format!("{}{}", label, dashes),
            theme::dim_style(),
        ));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("stdin   ", theme::muted_style()),
            Span::raw(ex.input.replace('\n', "  ")),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("stdout  ", theme::muted_style()),
            Span::raw(ex.output.replace('\n', "  ")),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_results(frame: &mut Frame, area: Rect, results: &[crate::runner::TestResult]) {
    let mut lines = vec![];
    for r in results {
        let status = if r.passed {
            Span::styled("PASS", theme::success_style())
        } else {
            Span::styled("FAIL", theme::fail_style())
        };
        let mut spans = vec![
            Span::raw("  "),
            Span::styled(format!("Test {} ", r.test_num), theme::muted_style()),
            status,
            Span::styled(format!("  {:.2}s", r.runtime_ms as f64 / 1000.0), theme::dim_style()),
        ];
        if !r.passed {
            spans.push(Span::styled(
                format!("  expected: {}  got: {}", r.expected, r.actual),
                theme::fail_style(),
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_status_line(
    frame: &mut Frame,
    area: Rect,
    mode: &edtui::EditorMode,
    command_input: &Option<String>,
) {
    let left = if let Some(cmd) = command_input {
        Span::styled(format!("  \\{}", cmd), theme::accent_style())
    } else {
        let mode_str = match mode {
            edtui::EditorMode::Normal => "NORMAL",
            edtui::EditorMode::Insert => "INSERT",
            edtui::EditorMode::Visual => "VISUAL",
            edtui::EditorMode::Search => "SEARCH",
        };
        Span::styled(format!("  {}", mode_str), theme::muted_style())
    };

    let right = Span::styled("\\run  \\help  \\quit  ", theme::dim_style());

    // Calculate padding
    let left_len = left.content.len();
    let right_len = right.content.len();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let line = Line::from(vec![left, Span::raw(" ".repeat(padding)), right]);
    frame.render_widget(Paragraph::new(line), area);
}
```

**Step 6: Create minimal main.rs**

```rust
mod app;
mod editor;
mod event;
mod runner;
mod theme;
mod ui;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::app::App;
use crate::event::{AppEvent, EventReader};

fn main() -> Result<()> {
    // Init terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut app = App::new()?;
    let events = EventReader::new();

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        match events.next()? {
            AppEvent::Key(key) => app.handle_key(key),
            AppEvent::Resize(w, h) => {
                app.terminal_width = w;
                app.terminal_height = h;
            }
            AppEvent::Tick => {}
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
```

**Step 7: Verify it compiles**

Run: `cargo check -p myro-tui`
Expected: Clean compilation (no errors, maybe warnings about unused items)

**Step 8: Commit**

```bash
git add -A crates/myro-tui/src/
git commit -m "rewrite: minimal TUI with edtui vim editor

Delete old multi-screen architecture (sidebar, blocks, 2D nav).
Replace with two-state linear flow: Home picker → Solving editor.
Uses edtui for built-in vim editing with syntax highlighting."
```

---

### Task 3: Fix compilation issues and polish

This task handles any compilation errors from Task 2 (ratatui 0.30 API changes, edtui integration quirks, borrow checker issues).

**Files:**
- Modify: any files from Task 2 that need fixes

**Step 1: Run cargo check and fix errors**

Run: `cargo check -p myro-tui 2>&1`
Fix each error iteratively. Common issues:
- `EditorMode` might not derive `Clone`/`PartialEq` — compare with matches! macro instead
- `Lines::from(&str)` might need specific import
- ratatui 0.30 may have minor API changes from 0.29
- `EditorView` is a Widget rendered via `frame.render_widget()` — may need `&mut state` borrow dance

**Step 2: Run cargo clippy**

Run: `cargo clippy -p myro-tui 2>&1`
Fix all warnings.

**Step 3: Run the app**

Run: `cargo run -p myro-tui`
Verify:
- Home screen shows `myro` header and menu
- j/k moves selection
- Enter on "Start training" opens the editor
- Vim motions work (i to insert, Esc to normal, hjkl navigation)
- `\` in normal mode shows command input
- `\run` executes tests
- `\quit` returns to home
- `q` on home screen quits

**Step 4: Commit fixes**

```bash
git add -A crates/myro-tui/src/
git commit -m "fix: resolve compilation and runtime issues from TUI rewrite"
```

---

### Task 4: Aesthetic polish

**Files:**
- Modify: `crates/myro-tui/src/theme.rs`
- Modify: `crates/myro-tui/src/ui.rs`
- Modify: `crates/myro-tui/src/editor.rs`

**Step 1: Tune the color palette**

Test different syntax highlighting themes in `editor.rs`. Try: `"base16-ocean.dark"`, `"base16-mocha.dark"`, `"Solarized (dark)"`. Pick the most muted/elegant one.

Adjust `theme.rs` accent color to complement the chosen syntax theme.

**Step 2: Polish the statement layout**

- Ensure proper padding (2 chars from left edge everywhere)
- Test with terminal widths 60, 80, 120
- Multi-line descriptions should wrap cleanly
- Example separators should span the content width

**Step 3: Polish the editor appearance**

- Ensure line numbers are dim enough
- Empty lines show `~` in dim gray (check if edtui does this natively or needs custom)
- Cursor should be visible in both normal and insert modes

**Step 4: Polish the status line**

- Mode indicator left-aligned, dim
- Command hints right-aligned, dimmer
- When in command input mode: `\run` shows in accent color with cursor

**Step 5: Test full workflow visually**

Run: `cargo run -p myro-tui`
Walk through: Home → Train → write `print(int(input().split()[0]) + int(input().split()[1]))` → `\run` → see results → `\quit` → `q`

**Step 6: Commit**

```bash
git add crates/myro-tui/src/
git commit -m "style: polish TUI aesthetics — muted palette, clean layout"
```

---

### Task 5: Verify everything works end-to-end

**Step 1: Clean build**

Run: `cargo build -p myro-tui`
Expected: Clean build, no warnings.

**Step 2: Run workspace tests**

Run: `cargo test --workspace`
Expected: All existing tests pass (myro-predict tests should be unaffected).

**Step 3: Run clippy on workspace**

Run: `cargo clippy --workspace`
Expected: No warnings.

**Step 4: Manual smoke test**

Run: `cargo run -p myro-tui`
1. Home screen renders cleanly
2. j/k/Enter navigates menu
3. Editor opens with template code
4. Vim motions work: i, Esc, hjkl, dd, yy, p, o, w, b
5. Syntax highlighting colors Python keywords
6. `\run` saves file and runs tests
7. Results show PASS/FAIL inline
8. `\quit` returns to Home
9. `q` quits cleanly (terminal restored properly)

**Step 5: Commit if any final tweaks needed**

```bash
git add -A
git commit -m "chore: final verification pass for TUI redesign"
```
