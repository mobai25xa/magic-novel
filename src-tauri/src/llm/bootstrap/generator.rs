use std::sync::Arc;

use async_trait::async_trait;

use crate::models::AppError;

use super::{
    live::LlmBootstrapGenerator, template::TemplateBootstrapGenerator, BootstrapArtifactKind,
    BootstrapGenerationResult, BootstrapPromptInput,
};

#[async_trait]
pub trait BootstrapGenerator: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate(
        &self,
        input: BootstrapPromptInput,
        requested_kinds: Vec<BootstrapArtifactKind>,
    ) -> Result<BootstrapGenerationResult, AppError>;
}

pub fn default_generator() -> Arc<dyn BootstrapGenerator> {
    if llm_bootstrap_enabled() {
        match LlmBootstrapGenerator::from_local_settings() {
            Ok(Some(generator)) => return Arc::new(generator),
            Ok(None) => tracing::warn!(
                target: "bootstrap::generator",
                "LLM bootstrap requested but provider settings are incomplete; falling back to template generator"
            ),
            Err(error) => tracing::warn!(
                target: "bootstrap::generator",
                error = %error,
                "failed to load LLM bootstrap settings; falling back to template generator"
            ),
        }
    }

    Arc::new(TemplateBootstrapGenerator)
}

fn llm_bootstrap_enabled() -> bool {
    match std::env::var("MAGIC_BOOTSTRAP_USE_LLM") {
        Ok(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes" | "enabled"
        ),
        Err(_) => false,
    }
}
