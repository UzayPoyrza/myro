use std::path::Path;

pub struct CoachingPromptContext {
    pub user_name: String,
    pub problem_title: String,
    pub problem_difficulty: Option<i32>,
    pub problem_description: String,
    pub route_name: Option<String>,
    pub route_description: Option<String>,
    pub observations: Vec<ObservationStatus>,
    pub code: String,
    pub trigger: String,
    pub recent_messages: Vec<(String, String)>,
    pub elapsed_secs: u64,
}

pub struct ObservationStatus {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
}

/// Compiled-in default templates (used when files not found)
const DEFAULT_SYSTEM_TEMPLATE: &str = include_str!("../../../../prompts/coaching-system.md");
const DEFAULT_USER_TEMPLATE: &str = include_str!("../../../../prompts/coaching-user.md");

/// Load a template from disk, falling back to compiled-in default
fn load_template(prompts_dir: Option<&Path>, filename: &str, default: &str) -> String {
    if let Some(dir) = prompts_dir {
        let path = dir.join(filename);
        if let Ok(content) = std::fs::read_to_string(&path) {
            return content;
        }
    }
    default.to_string()
}

/// Build the system prompt and user message for real-time coaching interventions.
/// Returns (system_prompt, user_message).
///
/// If `prompts_dir` is Some, loads templates from that directory.
/// Falls back to compiled-in defaults if files are not found.
pub fn build_coaching_prompt(
    ctx: &CoachingPromptContext,
    prompts_dir: Option<&Path>,
) -> (String, String) {
    let system_template = load_template(prompts_dir, "coaching-system.md", DEFAULT_SYSTEM_TEMPLATE);
    let user_template = load_template(prompts_dir, "coaching-user.md", DEFAULT_USER_TEMPLATE);

    let system_prompt = fill_system_template(&system_template, ctx);
    let user_message = fill_user_template(&user_template, ctx);
    (system_prompt, user_message)
}

fn fill_system_template(template: &str, ctx: &CoachingPromptContext) -> String {
    let observations_text = format_observations(&ctx.observations);
    let difficulty_str = ctx
        .problem_difficulty
        .map(|d| d.to_string())
        .unwrap_or_else(|| "?".to_string());

    let desc = truncate_description(&ctx.problem_description, 1500);

    let mut result = template.to_string();
    result = result.replace("{{user_name}}", &ctx.user_name);
    result = result.replace("{{problem_title}}", &ctx.problem_title);
    result = result.replace("{{problem_difficulty}}", &difficulty_str);
    result = result.replace("{{problem_description}}", &desc);
    result = result.replace(
        "{{route_name}}",
        ctx.route_name.as_deref().unwrap_or("(auto-detect)"),
    );
    result = result.replace(
        "{{route_description}}",
        ctx.route_description.as_deref().unwrap_or(""),
    );
    result = result.replace("{{observations}}", &observations_text);
    result = result.replace("{{elapsed_secs}}", &ctx.elapsed_secs.to_string());
    result
}

fn fill_user_template(template: &str, ctx: &CoachingPromptContext) -> String {
    let recent = format_recent_messages(&ctx.recent_messages);
    let code = truncate_code(&ctx.code, 3000);

    let mut result = template.to_string();
    result = result.replace("{{trigger}}", &ctx.trigger);
    result = result.replace("{{recent_messages}}", &recent);
    result = result.replace("{{code}}", &code);
    result
}

fn format_observations(observations: &[ObservationStatus]) -> String {
    let mut text = String::new();
    for obs in observations {
        let state_marker = match obs.state.as_str() {
            "found" => "[FOUND]",
            "approaching" => "[APPROACHING]",
            _ => "[LOCKED]",
        };
        text.push_str(&format!("- {} `{}`: {}\n", state_marker, obs.id, obs.title));
        if !obs.description.is_empty() {
            text.push_str(&format!("  What it means: {}\n", obs.description));
        }
    }
    text
}

fn format_recent_messages(messages: &[(String, String)]) -> String {
    if messages.is_empty() {
        return String::new();
    }
    let mut text = String::from("**Recent conversation**:\n");
    for (role, content) in messages {
        let label = if role == "user" { "User" } else { "Coach" };
        text.push_str(&format!("{}: {}\n", label, content));
    }
    text.push('\n');
    text
}

fn truncate_description(desc: &str, max_bytes: usize) -> String {
    if desc.len() <= max_bytes {
        desc.to_string()
    } else {
        let mut end = max_bytes;
        while end > 0 && !desc.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...[truncated]", &desc[..end])
    }
}

