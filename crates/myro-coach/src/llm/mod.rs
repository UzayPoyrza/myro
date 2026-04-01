pub mod openai_compat;

use anyhow::Result;
use std::future::Future;

pub struct CompletionRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

pub trait LlmProvider: Send + Sync {
    fn complete(&self, request: CompletionRequest) -> impl Future<Output = Result<String>> + Send;
}
