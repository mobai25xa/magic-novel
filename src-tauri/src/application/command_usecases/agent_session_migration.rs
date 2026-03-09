use serde::{Deserialize, Serialize};
use tauri::command;

use crate::application::command_usecases::agent_session_support::resolve_project_path;
use crate::models::{AppError, ErrorCode};
use crate::services::{
    commit_session_migration, dry_run_session_migration, rollback_session_migration,
    SessionMigrationCommitOutput, SessionMigrationDryRunOutput, SessionMigrationRollbackOutput,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionMigrationDryRunInput {
    pub project_path: String,
    #[serde(default)]
    pub session_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionMigrationCommitInput {
    pub project_path: String,
    #[serde(default)]
    pub session_ids: Option<Vec<String>>,
    #[serde(default)]
    pub batch_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionMigrationRollbackInput {
    pub project_path: String,
    pub migration_id: String,
    #[serde(default)]
    pub batch_id: Option<u32>,
}

#[command]
pub async fn agent_session_migration_dry_run(
    input: AgentSessionMigrationDryRunInput,
) -> Result<SessionMigrationDryRunOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    let session_ids = normalize_session_ids(input.session_ids);
    dry_run_session_migration(project_path.as_path(), session_ids.as_deref())
}

#[command]
pub async fn agent_session_migration_commit(
    input: AgentSessionMigrationCommitInput,
) -> Result<SessionMigrationCommitOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    let batch_size = input.batch_size.unwrap_or(50).max(1);
    let session_ids = normalize_session_ids(input.session_ids);

    commit_session_migration(project_path.as_path(), session_ids.as_deref(), batch_size)
}

#[command]
pub async fn agent_session_migration_rollback(
    input: AgentSessionMigrationRollbackInput,
) -> Result<SessionMigrationRollbackOutput, AppError> {
    if input.migration_id.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "migration_id is required".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_MIGRATION_INVALID_MIGRATION_ID",
            })),
            recoverable: Some(true),
        });
    }

    let project_path = resolve_project_path(&input.project_path)?;
    rollback_session_migration(
        project_path.as_path(),
        input.migration_id.trim(),
        input.batch_id,
    )
}

fn normalize_session_ids(session_ids: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut normalized = session_ids
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<String>>();

    if normalized.is_empty() {
        return None;
    }

    normalized.sort();
    normalized.dedup();
    Some(normalized)
}
