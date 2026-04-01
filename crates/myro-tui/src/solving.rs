use std::path::{Path, PathBuf};
use std::time::Instant;

use edtui::{EditorEventHandler, EditorState, Lines};
use myro_cf::types::{ProblemStatement, TestExample};
use myro_coach::{
    config::CoachConfig,
    seed::{self, ExampleFile, ProblemFile},
};

use crate::{
    app::{AppState, SolveMode},
    runner::TestResult,
    state::{self, PastEntry},
};

pub fn test_problem_file() -> ProblemFile {
    ProblemFile {
        contest_id: 1,
        index: "A".to_string(),
        title: "Sum of Two Numbers".to_string(),
        difficulty: 800,
        tags: vec!["math".to_string()],
        time_limit: Some("2 seconds".to_string()),
        memory_limit: Some("256 megabytes".to_string()),
        description: "You are given two integers a and b. Print their sum.".to_string(),
        input_spec: "The first line contains two integers a and b (1 <= a, b <= 10^9).".to_string(),
        output_spec: "Print the sum a + b.".to_string(),
        examples: vec![
            ExampleFile {
                input: "3 5".to_string(),
                output: "8".to_string(),
            },
            ExampleFile {
                input: "100 200".to_string(),
                output: "300".to_string(),
            },
        ],
        routes: vec![],
    }
}

pub fn problem_file_to_statement(pf: &ProblemFile) -> ProblemStatement {
    ProblemStatement {
        contest_id: pf.contest_id,
        index: pf.index.clone(),
        title: pf.title.clone(),
        time_limit: pf
            .time_limit
            .clone()
            .unwrap_or_else(|| "2 seconds".to_string()),
        memory_limit: pf
            .memory_limit
            .clone()
            .unwrap_or_else(|| "256 megabytes".to_string()),
        description: pf.description.clone(),
        input_spec: pf.input_spec.clone(),
        output_spec: pf.output_spec.clone(),
        examples: pf
            .examples
            .iter()
            .map(|e| TestExample {
                input: e.input.clone(),
                output: e.output.clone(),
            })
            .collect(),
        note: None,
    }
}

pub fn solution_file_path(problem: &ProblemStatement) -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myro")
        .join("solutions");
    data_dir.join(format!("{}{}.py", problem.contest_id, problem.index))
}

pub fn load_initial_code(solution_path: &Path) -> String {
    if solution_path.exists() {
        std::fs::read_to_string(solution_path).unwrap_or_default()
    } else {
        String::new()
    }
}

pub struct RecommendedProblemLoad {
    pub problem_file: ProblemFile,
    pub loaded_from_set: bool,
}

pub fn load_recommended_problem_file(
    problem: &ProblemStatement,
    fallback_rating: Option<i32>,
) -> RecommendedProblemLoad {
    let problem_set_dir = seed::default_problem_set_dir();
    let problem_json = problem_set_dir.join(format!("cf-{}{}.json", problem.contest_id, problem.index));

    if let Ok(problem_file) = seed::load_problem_file(&problem_json) {
        RecommendedProblemLoad {
            problem_file,
            loaded_from_set: true,
        }
    } else {
        RecommendedProblemLoad {
            problem_file: ProblemFile {
                contest_id: problem.contest_id,
                index: problem.index.clone(),
                title: problem.title.clone(),
                difficulty: fallback_rating.unwrap_or(0),
                tags: vec![],
                time_limit: Some(problem.time_limit.clone()),
                memory_limit: Some(problem.memory_limit.clone()),
                description: problem.description.clone(),
                input_spec: problem.input_spec.clone(),
                output_spec: problem.output_spec.clone(),
                examples: problem
                    .examples
                    .iter()
                    .map(|e| ExampleFile {
                        input: e.input.clone(),
                        output: e.output.clone(),
                    })
                    .collect(),
                routes: vec![],
            },
            loaded_from_set: false,
        }
    }
}

