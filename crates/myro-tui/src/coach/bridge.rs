use std::path::PathBuf;
use std::sync::mpsc;

use myro_coach::config::CoachConfig;
use myro_coach::llm::openai_compat::OpenAiCompatibleProvider;
use myro_coach::llm::{CompletionRequest, LlmProvider};
use myro_coach::prompt::coaching::{
    build_coaching_prompt, CoachingPromptContext, ObservationStatus,
};
use myro_coach::prompt::schema::parse_coach_response;
use myro_coach::seed::{HintsFile, ProblemFile};
use myro_coach::types::{CoachResponse, GhostFormat, ObservationState};

use super::{CoachEvent, CoachRequest, CoachState};

/// Spawn the coach background thread. Returns CoachState if coach is available.
pub fn spawn_coach(
    config: &CoachConfig,
    problem: &ProblemFile,
    user_name: &str,
) -> Option<CoachState> {
    if !config.is_available() {
        return None;
    }

    let (req_tx, req_rx) = mpsc::channel::<CoachRequest>();
    let (evt_tx, evt_rx) = mpsc::channel::<CoachEvent>();

    if config.mock {
        let problem = problem.clone();
        spawn_mock_thread(req_rx, evt_tx, problem);
    } else {
        let provider = OpenAiCompatibleProvider::from_config(config)?;
        let problem = problem.clone();
        let user_name = user_name.to_string();
        spawn_llm_thread(provider, req_rx, evt_tx, problem, user_name);
    }

    let session_id = format!(
        "session-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );

    Some(CoachState::new(
        req_tx,
        evt_rx,
        session_id,
        config.stall_threshold_secs,
        config.max_interventions,
    ))
}

/// In-memory observation state tracked by the coach thread
struct LiveObservation {
    id: String,
    #[allow(dead_code)]
    route_idx: usize,
    title: String,
    #[allow(dead_code)]
    description: String,
    hints: HintsFile,
    #[allow(dead_code)]
    skill_tag: Option<String>,
    state: ObservationState,
}

/// Flatten all observations from a ProblemFile into LiveObservations
fn flatten_observations(problem: &ProblemFile) -> Vec<LiveObservation> {
    let problem_id = problem.id();
    let mut obs = Vec::new();
    for (ri, route) in problem.routes.iter().enumerate() {
        for (oi, o) in route.observations.iter().enumerate() {
            obs.push(LiveObservation {
                id: format!("{}:route:{}:obs:{}", problem_id, ri + 1, oi + 1),
                route_idx: ri,
                title: o.title.clone(),
                description: o.description.clone(),
                hints: o.hints.clone(),
                skill_tag: o.skill_tag.clone(),
                state: ObservationState::Locked,
            });
        }
    }
    obs
}

/// Build CoachingPromptContext from in-memory data
fn build_context_from_memory(
    problem: &ProblemFile,
    observations: &[LiveObservation],
    user_name: &str,
    code: &str,
    trigger: &str,
    elapsed_secs: u64,
    recent_messages: &[(String, String)],
) -> CoachingPromptContext {
    // Use the first route by default (most problems have one)
    let route = problem.routes.first();

    let obs_statuses: Vec<ObservationStatus> = observations
        .iter()
        .map(|o| ObservationStatus {
            id: o.id.clone(),
            title: o.title.clone(),
            description: o.description.clone(),
            state: o.state.as_str().to_string(),
        })
        .collect();

    CoachingPromptContext {
        user_name: user_name.to_string(),
        problem_title: problem.title.clone(),
        problem_difficulty: Some(problem.difficulty),
        problem_description: problem.description.clone(),
        route_name: route.map(|r| r.name.clone()),
        route_description: route.map(|r| r.description.clone()),
        observations: obs_statuses,
        code: code.to_string(),
        trigger: trigger.to_string(),
        recent_messages: recent_messages.to_vec(),
        elapsed_secs,
    }
}

/// Find prompts/ directory relative to CWD or cargo manifest
fn find_prompts_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("prompts"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("prompts"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn spawn_mock_thread(
    req_rx: mpsc::Receiver<CoachRequest>,
    evt_tx: mpsc::Sender<CoachEvent>,
    problem: ProblemFile,
) {
    std::thread::spawn(move || {
        let mut call_count: usize = 0;
        let observations = flatten_observations(&problem);

        while let Ok(request) = req_rx.recv() {
            match request {
                CoachRequest::Quit => break,
                CoachRequest::Analyze { trigger, .. } => {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    let response = mock_analyze_response(call_count, &trigger);
                    call_count += 1;
                    let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                }
                CoachRequest::UserMessage { message, .. } => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let response = mock_user_message_response(&message);
                    call_count += 1;
                    let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                }
                CoachRequest::RequestHint { hint_count, .. } => {
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // Pick an observation to hint based on hint_count
                    let obs_idx = hint_count.min(observations.len().saturating_sub(1));
                    let obs = &observations[obs_idx.min(observations.len().saturating_sub(1))];

                    let (message, ghost) = if hint_count <= 2 {
                        // Nudge level
                        (obs.hints.nudge.clone(), None)
                    } else if hint_count <= 4 {
                        // More direct
                        (obs.hints.question.clone(), None)
                    } else {
                        // Near-complete
                        (obs.hints.formal.clone(), Some(format!("# {}", obs.title)))
                    };

                    let response = CoachResponse {
                        state: "approaching".to_string(),
                        confidence: 0.4,
                        matched_observation_id: Some(obs.id.clone()),
                        coach_message: message,
                        ghost_text: ghost,
                        ghost_format: Some(GhostFormat::Code),
                        next_action: Some(format!("Think about: {}", obs.title)),
                    };
                    call_count += 1;
                    let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                }
            }
        }
    });
}

fn mock_analyze_response(call_count: usize, trigger: &str) -> CoachResponse {
    let responses = [
        CoachResponse {
            state: "approaching".to_string(),
            confidence: 0.6,
            matched_observation_id: Some("mock:obs:1".to_string()),
            coach_message: "You're reading the input correctly. What data structure would help you find pairs efficiently?".to_string(),
            ghost_text: Some("# consider using a dictionary".to_string()),
            ghost_format: Some(GhostFormat::Code),
            next_action: Some("Think about lookup time".to_string()),
        },
        CoachResponse {
            state: "uncertain".to_string(),
            confidence: 0.3,
            matched_observation_id: None,
            coach_message: "I notice you've been thinking for a while. What's your current approach to handling the constraints?".to_string(),
            ghost_text: None,
            ghost_format: None,
            next_action: Some("Consider the time limit".to_string()),
        },
        CoachResponse {
            state: "approaching".to_string(),
            confidence: 0.75,
            matched_observation_id: Some("mock:obs:2".to_string()),
            coach_message: "Good direction! You're close to the key insight. What happens if you track what you've already seen?".to_string(),
            ghost_text: Some("seen = set()".to_string()),
            ghost_format: Some(GhostFormat::Code),
            next_action: Some("Build on the set idea".to_string()),
        },
        CoachResponse {
            state: "found".to_string(),
            confidence: 0.9,
            matched_observation_id: Some("mock:obs:1".to_string()),
            coach_message: "That's the right idea! Using a hash map gives O(1) lookups. Now think about what to store as keys vs values.".to_string(),
            ghost_text: Some("complement = target - num".to_string()),
            ghost_format: Some(GhostFormat::Code),
            next_action: Some("Implement the complement check".to_string()),
        },
        CoachResponse {
            state: "moving_away".to_string(),
            confidence: 0.4,
            matched_observation_id: None,
            coach_message: "Careful -- a nested loop here would be O(n^2). Is there a way to avoid checking every pair?".to_string(),
            ghost_text: Some("what if you remembered previous values?".to_string()),
            ghost_format: Some(GhostFormat::Natural),
            next_action: Some("Revisit the data structure choice".to_string()),
        },
        CoachResponse {
            state: "found".to_string(),
            confidence: 0.85,
            matched_observation_id: Some("mock:obs:3".to_string()),
            coach_message: "Nice! Single-pass with a dictionary is exactly right. Don't forget edge cases -- what if the same element appears twice?".to_string(),
            ghost_text: None,
            ghost_format: None,
            next_action: Some("Handle duplicate values".to_string()),
        },
    ];

    if trigger.contains("test") && trigger.contains("failed") {
        return CoachResponse {
            state: "uncertain".to_string(),
            confidence: 0.5,
            matched_observation_id: None,
            coach_message: "Test failed -- check your output format. Are you printing just the answer with no extra whitespace?".to_string(),
            ghost_text: Some("print(result)".to_string()),
            ghost_format: Some(GhostFormat::Code),
            next_action: Some("Compare expected vs actual output carefully".to_string()),
        };
    }

    responses[call_count % responses.len()].clone()
}

fn mock_user_message_response(message: &str) -> CoachResponse {
    let lower = message.to_lowercase();
    if lower.contains("stuck") || lower.contains("help") {
        CoachResponse {
            state: "uncertain".to_string(),
            confidence: 0.3,
            matched_observation_id: None,
            coach_message: "Let's step back. What's the simplest version of this problem you can solve? Start there and build up.".to_string(),
            ghost_text: None,
            ghost_format: None,
            next_action: Some("Simplify the problem first".to_string()),
        }
    } else if lower.contains("tle") || lower.contains("slow") || lower.contains("time") {
        CoachResponse {
            state: "moving_away".to_string(),
            confidence: 0.5,
            matched_observation_id: None,
            coach_message: "Think about your current time complexity. Can you reduce the number of iterations by using a smarter data structure?".to_string(),
            ghost_text: Some("# O(n) is possible here".to_string()),
            ghost_format: Some(GhostFormat::Code),
            next_action: Some("Analyze your loop structure".to_string()),
        }
    } else {
        CoachResponse {
            state: "approaching".to_string(),
            confidence: 0.5,
            matched_observation_id: None,
            coach_message: "Interesting thought. Try writing down the invariant your solution maintains -- what's always true after processing element i?".to_string(),
            ghost_text: None,
            ghost_format: None,
            next_action: Some("Define your loop invariant".to_string()),
        }
    }
}

fn spawn_llm_thread(
    provider: OpenAiCompatibleProvider,
    req_rx: mpsc::Receiver<CoachRequest>,
    evt_tx: mpsc::Sender<CoachEvent>,
    problem: ProblemFile,
    user_name: String,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = evt_tx.send(CoachEvent::Error {
                    message: format!("Failed to create runtime: {}", e),
                });
                return;
            }
        };

        let mut observations = flatten_observations(&problem);
        let mut recent_messages: Vec<(String, String)> = Vec::new();
        let prompts_dir = find_prompts_dir();

        let _ = evt_tx.send(CoachEvent::Debug {
            message: format!(
                "llm thread started: {} observations, model via {}",
                observations.len(),
                if prompts_dir.is_some() { "prompts/" } else { "compiled-in" },
            ),
        });

        while let Ok(request) = req_rx.recv() {
            match request {
                CoachRequest::Quit => break,
                CoachRequest::Analyze {
                    code,
                    trigger,
                    elapsed_secs,
                } => {
                    let _ = evt_tx.send(CoachEvent::Debug {
                        message: format!(
                            "analyze: trigger=\"{}\" code={}b elapsed={}s",
                            trigger,
                            code.len(),
                            elapsed_secs,
                        ),
                    });

                    let ctx = build_context_from_memory(
                        &problem,
                        &observations,
                        &user_name,
                        &code,
                        &trigger,
                        elapsed_secs,
                        &recent_messages,
                    );
                    let (system_prompt, user_message) =
                        build_coaching_prompt(&ctx, prompts_dir.as_deref());

                    let _ = evt_tx.send(CoachEvent::Debug {
                        message: format!(
                            "llm call: system={}b user={}b max_tokens=1024",
                            system_prompt.len(),
                            user_message.len(),
                        ),
                    });

                    // Use higher token limit to accommodate reasoning models
                    // that spend tokens on chain-of-thought before producing output
                    let t0 = std::time::Instant::now();
                    let result = rt.block_on(provider.complete(CompletionRequest {
                        system_prompt,
                        user_message,
                        max_tokens: 2048,
                        temperature: Some(0.7),
                    }));

                    let elapsed_ms = t0.elapsed().as_millis();

                    match result {
                        Ok(raw) => {
                            let _ = evt_tx.send(CoachEvent::Debug {
                                message: format!(
                                    "llm response: {}ms, {}b",
                                    elapsed_ms,
                                    raw.len(),
                                ),
                            });
                            let response = parse_coach_response(&raw);
                            // Update observation state based on response
                            update_observations(&mut observations, &response);
                            // Track conversation
                            recent_messages.push(("coach".to_string(), response.coach_message.clone()));
                            if recent_messages.len() > 20 {
                                recent_messages.remove(0);
                            }
                            let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                        }
                        Err(e) => {
                            let _ = evt_tx.send(CoachEvent::Error {
                                message: format!("LLM error: {:#}", e),
                            });
                        }
                    }
                }
                CoachRequest::UserMessage { message, code, elapsed_secs } => {
                    let _ = evt_tx.send(CoachEvent::Debug {
                        message: format!("user msg: \"{}\"", &message[..message.len().min(60)]),
                    });

                    recent_messages.push(("user".to_string(), message.clone()));
                    if recent_messages.len() > 20 {
                        recent_messages.remove(0);
                    }

                    let ctx = build_context_from_memory(
                        &problem,
                        &observations,
                        &user_name,
                        &code,
                        "user message",
                        elapsed_secs,
                        &recent_messages,
                    );
                    let (system_prompt, _) =
                        build_coaching_prompt(&ctx, prompts_dir.as_deref());

                    let _ = evt_tx.send(CoachEvent::Debug {
                        message: format!(
                            "llm call (msg): system={}b user={}b max_tokens=512",
                            system_prompt.len(),
                            message.len(),
                        ),
                    });

                    let t0 = std::time::Instant::now();
                    let result = rt.block_on(provider.complete(CompletionRequest {
                        system_prompt,
                        user_message: message,
                        max_tokens: 512,
                        temperature: Some(0.7),
                    }));
                    let elapsed_ms = t0.elapsed().as_millis();

                    match result {
                        Ok(raw) => {
                            let _ = evt_tx.send(CoachEvent::Debug {
                                message: format!("llm response (msg): {}ms, {}b", elapsed_ms, raw.len()),
                            });
                            let response = parse_coach_response(&raw);
                            update_observations(&mut observations, &response);
                            recent_messages.push(("coach".to_string(), response.coach_message.clone()));
                            if recent_messages.len() > 20 {
                                recent_messages.remove(0);
                            }
                            let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                        }
                        Err(e) => {
                            let _ = evt_tx.send(CoachEvent::Error {
                                message: format!("LLM error: {:#}", e),
                            });
                        }
                    }
                }
                CoachRequest::RequestHint { code, elapsed_secs, hint_count } => {
                    recent_messages.push(("user".to_string(), "I'm stuck, can I get a hint?".to_string()));
                    if recent_messages.len() > 20 {
                        recent_messages.remove(0);
                    }

                    // Collect ALL precomputed hints as reference material for the LLM
                    let hints_ref = build_hints_reference(&observations);

                    let escalation = if hint_count <= 2 {
                        "Give a gentle nudge. Ask a guiding question about the most relevant unfound observation."
                    } else if hint_count <= 4 {
                        "Be more direct. Name the relevant concept or technique. Ask a pointed question."
                    } else {
                        "Give a near-complete explanation of the next key insight. Describe the approach clearly."
                    };

                    let ctx = build_context_from_memory(
                        &problem,
                        &observations,
                        &user_name,
                        &code,
                        &format!("hint requested (#{hint_count})"),
                        elapsed_secs,
                        &recent_messages,
                    );
                    let (system_prompt, user_message) =
                        build_coaching_prompt(&ctx, prompts_dir.as_deref());

                    let hint_prompt = format!(
                        "{}\n\nThe user requested hint #{hint_count}. Here are all available hints for reference:\n\
                        {hints_ref}\n\
                        {escalation}\n\
                        Pick the hint most relevant to the user's current code and approach. \
                        If they've already figured out some observations, focus on what they haven't found yet.",
                        user_message,
                    );

                    let _ = evt_tx.send(CoachEvent::Debug {
                        message: format!(
                            "hint llm call: #{} system={}b user={}b",
                            hint_count, system_prompt.len(), hint_prompt.len(),
                        ),
                    });

                    let t0 = std::time::Instant::now();
                    let result = rt.block_on(provider.complete(CompletionRequest {
                        system_prompt,
                        user_message: hint_prompt,
                        max_tokens: 512,
                        temperature: Some(0.7),
                    }));
                    let elapsed_ms = t0.elapsed().as_millis();

                    let response = match result {
                        Ok(raw) => {
                            let _ = evt_tx.send(CoachEvent::Debug {
                                message: format!("hint llm response: {}ms, {}b", elapsed_ms, raw.len()),
                            });
                            let resp = parse_coach_response(&raw);
                            update_observations(&mut observations, &resp);
                            recent_messages.push(("coach".to_string(), resp.coach_message.clone()));
                            if recent_messages.len() > 20 {
                                recent_messages.remove(0);
                            }
                            resp
                        }
                        Err(_) => {
                            // Fallback: pick a nudge from the first locked observation
                            let fallback_obs = observations
                                .iter()
                                .find(|o| o.state != ObservationState::Found);
                            if let Some(obs) = fallback_obs {
                                CoachResponse {
                                    state: "approaching".to_string(),
                                    confidence: 0.4,
                                    matched_observation_id: Some(obs.id.clone()),
                                    coach_message: obs.hints.nudge.clone(),
                                    ghost_text: None,
                                    ghost_format: None,
                                    next_action: Some(format!("Think about: {}", obs.title)),
                                }
                            } else {
                                CoachResponse {
                                    state: "uncertain".to_string(),
                                    confidence: 0.0,
                                    matched_observation_id: None,
                                    coach_message: "Try putting the pieces together.".to_string(),
                                    ghost_text: None,
                                    ghost_format: None,
                                    next_action: Some("Implement your solution".to_string()),
                                }
                            }
                        }
                    };
                    let _ = evt_tx.send(CoachEvent::CoachMessage { response });
                }
            }
        }
    });
}

/// Build a text block with all precomputed hints for LLM reference
fn build_hints_reference(observations: &[LiveObservation]) -> String {
    let mut text = String::new();
    for obs in observations {
        text.push_str(&format!(
            "- [{}] \"{}\"\n  nudge: \"{}\"\n  question: \"{}\"\n  formal: \"{}\"\n",
            obs.state.as_str(),
            obs.title,
            obs.hints.nudge,
            obs.hints.question,
            obs.hints.formal,
        ));
    }
    text
}

/// Update observation states based on LLM response
fn update_observations(observations: &mut [LiveObservation], response: &CoachResponse) {
    if let Some(ref obs_id) = response.matched_observation_id {
        if let Some(obs) = observations.iter_mut().find(|o| o.id == *obs_id) {
            let new_state = ObservationState::parse(&response.state);
            // Only advance state, never regress
            match (&obs.state, &new_state) {
                (ObservationState::Locked, ObservationState::Approaching)
                | (ObservationState::Locked, ObservationState::Found)
                | (ObservationState::Approaching, ObservationState::Found) => {
                    obs.state = new_state;
                }
                _ => {}
            }
        }
    }
}
