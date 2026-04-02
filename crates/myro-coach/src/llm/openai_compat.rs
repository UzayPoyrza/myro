use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, LlmProvider};

/// Send a lightweight test request to verify the API key and model work.
/// Returns Ok(()) on success, Err with a user-friendly message on failure.
pub fn test_connection(base_url: &str, api_key: Option<&str>, model: &str) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Say hi"}],
        "max_tokens": 3,
    });

    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().context("connection failed")?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        // Try to extract a short error message from JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(msg) = json["error"]["message"].as_str() {
                anyhow::bail!("{}", msg);
            }
        }
        anyhow::bail!("HTTP {}", status);
    }

    Ok(())
}

pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(0) // fresh connection each request — avoids stale conn RST
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    pub fn from_config(config: &crate::config::CoachConfig) -> Option<Self> {
        let base_url = config.base_url.as_ref()?;
        Some(Self::new(
            base_url.clone(),
            config.api_key.clone(),
            config
                .model
                .clone()
                .unwrap_or_else(|| "anthropic/claude-sonnet-4".to_string()),
        ))
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(default)]
    content: String,
    /// Reasoning models (e.g. DeepSeek) put their output here instead of content
    #[serde(default, skip_serializing)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl LlmProvider for OpenAiCompatibleProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: request.system_prompt,
                    reasoning_content: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: request.user_message,
                    reasoning_content: None,
                },
            ],
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let mut req = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.context("LLM request send failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, body);
        }

        let chat_resp: ChatResponse = resp.json().await.context("Failed to parse LLM response")?;
        chat_resp
            .choices
            .into_iter()
            .next()
            .map(|c| {
                let text = c.message.content.trim().to_string();
                if !text.is_empty() {
                    // Prefer content field — this is the actual response
                    text
                } else if let Some(reasoning) = c.message.reasoning_content {
                    // Some reasoning models (DeepSeek-R1) put their answer
                    // in reasoning_content. Only use it if it looks like
                    // valid JSON (our expected format), not raw CoT.
                    let trimmed = reasoning.trim();
                    if trimmed.starts_with('{') {
                        trimmed.to_string()
                    } else {
                        // Raw chain-of-thought, not a usable response
                        String::new()
                    }
                } else {
                    String::new()
                }
            })
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Empty response from LLM (model may have exhausted tokens on reasoning)"))
    }
}
