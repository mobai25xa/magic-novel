//! Shared LLM router construction helpers.
//!
//! Keeps provider/router wiring consistent between main agent and mission workers.

use std::sync::Arc;

use super::providers::anthropic::AnthropicProvider;
use super::providers::gemini::GeminiProvider;
use super::providers::openai_chat::OpenAiChatProvider;
use super::router::{LlmRouter, RetryConfig};

pub fn build_router(
    provider: &str,
    base_url: String,
    api_key: String,
    retry_config: RetryConfig,
) -> Arc<LlmRouter> {
    let mut router = LlmRouter::new(retry_config);

    match provider {
        "anthropic" => {
            router.register(
                Arc::new(AnthropicProvider::new(base_url, api_key)),
                vec!["anthropic".to_string()],
            );
        }
        "gemini" => {
            router.register(
                Arc::new(GeminiProvider::new(base_url, api_key)),
                vec!["gemini".to_string()],
            );
        }
        _ => {
            router.register(
                Arc::new(OpenAiChatProvider::new(base_url, api_key)),
                vec!["openai-compatible".to_string(), "openai".to_string()],
            );
        }
    }

    Arc::new(router)
}
