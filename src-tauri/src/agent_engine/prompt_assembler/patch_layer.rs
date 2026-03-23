//! Layer E: Model Patch — provider/model-specific adjustments.
//!
//! Only handles known differences. No large-scale rewrites.

/// Provider/model descriptor used to select a patch.
#[derive(Debug, Clone)]
pub struct ModelPatch {
    pub provider: String,
    pub model: String,
}

impl ModelPatch {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }
}

/// Render provider/model-specific patch text.
/// Returns empty string for unknown providers (no patch needed).
pub fn render_patch(patch: &ModelPatch) -> String {
    match patch.provider.as_str() {
        "openai-compatible" => OPENAI_PATCH.to_string(),
        _ => String::new(),
    }
}

const OPENAI_PATCH: &str = r#"## Output format note
- Do not wrap responses in markdown fences unless producing code.
- Do not inject XML tags into plain prose output.
- When outputting JSON tool arguments, use compact format."#;
