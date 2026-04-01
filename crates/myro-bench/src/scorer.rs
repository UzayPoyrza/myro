use std::collections::{HashMap, HashSet};

use crate::runner::{RunResult, TurnRunResult};
use crate::scenario::ScenarioExpect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResult {
    pub score: u32,
    pub max_score: u32,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub check: String,
    pub message: String,
    pub points_lost: u32,
}

pub fn score_result(result: &RunResult, expect: &ScenarioExpect) -> ScoreResult {
    let mut score: u32 = 0;
    let mut max_score: u32 = 0;
    let mut issues = Vec::new();

    let resp = &result.parsed;

    // 1. Valid parse (2 pts) — check the raw response contains parseable JSON
    max_score += 2;
    if is_valid_json_response(&result.raw_response) {
        score += 2;
    } else {
        issues.push(Issue {
            check: "valid_parse".to_string(),
            message: "Response did not contain valid JSON (fallback used)".to_string(),
            points_lost: 2,
        });
    }

    // 2. State match (2 pts)
    max_score += 2;
    let state_ok = check_state(&resp.state, expect);
    if state_ok {
        score += 2;
    } else {
        let detail = if let Some(ref expected) = expect.state {
            format!("expected '{}', got '{}'", expected, resp.state)
        } else if !expect.valid_states.is_empty() {
            format!("expected one of {:?}, got '{}'", expect.valid_states, resp.state)
        } else {
            format!("'{}' is in banned states {:?}", resp.state, expect.banned_states)
        };
        issues.push(Issue {
            check: "state_match".to_string(),
            message: format!("State mismatch: {}", detail),
            points_lost: 2,
        });
    }

    // 3. Confidence range (1 pt)
    max_score += 1;
    let min = expect.min_confidence.unwrap_or(0.0);
    let max_c = expect.max_confidence.unwrap_or(1.0);
    if resp.confidence >= min && resp.confidence <= max_c {
        score += 1;
    } else {
        issues.push(Issue {
            check: "confidence_range".to_string(),
            message: format!(
                "Confidence {:.2} outside [{:.2}, {:.2}]",
                resp.confidence, min, max_c,
            ),
            points_lost: 1,
        });
    }

    // 4. Observation ID match (2 pts, only if expected)
    if expect.obs_id.is_some() {
        max_score += 2;
        if resp.matched_observation_id == expect.obs_id {
            score += 2;
        } else {
            issues.push(Issue {
                check: "obs_id_match".to_string(),
                message: format!(
                    "Obs ID mismatch: expected {:?}, got {:?}",
                    expect.obs_id, resp.matched_observation_id,
                ),
                points_lost: 2,
            });
        }
    }

    // 5. Socratic check (1 pt)
    if expect.must_be_socratic {
        max_score += 1;
        if resp.coach_message.contains('?') {
            score += 1;
        } else {
            issues.push(Issue {
                check: "socratic".to_string(),
                message: "Coach message contains no question mark".to_string(),
                points_lost: 1,
            });
        }
    }

    // 6. Answer leak detection (2 pts)
    if !expect.banned_patterns.is_empty() {
        max_score += 2;
        let lower_msg = resp.coach_message.to_lowercase();
        let lower_ghost = resp
            .ghost_text
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let leaked: Vec<&str> = expect
            .banned_patterns
            .iter()
            .filter(|p| {
                let lp = p.to_lowercase();
                lower_msg.contains(&lp) || lower_ghost.contains(&lp)
            })
            .map(|s| s.as_str())
            .collect();
        if leaked.is_empty() {
            score += 2;
        } else {
            issues.push(Issue {
                check: "no_leaks".to_string(),
                message: format!("Answer leak detected: {:?}", leaked),
                points_lost: 2,
            });
        }
    }

    ScoreResult {
        score,
        max_score,
        issues,
    }
}

fn check_state(actual: &str, expect: &ScenarioExpect) -> bool {
    // Exact match
    if let Some(ref expected) = expect.state {
        if actual == expected {
            return true;
        }
        return false;
    }

    // Valid states list
    if !expect.valid_states.is_empty() && !expect.valid_states.iter().any(|s| s == actual) {
        return false;
    }

    // Banned states
    if expect.banned_states.iter().any(|s| s == actual) {
        return false;
    }

    true
}

fn is_valid_json_response(raw: &str) -> bool {
    // Check if the raw response contains parseable JSON with the required fields
    let candidate = raw.trim();

    // Try direct parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
        return has_coach_fields(&v);
    }

    // Try extracting from code fence
    if let Some(block) = extract_fenced_json(candidate) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&block) {
            return has_coach_fields(&v);
        }
    }

    // Try finding embedded JSON object
    if let Some(obj) = extract_embedded_json(candidate) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&obj) {
            return has_coach_fields(&v);
        }
    }

    false
}

