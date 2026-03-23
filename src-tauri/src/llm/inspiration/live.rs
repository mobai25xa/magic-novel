use async_trait::async_trait;
use futures::StreamExt;

use crate::agent_engine::messages::AgentMessage;
use crate::application::command_usecases::inspiration::{
    GenerateMetadataVariantsOutput, ResolvedConsensusSnapshot,
};
use crate::llm::accumulator::StreamAccumulator;
use crate::llm::provider::new_cancel_token;
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::types::{LlmRequest, SystemBlock, ToolChoice};
use crate::models::AppError;
use crate::services::load_openai_search_settings;

use super::generator::MetadataVariantGenerator;
use super::prompt::{build_system_prompt, build_user_prompt};

pub struct LlmMetadataVariantGenerator {
    provider: String,
    model: String,
    base_url: String,
    api_key: String,
}

impl LlmMetadataVariantGenerator {
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
impl MetadataVariantGenerator for LlmMetadataVariantGenerator {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn generate(
        &self,
        input: ResolvedConsensusSnapshot,
    ) -> Result<GenerateMetadataVariantsOutput, AppError> {
        let router = build_router(
            &self.provider,
            self.base_url.clone(),
            self.api_key.clone(),
            RetryConfig::interactive(),
        );
        let request = LlmRequest {
            provider_name: self.provider.clone(),
            model: self.model.clone(),
            system: vec![SystemBlock {
                text: build_system_prompt(),
                cache_control: None,
            }],
            messages: vec![AgentMessage::user(build_user_prompt(&input)?)],
            tools: Vec::new(),
            tool_choice: ToolChoice::None,
            parallel_tool_calls: false,
            temperature: 0.35,
            reasoning: None,
        };

        let (_cancel_tx, cancel_rx) = new_cancel_token();
        let mut stream = router
            .stream_chat(request, cancel_rx)
            .await
            .map_err(AppError::from)?;
        let mut accumulator = StreamAccumulator::new();
        while let Some(event) = stream.next().await {
            accumulator.apply(&event.map_err(AppError::from)?);
        }

        let output = accumulator.into_turn_output()?;
        if !output.tool_calls.is_empty() {
            return Err(AppError::invalid_argument(
                "metadata variants LLM response must not contain tool calls",
            ));
        }

        parse_variants_payload(&output.assistant_message.text_content())
    }
}

fn parse_variants_payload(raw: &str) -> Result<GenerateMetadataVariantsOutput, AppError> {
    let cleaned = raw.replace("```json", "").replace("```", "");
    let candidate = extract_json_object(&cleaned).unwrap_or(cleaned.as_str());
    serde_json::from_str::<GenerateMetadataVariantsOutput>(candidate.trim()).map_err(|error| {
        AppError::invalid_argument(format!("metadata variants payload parse failed: {error}"))
    })
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        None
    } else {
        Some(&raw[start..=end])
    }
}
