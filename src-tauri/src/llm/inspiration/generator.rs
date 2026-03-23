use std::sync::Arc;

use async_trait::async_trait;

use crate::application::command_usecases::inspiration::{
    GenerateMetadataVariantsOutput, ResolvedConsensusSnapshot,
};
use crate::models::AppError;

use super::{live::LlmMetadataVariantGenerator, template::TemplateMetadataVariantGenerator};

#[async_trait]
pub trait MetadataVariantGenerator: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate(
        &self,
        input: ResolvedConsensusSnapshot,
    ) -> Result<GenerateMetadataVariantsOutput, AppError>;
}

pub fn default_generator() -> Arc<dyn MetadataVariantGenerator> {
    if llm_generator_enabled() {
        match LlmMetadataVariantGenerator::from_local_settings() {
            Ok(Some(generator)) => return Arc::new(generator),
            Ok(None) => tracing::warn!(
                target: "inspiration::generator",
                "LLM metadata variants requested but provider settings are incomplete; falling back to template generator"
            ),
            Err(error) => tracing::warn!(
                target: "inspiration::generator",
                error = %error,
                "failed to load inspiration LLM settings; falling back to template generator"
            ),
        }
    }

    Arc::new(TemplateMetadataVariantGenerator)
}

fn llm_generator_enabled() -> bool {
    match std::env::var("MAGIC_INSPIRATION_USE_LLM") {
        Ok(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes" | "enabled"
        ),
        Err(_) => false,
    }
}