fn has_coach_fields(v: &serde_json::Value) -> bool {
    v.get("state").is_some() && v.get("coach_message").is_some()
}

fn extract_fenced_json(text: &str) -> Option<String> {
    let start = text.find("```json").or_else(|| text.find("```"))?;
    let content_start = text[start..].find('\n')? + start + 1;
    let end = text[content_start..].find("```")? + content_start;
    Some(text[content_start..end].trim().to_string())
}

fn extract_embedded_json(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0;
    let bytes = text.as_bytes();
    for i in start..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Trajectory-level scoring for multi-turn scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryScore {
    pub score: u32,
    pub max_score: u32,
    pub issues: Vec<Issue>,
}

/// Score a multi-turn trajectory across all turns.
///
/// Checks:
/// - Progression (2 pts): observation unlock count is non-decreasing across turns
/// - No contradiction (2 pts): no observation goes from "found" to "moving_away"
/// - No repetition (1 pt): consecutive coach messages are not near-duplicates (word Jaccard < 0.8)
pub fn score_trajectory(turn_results: &[TurnRunResult]) -> TrajectoryScore {
    let mut score: u32 = 0;
    let max_score: u32 = 5;
    let mut issues = Vec::new();

    if turn_results.is_empty() {
        return TrajectoryScore {
            score: 0,
            max_score,
            issues,
        };
    }

    // 1. Progression (2 pts): found count non-decreasing
    let mut progression_ok = true;
    let mut prev_found = count_found(&turn_results[0].obs_states_after);
    for tr in &turn_results[1..] {
        let cur_found = count_found(&tr.obs_states_after);
        if cur_found < prev_found {
            progression_ok = false;
            break;
        }
        prev_found = cur_found;
    }
    if progression_ok {
        score += 2;
    } else {
        issues.push(Issue {
            check: "trajectory_progression".to_string(),
            message: "Observation found count decreased across turns".to_string(),
            points_lost: 2,
        });
    }

    // 2. No contradiction (2 pts): no obs goes from "found" to response state "moving_away"
    let mut contradiction = false;
    let mut found_obs: HashSet<String> = HashSet::new();
    // Seed from first turn's initial state (obs_states_after minus any changes)
    // Actually check across turns: if an obs was "found" after turn N, it shouldn't
    // get state "moving_away" in a later turn's response
    for tr in turn_results {
        if let Some(ref obs_id) = tr.parsed.matched_observation_id {
            if found_obs.contains(obs_id) && tr.parsed.state == "moving_away" {
                contradiction = true;
                break;
            }
        }
        // Track which obs became found after this turn
        for (idx, state) in &tr.obs_states_after {
            if state == "found" {
                // Reconstruct obs_id from index (we don't have problem_id here, so use the
                // matched_observation_id pattern to infer)
                found_obs.insert(idx.clone());
            }
        }
    }
    // Also check using actual observation IDs from responses
    let mut found_by_id: HashSet<String> = HashSet::new();
    for tr in turn_results {
        if let Some(ref obs_id) = tr.parsed.matched_observation_id {
            if found_by_id.contains(obs_id) && tr.parsed.state == "moving_away" {
                contradiction = true;
            }
            if tr.parsed.state == "found" {
                found_by_id.insert(obs_id.clone());
            }
        }
    }
    if !contradiction {
        score += 2;
    } else {
        issues.push(Issue {
            check: "trajectory_no_contradiction".to_string(),
            message: "Observation went from found to moving_away across turns".to_string(),
            points_lost: 2,
        });
    }

    // 3. No repetition (1 pt): consecutive messages word Jaccard < 0.8
    let mut repetition = false;
    for pair in turn_results.windows(2) {
        let j = word_jaccard(&pair[0].parsed.coach_message, &pair[1].parsed.coach_message);
        if j >= 0.8 {
            repetition = true;
            break;
        }
    }
    if !repetition {
        score += 1;
    } else {
        issues.push(Issue {
            check: "trajectory_no_repetition".to_string(),
            message: "Consecutive coach messages are near-duplicates (Jaccard >= 0.8)".to_string(),
            points_lost: 1,
        });
    }

    TrajectoryScore {
        score,
        max_score,
        issues,
    }
}

fn count_found(obs_states: &HashMap<String, String>) -> usize {
    obs_states.values().filter(|s| *s == "found").count()
}

fn word_jaccard(a: &str, b: &str) -> f64 {
    let words_a: HashSet<&str> = a.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric())).filter(|w| !w.is_empty()).collect();
    let words_b: HashSet<&str> = b.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric())).filter(|w| !w.is_empty()).collect();
    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }
    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}
