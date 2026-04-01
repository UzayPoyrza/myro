use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use myro_coach::prompt::coaching::{CoachingPromptContext, ObservationStatus};
use myro_coach::seed::{load_problem_file, ProblemFile};

#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub scenario: ScenarioDef,
    pub expect: Option<ScenarioExpect>,
    #[serde(default)]
    pub turns: Vec<TurnDef>,
}

#[derive(Debug, Deserialize)]
pub struct TurnDef {
    pub trigger: String,
    pub elapsed_secs: u64,
    pub code: CodeBlock,
    pub user_message: Option<String>,
    pub expect: ScenarioExpect,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioDef {
    pub name: String,
    pub problem: String,
    pub route: usize,
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub elapsed_secs: u64,
    #[serde(default)]
    #[allow(dead_code)]
    pub observations_found: usize,
    #[serde(default)]
    pub obs_states: HashMap<String, String>,
    #[serde(default)]
    pub code: CodeBlock,
    #[serde(default)]
    pub recent_messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CodeBlock {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioExpect {
    pub state: Option<String>,
    #[serde(default)]
    pub valid_states: Vec<String>,
    #[serde(default)]
    pub banned_states: Vec<String>,
    pub obs_id: Option<String>,
    pub min_confidence: Option<f64>,
    pub max_confidence: Option<f64>,
    #[serde(default = "default_true")]
    pub must_be_socratic: bool,
    #[serde(default)]
    pub banned_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

pub struct LoadedScenario {
    pub name: String,
    pub file_name: String,
    pub problem: ProblemFile,
    pub def: ScenarioDef,
    pub expect: Option<ScenarioExpect>,
    pub turns: Vec<TurnDef>,
}

impl LoadedScenario {
    pub fn is_multi_turn(&self) -> bool {
        !self.turns.is_empty()
    }
}

pub fn load_scenarios(scenarios_dir: &Path, problem_set_dir: &Path) -> Result<Vec<LoadedScenario>> {
    let mut entries: Vec<_> = std::fs::read_dir(scenarios_dir)
        .with_context(|| format!("reading scenarios dir: {}", scenarios_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "toml")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut scenarios = Vec::new();
    for entry in entries {
        let path = entry.path();
        let file_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading scenario: {}", path.display()))?;
        let file: ScenarioFile = toml::from_str(&content)
            .with_context(|| format!("parsing scenario: {}", path.display()))?;

        let problem_path = problem_set_dir.join(format!("{}.json", file.scenario.problem));
        let problem = load_problem_file(&problem_path)
            .with_context(|| format!("loading problem {} for scenario {}", file.scenario.problem, file_name))?;

        scenarios.push(LoadedScenario {
            name: file.scenario.name.clone(),
            file_name,
            problem,
            def: file.scenario,
            expect: file.expect,
            turns: file.turns,
        });
    }

    Ok(scenarios)
}

pub fn build_context(scenario: &LoadedScenario) -> CoachingPromptContext {
    let problem = &scenario.problem;
    let def = &scenario.def;

    let route = problem.routes.get(def.route);
    let problem_id = problem.id();

    let observations = if let Some(route) = route {
        route
            .observations
            .iter()
            .enumerate()
            .map(|(i, obs)| {
                let state = def
                    .obs_states
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or_else(|| "locked".to_string());
                ObservationStatus {
                    id: format!("{}:route:{}:obs:{}", problem_id, def.route + 1, i + 1),
                    title: obs.title.clone(),
                    description: obs.description.clone(),
                    state,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let recent_messages = def
        .recent_messages
        .iter()
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect();

    CoachingPromptContext {
        user_name: "Bench User".to_string(),
        problem_title: problem.title.clone(),
        problem_difficulty: Some(problem.difficulty),
        problem_description: problem.description.clone(),
        route_name: route.map(|r| r.name.clone()),
        route_description: route.map(|r| r.description.clone()),
        observations,
        code: def.code.text.clone(),
        trigger: def.trigger.clone(),
        recent_messages,
        elapsed_secs: def.elapsed_secs,
    }
}

/// Build a CoachingPromptContext with dynamic state for multi-turn scenarios.
pub fn build_context_dynamic(
    scenario: &LoadedScenario,
    obs_states: &HashMap<String, String>,
    conversation_history: &[(String, String)],
    trigger: &str,
    elapsed_secs: u64,
    code: &str,
) -> CoachingPromptContext {
    let problem = &scenario.problem;
    let def = &scenario.def;

    let route = problem.routes.get(def.route);
    let problem_id = problem.id();

    let observations = if let Some(route) = route {
        route
            .observations
            .iter()
            .enumerate()
            .map(|(i, obs)| {
                let state = obs_states
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or_else(|| "locked".to_string());
                ObservationStatus {
                    id: format!("{}:route:{}:obs:{}", problem_id, def.route + 1, i + 1),
                    title: obs.title.clone(),
                    description: obs.description.clone(),
                    state,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    CoachingPromptContext {
        user_name: "Bench User".to_string(),
        problem_title: problem.title.clone(),
        problem_difficulty: Some(problem.difficulty),
        problem_description: problem.description.clone(),
        route_name: route.map(|r| r.name.clone()),
        route_description: route.map(|r| r.description.clone()),
        observations,
        code: code.to_string(),
        trigger: trigger.to_string(),
        recent_messages: conversation_history.to_vec(),
        elapsed_secs,
    }
}

/// Update observation states based on LLM response (monotonic: Locked→Approaching→Found).
pub fn update_obs_states(
    obs_states: &mut HashMap<String, String>,
    response: &myro_coach::types::CoachResponse,
    route_obs_count: usize,
    problem_id: &str,
    route_idx: usize,
) {
    if let Some(ref obs_id) = response.matched_observation_id {
        for i in 0..route_obs_count {
            let expected_id = format!("{}:route:{}:obs:{}", problem_id, route_idx + 1, i + 1);
            if expected_id == *obs_id {
                let current = obs_states
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or_else(|| "locked".to_string());
                let should_update = matches!(
                    (current.as_str(), response.state.as_str()),
                    ("locked", "approaching")
                        | ("locked", "found")
                        | ("approaching", "found")
                );
                if should_update {
                    obs_states.insert(i.to_string(), response.state.clone());
                }
                break;
            }
        }
    }
}
