use std::collections::HashSet;
use std::time::Duration;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::models::AppError;

use super::super::runtime::{
    append_mission_recovery_log, clear_worker_from_state, list_worker_handles, remove_worker_handle,
};
use super::super::scheduler;

pub(in crate::commands::mission) async fn interrupt_mission(
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

    if matches!(
        current_state.state,
        MissionState::Completed | MissionState::Cancelled | MissionState::Failed
    ) {
        return Err(AppError::invalid_argument("mission already completed"));
    }

    if !matches!(
        current_state.state,
        MissionState::Running | MissionState::Initializing | MissionState::OrchestratorTurn
    ) {
        return Err(AppError::invalid_argument("mission is not running"));
    }

    let state_doc = artifacts::read_state(project_path, mission_id)?;
    let mut running_feature_ids: HashSet<String> = state_doc
        .assignments
        .values()
        .map(|a| a.feature_id.clone())
        .collect();
    if let Some(feature_id) = state_doc.current_feature_id.clone() {
        running_feature_ids.insert(feature_id);
    }

    for worker_id in scheduler::cancel_in_process_delegates(mission_id, None, true) {
        clear_worker_from_state(project_path, mission_id, &worker_id);
        append_mission_recovery_log(
            project_path,
            mission_id,
            format!("mission_interrupt requested in-process delegate stop for {worker_id}"),
        );
    }

    for (worker_id, worker_entry) in list_worker_handles(mission_id) {
        let worker = worker_entry.worker.lock().await;
        if let Err(e) = worker.kill(Duration::from_secs(2)).await {
            tracing::warn!(
                target: "mission",
                mission_id = %mission_id,
                worker_id = %worker_id,
                error = %e,
                "failed to stop worker during interrupt (may have already exited)"
            );
            append_mission_recovery_log(
                project_path,
                mission_id,
                format!("mission_interrupt worker stop failed for {worker_id}: {e}"),
            );
        }
        clear_worker_from_state(project_path, mission_id, &worker_id);
        let _ = remove_worker_handle(mission_id, &worker_id);
    }

    // Also clear any persisted worker entries (e.g. after crash/restart where handles are lost).
    for worker_id in state_doc.worker_pids.keys() {
        clear_worker_from_state(project_path, mission_id, worker_id);
        let _ = remove_worker_handle(mission_id, worker_id);
    }

    // Roll back any in-progress feature(s) to pending so the mission can be safely resumed.
    let features_doc = orch.get_features()?;
    for feature in &features_doc.features {
        if feature.status == FeatureStatus::InProgress {
            running_feature_ids.insert(feature.id.clone());
        }
    }

    for feature_id in running_feature_ids {
        if let Err(e) = orch.update_feature_status(&feature_id, FeatureStatus::Pending) {
            append_mission_recovery_log(
                project_path,
                mission_id,
                format!(
                    "mission_interrupt failed to roll back feature {feature_id} to pending: {e}"
                ),
            );
            continue;
        }
        if let Some(emitter) = emitter {
            let _ = emitter.features_changed(&feature_id, "pending");
        }
    }

    orch.transition(MissionState::Paused)?;
    if let Some(emitter) = emitter {
        emitter.state_changed(&old_state_str, "paused")?;
    }

    super::super::macro_commands::update_macro_stage_on_lifecycle(
        project_path,
        mission_id,
        crate::mission::macro_types::MacroStage::Blocked,
        emitter,
    );

    append_mission_recovery_log(project_path, mission_id, "mission interrupted by user");

    Ok(())
}
