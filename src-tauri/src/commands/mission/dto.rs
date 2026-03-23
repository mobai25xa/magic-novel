//! Mission command DTOs and input validation helpers.

use serde::{Deserialize, Serialize};

use crate::agent_engine::types::{DEFAULT_MODEL, DEFAULT_PROVIDER};
use crate::mission::blockers::WorkflowBlockersDoc;
use crate::mission::job_types::JobSnapshot;
use crate::mission::result_types::AgentTaskResult;
use crate::mission::types::*;
use crate::mission::workflow_types::{
    MissionWorkflowKind, SummaryJobPolicy, WorkflowCreationReason, WorkflowDoc,
};
use crate::models::{AppError, ErrorCode};

use super::{DelegateTransportMode, MissionRunConfig, MissionStartConfig};

const DEFAULT_MISSION_MAX_WORKERS: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionCreateInput {
    pub project_path: String,
    pub title: String,
    pub mission_text: String,
    pub features: Vec<Feature>,
    #[serde(default)]
    pub workflow_kind: Option<MissionWorkflowKind>,
    #[serde(default)]
    pub creation_reason: Option<WorkflowCreationReason>,
    #[serde(default)]
    pub summary_job_policy: Option<SummaryJobPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionCreateOutput {
    pub schema_version: i32,
    pub mission_id: String,
    pub workflow: WorkflowDoc,
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
    pub task_results: Vec<AgentTaskResult>,
    pub handoffs: Vec<HandoffEntry>,
    pub workflow: WorkflowDoc,
    pub blockers: WorkflowBlockersDoc,
    pub job_snapshot: JobSnapshot,
    pub recovery_log: Vec<super::runtime::MissionProgressLogEntry>,
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
    #[serde(default)]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub parent_turn_id: Option<u32>,
    #[serde(default)]
    pub delegate_transport: Option<DelegateTransportMode>,
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
    let parent_session_id = input
        .parent_session_id
        .clone()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let parent_turn_id = input.parent_turn_id.filter(|turn_id| *turn_id > 0);
    let delegate_transport = input.delegate_transport.unwrap_or_default();

    Ok(MissionStartConfig {
        run_config: MissionRunConfig {
            model,
            provider,
            base_url,
            api_key,
        },
        max_workers: clamp_max_workers(input.max_workers),
        parent_session_id,
        parent_turn_id,
        delegate_transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_start_input() -> MissionStartInput {
        MissionStartInput {
            project_path: "D:/project".to_string(),
            mission_id: "mis_chat_dispatch".to_string(),
            max_workers: Some(1),
            model: Some("gpt-4.1-mini".to_string()),
            provider: Some("openai-compatible".to_string()),
            base_url: Some("http://localhost:1234/v1".to_string()),
            api_key: Some("sk-test".to_string()),
            parent_session_id: None,
            parent_turn_id: None,
            delegate_transport: None,
        }
    }

    #[test]
    fn resolve_start_config_preserves_parent_linkage() {
        let mut input = base_start_input();
        input.parent_session_id = Some("  sess_123  ".to_string());
        input.parent_turn_id = Some(7);

        let config = resolve_start_config(&input).expect("config should resolve");

        assert_eq!(config.parent_session_id.as_deref(), Some("sess_123"));
        assert_eq!(config.parent_turn_id, Some(7));
    }

    #[test]
    fn resolve_start_config_drops_blank_or_zero_parent_linkage() {
        let mut input = base_start_input();
        input.parent_session_id = Some("   ".to_string());
        input.parent_turn_id = Some(0);

        let config = resolve_start_config(&input).expect("config should resolve");

        assert_eq!(config.parent_session_id, None);
        assert_eq!(config.parent_turn_id, None);
    }

    #[test]
    fn resolve_start_config_defaults_delegate_transport_to_process() {
        let input = base_start_input();

        let config = resolve_start_config(&input).expect("config should resolve");

        assert_eq!(config.delegate_transport, DelegateTransportMode::Process);
    }

    #[test]
    fn resolve_start_config_accepts_in_process_delegate_transport() {
        let mut input = base_start_input();
        input.delegate_transport = Some(DelegateTransportMode::InProcess);

        let config = resolve_start_config(&input).expect("config should resolve");

        assert_eq!(config.delegate_transport, DelegateTransportMode::InProcess);
    }
}
