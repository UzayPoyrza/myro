use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use myro_coach::llm::openai_compat::OpenAiCompatibleProvider;
use myro_coach::llm::{CompletionRequest, LlmProvider};
use myro_coach::prompt::coaching::build_coaching_prompt;
use myro_coach::prompt::schema::parse_coach_response;
use myro_coach::types::CoachResponse;

use crate::scenario::{build_context, build_context_dynamic, update_obs_states, LoadedScenario};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub scenario_name: String,
    pub scenario_file: String,
    pub model: String,
    pub system_prompt: String,
    pub user_message: String,
    pub raw_response: String,
    pub parsed: CoachResponse,
    pub latency_ms: u64,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<Vec<TurnRunResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRunResult {
    pub turn_index: usize,
    pub system_prompt: String,
    pub user_message: String,
    pub raw_response: String,
    pub parsed: CoachResponse,
    pub latency_ms: u64,
    pub obs_states_after: HashMap<String, String>,
}

pub fn build_prompts(
    scenario: &LoadedScenario,
    prompts_dir: Option<&Path>,
) -> (String, String) {
    let ctx = build_context(scenario);
    build_coaching_prompt(&ctx, prompts_dir)
}

pub async fn run_scenario(
    provider: &OpenAiCompatibleProvider,
    scenario: &LoadedScenario,
    model_name: &str,
    prompts_dir: Option<&Path>,
) -> Result<RunResult> {
    let (system_prompt, user_message) = build_prompts(scenario, prompts_dir);

    let t0 = std::time::Instant::now();
    let raw_response = provider
        .complete(CompletionRequest {
            system_prompt: system_prompt.clone(),
            user_message: user_message.clone(),
            max_tokens: 2048,
            temperature: Some(0.6),
        })
        .await?;
    let latency_ms = t0.elapsed().as_millis() as u64;

    let parsed = parse_coach_response(&raw_response);

    Ok(RunResult {
        scenario_name: scenario.name.clone(),
        scenario_file: scenario.file_name.clone(),
        model: model_name.to_string(),
        system_prompt,
        user_message,
        raw_response,
        parsed,
        latency_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        turns: None,
    })
}

pub async fn run_multi_turn_scenario(
    provider: &OpenAiCompatibleProvider,
    scenario: &LoadedScenario,
    model_name: &str,
    prompts_dir: Option<&Path>,
) -> Result<RunResult> {
    let problem_id = scenario.problem.id();
    let route_obs_count = scenario
        .problem
        .routes
        .get(scenario.def.route)
        .map_or(0, |r| r.observations.len());

    // Initialize obs_states from scenario defaults
    let mut obs_states = scenario.def.obs_states.clone();
    let mut conversation_history: Vec<(String, String)> = scenario
        .def
        .recent_messages
        .iter()
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect();

    let mut turn_results = Vec::new();
    let mut last_system = String::new();
    let mut last_user = String::new();
    let mut last_raw = String::new();
    let mut last_parsed = parse_coach_response("{}"); // fallback
    let mut total_latency: u64 = 0;

    for (idx, turn) in scenario.turns.iter().enumerate() {
        // Add user message to conversation history if present
        if let Some(ref msg) = turn.user_message {
            conversation_history.push(("user".to_string(), msg.clone()));
        }

        let ctx = build_context_dynamic(
            scenario,
            &obs_states,
            &conversation_history,
            &turn.trigger,
            turn.elapsed_secs,
            &turn.code.text,
        );
        let (system_prompt, user_message) = build_coaching_prompt(&ctx, prompts_dir);

        let t0 = std::time::Instant::now();
        let raw_response = provider
            .complete(CompletionRequest {
                system_prompt: system_prompt.clone(),
                user_message: user_message.clone(),
                max_tokens: 2048,
                temperature: Some(0.6),
            })
            .await?;
        let latency_ms = t0.elapsed().as_millis() as u64;
        total_latency += latency_ms;

        let parsed = parse_coach_response(&raw_response);

        // Update obs_states monotonically
        update_obs_states(
            &mut obs_states,
            &parsed,
            route_obs_count,
            &problem_id,
            scenario.def.route,
        );

        // Add assistant response to conversation history (rolling 20)
        conversation_history.push(("assistant".to_string(), parsed.coach_message.clone()));
        if conversation_history.len() > 20 {
            let excess = conversation_history.len() - 20;
            conversation_history.drain(..excess);
        }

        last_system = system_prompt.clone();
        last_user = user_message.clone();
        last_raw = raw_response.clone();
        last_parsed = parsed.clone();

        turn_results.push(TurnRunResult {
            turn_index: idx,
            system_prompt,
            user_message,
            raw_response,
            parsed,
            latency_ms,
            obs_states_after: obs_states.clone(),
        });
    }

    Ok(RunResult {
        scenario_name: scenario.name.clone(),
        scenario_file: scenario.file_name.clone(),
        model: model_name.to_string(),
        system_prompt: last_system,
        user_message: last_user,
        raw_response: last_raw,
        parsed: last_parsed,
        latency_ms: total_latency,
        timestamp: chrono::Utc::now().to_rfc3339(),
        turns: Some(turn_results),
    })
}
