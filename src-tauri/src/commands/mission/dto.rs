//! Mission command DTOs and input validation helpers.

use serde::{Deserialize, Serialize};

use crate::agent_engine::types::{DEFAULT_MODEL, DEFAULT_PROVIDER};
use crate::mission::types::*;
use crate::models::{AppError, ErrorCode};

use super::{MissionRunConfig, MissionStartConfig};

const DEFAULT_MISSION_MAX_WORKERS: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionCreateInput {
    pub project_path: String,
    pub title: String,
    pub mission_text: String,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionCreateOutput {
    pub schema_version: i32,
    pub mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionListInput {
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionGetStatusInput {
    pub project_path: String,
    pub mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionGetStatusOutput {
    pub state: StateDoc,
    pub features: FeaturesDoc,
    pub handoffs: Vec<HandoffEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionStartInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub max_workers: Option<usize>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionControlInput {
    pub project_path: String,
    pub mission_id: String,
}

fn clamp_max_workers(max_workers: Option<usize>) -> usize {
    max_workers
        .map(|v| v.max(1))
        .unwrap_or(DEFAULT_MISSION_MAX_WORKERS)
}

fn require_non_empty(
    value: Option<String>,
    code: &'static str,
    field: &'static str,
) -> Result<String, AppError> {
    let normalized = value.map(|v| v.trim().to_string()).unwrap_or_default();
    if normalized.is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: format!("mission setting '{field}' is missing"),
            details: Some(serde_json::json!({ "code": code })),
            recoverable: Some(true),
        });
    }
    Ok(normalized)
}

pub(super) fn resolve_start_config(
    input: &MissionStartInput,
) -> Result<MissionStartConfig, AppError> {
    let base_url = require_non_empty(
        input.base_url.clone(),
        "E_MISSION_SETTINGS_MISSING_BASEURL",
        "base_url",
    )?;
    let api_key = require_non_empty(
        input.api_key.clone(),
        "E_MISSION_SETTINGS_MISSING_APIKEY",
        "api_key",
    )?;
    let model = input
        .model
        .clone()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let provider = input
        .provider
        .clone()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_PROVIDER.to_string());

    Ok(MissionStartConfig {
        run_config: MissionRunConfig {
            model,
            provider,
            base_url,
            api_key,
        },
        max_workers: clamp_max_workers(input.max_workers),
    })
}
