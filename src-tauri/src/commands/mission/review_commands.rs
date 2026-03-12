//! Mission ReviewGate Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::models::AppError;
use crate::review::types as review_types;

use super::review_gate::*;
use super::runtime::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionReviewGetLatestInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_review_get_latest(
    input: MissionReviewGetLatestInput,
) -> Result<Option<review_types::ReviewReport>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::read_review_latest(project_path, &input.mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionReviewListInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_review_list(
    input: MissionReviewListInput,
) -> Result<Vec<review_types::ReviewReport>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::read_review_reports(project_path, &input.mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionReviewGetPendingDecisionInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_review_get_pending_decision(
    input: MissionReviewGetPendingDecisionInput,
) -> Result<Option<review_types::ReviewDecisionRequest>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::read_pending_review_decision(project_path, &input.mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionReviewAnswerInput {
    pub project_path: String,
    pub mission_id: String,
    pub answer: review_types::ReviewDecisionAnswer,
}

#[command]
pub async fn mission_review_answer(
    app_handle: tauri::AppHandle,
    input: MissionReviewAnswerInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle.clone(), input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let pending = artifacts::read_pending_review_decision(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::invalid_argument("no pending review decision"))?;

    if pending.review_id != input.answer.review_id {
        return Err(AppError::invalid_argument(
            "review_id mismatch for decision answer",
        ));
    }

    append_mission_recovery_log(
        project_path,
        &input.mission_id,
        format!(
            "review decision answered: review_id={} option={}",
            input.answer.review_id, input.answer.selected_option
        ),
    );

    if input.answer.selected_option.trim() == "auto_fix" {
        let Some(feature_id) = pending.feature_id.clone().filter(|s| !s.trim().is_empty()) else {
            return Err(AppError::invalid_argument(
                "pending decision missing feature_id for auto_fix",
            ));
        };

        let start_cfg = paused_config_registry()
            .get(&input.mission_id)
            .map(|cfg| cfg.clone())
            .ok_or_else(|| AppError::invalid_argument("missing mission run config"))?;

        let latest = artifacts::read_review_latest(project_path, &input.mission_id)?
            .ok_or_else(|| AppError::invalid_argument("no latest review report found"))?;

        if latest.review_id != pending.review_id {
            return Err(AppError::invalid_argument(
                "latest review_id mismatch; cannot auto_fix",
            ));
        }

        start_review_fixup_attempt(
            app_handle,
            &orch,
            &emitter,
            &start_cfg,
            project_path,
            &input.project_path,
            &input.mission_id,
            &feature_id,
            &latest,
        )
        .await?;
    }

    // Clear pending decision after processing.
    artifacts::clear_pending_review_decision(project_path, &input.mission_id)?;

    Ok(())
}
