use std::collections::HashSet;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::models::AppError;

use super::super::runtime::{append_mission_recovery_log, list_worker_handles};

pub(in crate::commands::mission) fn recover_mission(
    orch: &Orchestrator<'_>,
    emitter: Option<&MissionEventEmitter>,
    project_path: &std::path::Path,
    mission_id: &str,
) -> Result<(), AppError> {
    let current_state = orch.get_state()?;
    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    match current_state.state {
        MissionState::Paused => return Ok(()),
        MissionState::Completed | MissionState::Cancelled | MissionState::Failed => {
            return Err(AppError::invalid_argument("mission already completed"));
        }
        MissionState::Running | MissionState::Initializing | MissionState::OrchestratorTurn => {}
        _ => {
            return Err(AppError::invalid_argument(
                "mission recover requires mission to be running or initializing",
            ));
        }
    }

    if !list_worker_handles(mission_id).is_empty() {
        return Err(AppError::invalid_argument(
            "mission has active workers; cannot recover while running",
        ));
    }

    let state_doc = artifacts::read_state(project_path, mission_id)?;
    let mut rollback_ids: HashSet<String> = state_doc
        .assignments
        .values()
        .map(|a| a.feature_id.clone())
        .collect();
    if let Some(fid) = state_doc.current_feature_id.clone() {
        rollback_ids.insert(fid);
    }

    // Also roll back any feature that is still marked in_progress.
    let features_doc = orch.get_features()?;
    for feature in &features_doc.features {
        if feature.status == FeatureStatus::InProgress {
            rollback_ids.insert(feature.id.clone());
        }
    }

    for feature_id in rollback_ids {
        if orch
            .update_feature_status(&feature_id, FeatureStatus::Pending)
            .is_ok()
        {
            if let Some(emitter) = emitter {
                let _ = emitter.features_changed(&feature_id, "pending");
            }
        }
    }

    orch.transition(MissionState::Paused)?;
    if let Some(emitter) = emitter {
        emitter.state_changed(&old_state_str, "paused")?;
    }

    // Clear any persisted worker state (after crash/restart, handles are lost).
    if let Ok(mut doc) = artifacts::read_state(project_path, mission_id) {
        doc.worker_pids.clear();
        doc.assignments.clear();
        doc.current_worker_id = None;
        doc.current_feature_id = None;
        doc.updated_at = chrono::Utc::now().timestamp_millis();
        let _ = artifacts::write_state(project_path, mission_id, &doc);
    }

    super::super::macro_commands::update_macro_stage_on_lifecycle(
        project_path,
        mission_id,
        crate::mission::macro_types::MacroStage::Blocked,
        emitter,
    );

    append_mission_recovery_log(
        project_path,
        mission_id,
        "mission recovered from fake running",
    );

    Ok(())
}
