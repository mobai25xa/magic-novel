//! Mission Knowledge Writeback Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::knowledge::{types as knowledge_types, writeback as knowledge_writeback};
use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::models::AppError;

use super::runtime::acquire_mission_runtime_lock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeGetLatestInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_knowledge_get_latest(
    input: MissionKnowledgeGetLatestInput,
) -> Result<knowledge_types::MissionKnowledgeLatest, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?;
    Ok(knowledge_types::MissionKnowledgeLatest { bundle, delta })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeDecideInput {
    pub project_path: String,
    pub mission_id: String,
    pub decision: knowledge_types::KnowledgeDecisionInput,
}

#[command]
pub async fn mission_knowledge_decide(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeDecideInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge proposal bundle found"))?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge delta found"))?;

    let updated = knowledge_writeback::apply_decision_to_delta(&bundle, delta, &input.decision)?;

    artifacts::write_knowledge_delta_latest(project_path, &input.mission_id, &updated)?;
    let _ = artifacts::append_knowledge_delta(project_path, &input.mission_id, &updated);
    if updated.conflicts.is_empty() {
        let _ = artifacts::clear_pending_knowledge_decision(project_path, &input.mission_id);
    }

    // Best-effort event emission.
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    if !updated.conflicts.is_empty() {
        let _ = emitter.knowledge_decision_required(&updated);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeApplyInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_knowledge_apply(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeApplyInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge proposal bundle found"))?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge delta found"))?;

    let applied = knowledge_writeback::apply_accepted(project_path, &input.mission_id, &bundle, &delta)?;

    artifacts::write_knowledge_delta_latest(project_path, &input.mission_id, &applied)?;
    let _ = artifacts::append_knowledge_delta(project_path, &input.mission_id, &applied);
    let _ = artifacts::clear_pending_knowledge_decision(project_path, &input.mission_id);

    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    let _ = emitter.knowledge_applied(&applied);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeRollbackInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub token: Option<String>,
}

#[command]
pub async fn mission_knowledge_rollback(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeRollbackInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let token = input
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)
                .ok()
                .flatten()
                .and_then(|d| d.rollback)
                .and_then(|rb| rb.token)
        })
        .ok_or_else(|| AppError::invalid_argument("rollback token is required"))?;

    let (restored, deleted) = knowledge_writeback::rollback(project_path, &input.mission_id, &token)?;

    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    let _ = emitter.knowledge_rolled_back(&token, restored, deleted);

    Ok(())
}
