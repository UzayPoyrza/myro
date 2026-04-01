use crate::types::CoachResponse;

/// Find the largest byte index <= max_bytes that is a valid char boundary.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// Parse a coach response from LLM output.
/// Tries: direct JSON parse -> extract from markdown code fences -> fallback
pub fn parse_coach_response(raw: &str) -> CoachResponse {
    // Try direct JSON parse
    if let Ok(resp) = serde_json::from_str::<CoachResponse>(raw) {
        return resp;
    }

    // Try extracting JSON from markdown code fences
    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(resp) = serde_json::from_str::<CoachResponse>(&json_str) {
            return resp;
        }
    }

    // Try finding JSON object in the text
    if let Some(json_str) = extract_json_object(raw) {
        if let Ok(resp) = serde_json::from_str::<CoachResponse>(&json_str) {
            return resp;
        }
    }

    // Fallback: use the raw text as the coach message
    fallback_response(raw)
}

pub fn extract_json_block(text: &str) -> Option<String> {
    let start_markers = ["```json\n", "```json\r\n", "```\n", "```\r\n"];
    for marker in start_markers {
        if let Some(start) = text.find(marker) {
            let content_start = start + marker.len();
            if let Some(end) = text[content_start..].find("```") {
                return Some(text[content_start..content_start + end].trim().to_string());
            }
        }
    }
    None
}

pub fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn fallback_response(raw: &str) -> CoachResponse {
    CoachResponse {
        state: "uncertain".to_string(),
        confidence: 0.0,
        matched_observation_id: None,
        coach_message: if raw.len() > 500 {
            let end = truncate_at_char_boundary(raw, 497);
            format!("{}...", &raw[..end])
        } else if raw.is_empty() {
            "Let me think about that differently...".to_string()
        } else {
            raw.to_string()
        },
        ghost_text: None,
        ghost_format: None,
        next_action: Some("Describe your current thinking about the problem".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_json() {
        let json = r#"{"state":"found","confidence":0.9,"matched_observation_id":"obs1","coach_message":"Great!","ghost_text":null,"ghost_format":null,"next_action":null}"#;
        let resp = parse_coach_response(json);
        assert_eq!(resp.state, "found");
        assert!((resp.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(resp.matched_observation_id, Some("obs1".to_string()));
        assert_eq!(resp.coach_message, "Great!");
    }

    #[test]
    fn test_parse_json_in_code_fence() {
        let raw = r#"Here is my analysis:

```json
{"state":"approaching","confidence":0.7,"matched_observation_id":null,"coach_message":"You're on the right track.","ghost_text":null,"ghost_format":null,"next_action":"Keep going"}
```

That's my response."#;
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "approaching");
        assert_eq!(resp.coach_message, "You're on the right track.");
    }

    #[test]
    fn test_parse_json_object_in_text() {
        let raw = r#"I think the response should be {"state":"moving_away","confidence":0.4,"matched_observation_id":null,"coach_message":"Consider a different approach.","ghost_text":null,"ghost_format":null,"next_action":null} based on the code."#;
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "moving_away");
        assert_eq!(resp.coach_message, "Consider a different approach.");
    }

    #[test]
    fn test_fallback_on_garbage() {
        let raw = "This is not JSON at all, just natural text from the LLM.";
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "uncertain");
        assert_eq!(resp.confidence, 0.0);
        assert_eq!(resp.coach_message, raw);
    }

    #[test]
    fn test_fallback_on_empty() {
        let resp = parse_coach_response("");
        assert_eq!(resp.coach_message, "Let me think about that differently...");
    }

    #[test]
    fn test_fallback_truncates_long_text() {
        let raw = "x".repeat(600);
        let resp = parse_coach_response(&raw);
        assert!(resp.coach_message.len() <= 500);
        assert!(resp.coach_message.ends_with("..."));
    }

    #[test]
    fn test_nested_braces() {
        let raw = r#"Some text {"state":"found","confidence":0.85,"matched_observation_id":"obs:1","coach_message":"The nested {thing} works.","ghost_text":null,"ghost_format":null,"next_action":null} end"#;
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "found");
        assert_eq!(resp.coach_message, "The nested {thing} works.");
    }

    #[test]
    fn test_parse_realistic_llm_response() {
        // Real response format from local 20B model
        let raw = r#"{"state":"approaching","confidence":0.8,"matched_observation_id":"cf:1A:route:1:obs:2","coach_message":"You are using math.ceil with floating division; have you considered how to compute the ceiling without using floating-point arithmetic?","ghost_text":null,"ghost_format":"natural","next_action":"Think about integer operations."}"#;
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "approaching");
        assert!((resp.confidence - 0.8).abs() < f64::EPSILON);
        assert_eq!(
            resp.matched_observation_id,
            Some("cf:1A:route:1:obs:2".to_string())
        );
        assert!(resp.coach_message.contains("floating"));
        assert!(resp.ghost_text.is_none());
    }

    #[test]
    fn test_parse_with_unicode() {
        // Models sometimes use unicode dashes, smart quotes, etc.
        let raw = r#"{"state":"found","confidence":0.95,"matched_observation_id":"cf:1A:route:1:obs:3","coach_message":"What happens if n, m, a are all 10\u2079?","ghost_text":null,"ghost_format":null,"next_action":null}"#;
        let resp = parse_coach_response(raw);
        assert_eq!(resp.state, "found");
        assert_eq!(
            resp.matched_observation_id,
            Some("cf:1A:route:1:obs:3".to_string())
        );
    }
}
