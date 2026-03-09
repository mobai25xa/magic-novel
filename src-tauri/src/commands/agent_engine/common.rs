use crate::agent_engine::types::{
    AgentMode, ApprovalMode, ClarificationMode, LoopConfig, DEFAULT_MODEL,
};
use crate::models::AppError;

use super::core::AgentTurnStartInput;

fn require_non_empty(
    value: Option<String>,
    code: &'static str,
    field: &'static str,
) -> Result<String, AppError> {
    let normalized = value.map(|v| v.trim().to_string()).unwrap_or_default();
    if normalized.is_empty() {
        return Err(AppError {
            code: crate::models::ErrorCode::InvalidArgument,
            message: format!("AI setting '{field}' is missing"),
            details: Some(serde_json::json!({ "code": code })),
            recoverable: Some(true),
        });
    }
    Ok(normalized)
}

pub(super) fn resolve_turn_provider_config(
    input: &AgentTurnStartInput,
) -> Result<(String, String, String), AppError> {
    let base_url = require_non_empty(
        input.base_url.clone(),
        "E_AI_SETTINGS_MISSING_BASEURL",
        "base_url",
    )?;
    let api_key = require_non_empty(
        input.api_key.clone(),
        "E_AI_SETTINGS_MISSING_APIKEY",
        "api_key",
    )?;
    let model = input
        .model
        .clone()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    Ok((base_url, api_key, model))
}

pub(super) fn build_loop_config(input: &AgentTurnStartInput) -> LoopConfig {
    let capability_mode = input.capability_mode.unwrap_or(AgentMode::Writing);
    let approval_mode = input.approval_mode.unwrap_or(ApprovalMode::ConfirmWrites);
    let clarification_mode = input
        .clarification_mode
        .unwrap_or(ClarificationMode::Interactive);

    LoopConfig {
        capability_mode,
        approval_mode,
        clarification_mode,
        autonomy_level: approval_mode.to_autonomy_level(),
        ..LoopConfig::default()
    }
}