fn truncate_code(code: &str, max_chars: usize) -> String {
    if code.len() <= max_chars {
        code.to_string()
    } else {
        let lines: Vec<&str> = code.lines().collect();
        let mut result = String::new();
        for line in &lines {
            if result.len() + line.len() + 1 > max_chars {
                result.push_str("\n# ... [code truncated] ...");
                break;
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context() -> CoachingPromptContext {
        CoachingPromptContext {
            user_name: "Alice".to_string(),
            problem_title: "Two Sum".to_string(),
            problem_difficulty: Some(1200),
            problem_description: "Given an array of integers...".to_string(),
            route_name: Some("Hash map approach".to_string()),
            route_description: Some("Use a hash map to find complements".to_string()),
            observations: vec![
                ObservationStatus {
                    id: "obs:1".to_string(),
                    title: "Complement lookup".to_string(),
                    description: "For each element, check if target - element exists in a hash map.".to_string(),
                    state: "locked".to_string(),
                },
                ObservationStatus {
                    id: "obs:2".to_string(),
                    title: "Single pass".to_string(),
                    description: "You can check and insert in one pass through the array.".to_string(),
                    state: "approaching".to_string(),
                },
            ],
            code: "n = int(input())\narr = list(map(int, input().split()))".to_string(),
            trigger: "user idle for 45s".to_string(),
            recent_messages: vec![
                ("user".to_string(), "I'm thinking about using a nested loop".to_string()),
                ("coach".to_string(), "What's the time complexity of that approach?".to_string()),
            ],
            elapsed_secs: 120,
        }
    }

    #[test]
    fn test_build_coaching_prompt_contains_key_elements() {
        let ctx = sample_context();
        let (system, user) = build_coaching_prompt(&ctx, None);

        // System prompt checks
        assert!(system.contains("Two Sum"));
        assert!(system.contains("1200"));
        assert!(system.contains("Hash map approach"));
        assert!(system.contains("[LOCKED]"));
        assert!(system.contains("[APPROACHING]"));
        assert!(system.contains("coaching"));
        assert!(system.contains("JSON"));
        assert!(system.contains("obs:1"));
        assert!(system.contains("obs:2"));
        assert!(system.contains("Alice"));

        // Observation descriptions should be included
        assert!(
            system.contains("For each element, check if target"),
            "Observation descriptions should be in the prompt"
        );
        assert!(
            system.contains("one pass through the array"),
            "Observation descriptions should be in the prompt"
        );

        // User message checks
        assert!(user.contains("user idle for 45s"));
        assert!(user.contains("nested loop"));
        assert!(user.contains("n = int(input())"));
    }

    #[test]
    fn test_truncate_description() {
        let short = "short text";
        assert_eq!(truncate_description(short, 100), short);

        let long = "x".repeat(200);
        let truncated = truncate_description(&long, 100);
        assert!(truncated.len() < 200);
        assert!(truncated.contains("[truncated]"));
    }

    #[test]
    fn test_prompt_includes_observation_descriptions() {
        // Simulate Theatre Square with real observation structure
        let ctx = CoachingPromptContext {
            user_name: "Noob".to_string(),
            problem_title: "Theatre Square".to_string(),
            problem_difficulty: Some(1000),
            problem_description: "Cover n x m with a x a tiles.".to_string(),
            route_name: Some("Ceiling division".to_string()),
            route_description: Some("Use ceiling division per dimension.".to_string()),
            observations: vec![
                ObservationStatus {
                    id: "cf:1A:route:1:obs:1".to_string(),
                    title: "Independent dimensions".to_string(),
                    description: "Tiles along width and height computed independently, then multiplied.".to_string(),
                    state: "locked".to_string(),
                },
                ObservationStatus {
                    id: "cf:1A:route:1:obs:2".to_string(),
                    title: "Ceiling division without floats".to_string(),
                    description: "ceil(n/a) = (n + a - 1) / a in integer arithmetic.".to_string(),
                    state: "approaching".to_string(),
                },
            ],
            code: "n, m, a = map(int, input().split())".to_string(),
            trigger: "user idle for 45s".to_string(),
            recent_messages: vec![],
            elapsed_secs: 60,
        };

        let (system, _) = build_coaching_prompt(&ctx, None);

        // Verify observation IDs are exact (for LLM to copy-paste)
        assert!(system.contains("`cf:1A:route:1:obs:1`"));
        assert!(system.contains("`cf:1A:route:1:obs:2`"));

        // Verify descriptions are included (critical for LLM to match code to observations)
        assert!(system.contains("Tiles along width and height computed independently"));
        assert!(system.contains("ceil(n/a) = (n + a - 1) / a"));

        // Verify state markers
        assert!(system.contains("[LOCKED] `cf:1A:route:1:obs:1`"));
        assert!(system.contains("[APPROACHING] `cf:1A:route:1:obs:2`"));

        // Verify prompt structure includes coaching philosophy
        assert!(system.contains("Help Calibration"));
        assert!(system.contains("coaching"));
    }

    #[test]
    fn test_minimal_context() {
        let ctx = CoachingPromptContext {
            user_name: "User".to_string(),
            problem_title: "Test".to_string(),
            problem_difficulty: None,
            problem_description: String::new(),
            route_name: None,
            route_description: None,
            observations: vec![
                ObservationStatus {
                    id: "test:obs:1".to_string(),
                    title: "Test obs".to_string(),
                    description: String::new(),
                    state: "locked".to_string(),
                },
            ],
            code: String::new(),
            trigger: "user requested help".to_string(),
            recent_messages: vec![],
            elapsed_secs: 0,
        };
        let (system, user) = build_coaching_prompt(&ctx, None);
        assert!(system.contains("Test"));
        assert!(user.contains("user requested help"));
    }
}
