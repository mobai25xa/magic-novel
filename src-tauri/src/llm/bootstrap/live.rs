use async_trait::async_trait;
use futures::StreamExt;

use crate::agent_engine::messages::AgentMessage;
use crate::llm::accumulator::StreamAccumulator;
use crate::llm::provider::new_cancel_token;
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::types::{LlmRequest, SystemBlock, ToolChoice};
use crate::models::AppError;
use crate::services::load_openai_search_settings;

use super::{
    build_system_prompt, build_user_prompt, BootstrapArtifactKind, BootstrapCreativePayload,
    BootstrapGenerationResult, BootstrapGenerator, BootstrapPromptInput,
};

pub struct LlmBootstrapGenerator {
    provider: String,
    model: String,
    base_url: String,
    api_key: String,
}

impl LlmBootstrapGenerator {
    pub fn from_local_settings() -> Result<Option<Self>, AppError> {
        let settings = load_openai_search_settings()?;
        if settings.openai_base_url.trim().is_empty() || settings.openai_api_key.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(Self {
            provider: "openai-compatible".to_string(),
            model: settings.openai_model,
            base_url: settings.openai_base_url,
            api_key: settings.openai_api_key,
        }))
    }
}

#[async_trait]
impl BootstrapGenerator for LlmBootstrapGenerator {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn generate(
        &self,
        input: BootstrapPromptInput,
        requested_kinds: Vec<BootstrapArtifactKind>,
    ) -> Result<BootstrapGenerationResult, AppError> {
        let router = build_router(
            &self.provider,
            self.base_url.clone(),
            self.api_key.clone(),
            RetryConfig::worker(),
        );
        let request = LlmRequest {
            provider_name: self.provider.clone(),
            model: self.model.clone(),
            system: vec![SystemBlock {
                text: build_system_prompt(),
                cache_control: None,
            }],
            messages: vec![AgentMessage::user(build_user_prompt(
                &input,
                &requested_kinds,
            )?)],
            tools: Vec::new(),
            tool_choice: ToolChoice::None,
            parallel_tool_calls: false,
            temperature: 0.2,
            reasoning: None,
        };

        let (_cancel_tx, cancel_rx) = new_cancel_token();
        let mut stream = router
            .stream_chat(request, cancel_rx)
            .await
            .map_err(AppError::from)?;
        let mut accumulator = StreamAccumulator::new();
        while let Some(event) = stream.next().await {
            let event = event.map_err(AppError::from)?;
            accumulator.apply(&event);
        }

        let output = accumulator.into_turn_output()?;
        if !output.tool_calls.is_empty() {
            return Err(AppError::invalid_argument(
                "bootstrap LLM response must not contain tool calls",
            ));
        }

        let payload = parse_bootstrap_payload(&output.assistant_message.text_content())?;
        Ok(payload.materialize(&requested_kinds, self.name()))
    }
}

fn parse_bootstrap_payload(raw: &str) -> Result<BootstrapCreativePayload, AppError> {
    let cleaned = strip_code_fences(raw).trim().to_string();
    let candidate = extract_json_object(&cleaned).unwrap_or(cleaned);
    serde_json::from_str::<BootstrapCreativePayload>(&candidate).map_err(|error| {
        AppError::invalid_argument(format!("bootstrap payload parse failed: {error}"))
    })
}

fn strip_code_fences(raw: &str) -> String {
    raw.replace("```json", "").replace("```", "")
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].to_string())
}