pub fn create_solving_state(
    problem: ProblemStatement,
    problem_file: ProblemFile,
    initial_code: String,
    coach_config: &CoachConfig,
    user_name: &str,
    mode: SolveMode,
    from_past: bool,
) -> (AppState, bool) {
    let solution_path = solution_file_path(&problem);
    let mut coach = match mode {
        SolveMode::Chill => crate::coach::bridge::spawn_coach(coach_config, &problem_file, user_name)
            .map(Box::new),
        SolveMode::Intense => None,
    };

    if let Some(ref mut coach_state) = coach {
        coach_state.panel_visible = true;
    }

    let state = AppState::Solving {
        problem: Box::new(problem),
        problem_file: Box::new(problem_file),
        editor_state: Box::new(EditorState::new(Lines::from(initial_code.as_str()))),
        editor_handler: EditorEventHandler::default(),
        results: None::<Vec<TestResult>>,
        running: None,
        show_statement: true,
        statement_scroll: 0,
        statement_focused: false,
        command_input: None,
        solution_path,
        coach,
        test_panel: None,
        mode,
        timer_started: (mode == SolveMode::Intense).then(Instant::now),
        timer_paused_secs: 0,
        timer_expired: false,
        from_past,
    };

    (state, matches!(mode, SolveMode::Chill))
}

pub fn sync_past_entry_on_start(
    entries: &mut Vec<PastEntry>,
    reopening_past_entry: &mut Option<usize>,
    problem: &ProblemStatement,
    problem_file: &ProblemFile,
    mode: SolveMode,
    rating: Option<i32>,
) -> bool {
    let from_past = reopening_past_entry.is_some();
    let now = chrono::Utc::now().timestamp();
    let mode_label = if mode == SolveMode::Chill { "chill" } else { "intense" };

    if let Some(idx) = reopening_past_entry.take() {
        if let Some(entry) = entries.get_mut(idx) {
            entry.last_seen_at = now;
        }
    } else if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.contest_id == problem.contest_id && entry.index == problem.index)
    {
        entry.last_seen_at = now;
        entry.mode = mode_label.into();
        if entry.finished_at.is_some() {
            entry.outcome = "in_progress".into();
            entry.finished_at = None;
        }
    } else {
        entries.push(PastEntry {
            contest_id: problem.contest_id,
            index: problem.index.clone(),
            title: problem.title.clone(),
            rating,
            tags: problem_file.tags.clone(),
            mode: mode_label.into(),
            outcome: "in_progress".into(),
            last_verdict: None,
            ever_accepted: false,
            ever_submitted: false,
            first_seen_at: now,
            last_seen_at: now,
            first_submitted_at: None,
            last_submitted_at: None,
            finished_at: None,
            time_taken_secs: None,
        });
    }

    let _ = state::save_past(entries);
    from_past
}

pub fn save_solution(path: &Path, code: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, code);
}

pub fn mark_outcome(
    entries: &mut [PastEntry],
    contest_id: i64,
    index: &str,
    outcome: &str,
    time_taken_secs: Option<u64>,
) {
    let now = chrono::Utc::now().timestamp();
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.contest_id == contest_id && entry.index == index)
    {
        entry.outcome = outcome.into();
        entry.finished_at = Some(now);
        if time_taken_secs.is_some() {
            entry.time_taken_secs = time_taken_secs;
        }
    }
}

pub fn record_submission_verdict(
    entries: &mut [PastEntry],
    contest_id: i64,
    index: &str,
    verdict: &str,
    accepted: bool,
    time_taken_secs: Option<u64>,
) {
    let now = chrono::Utc::now().timestamp();
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.contest_id == contest_id && entry.index == index)
    {
        entry.ever_submitted = true;
        if entry.first_submitted_at.is_none() {
            entry.first_submitted_at = Some(now);
        }
        entry.last_submitted_at = Some(now);
        entry.last_verdict = Some(verdict.to_string());
        if accepted {
            entry.ever_accepted = true;
            entry.outcome = "accepted".into();
            entry.finished_at = Some(now);
            entry.time_taken_secs = time_taken_secs;
        }
    }
}
