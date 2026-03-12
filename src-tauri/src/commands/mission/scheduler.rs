//! Mission scheduling and worker supervision logic.

use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::WorkerProcess;
use crate::mission::types::Feature;
use crate::mission::worker_profile::WorkerProfile;
use crate::models::AppError;

use super::{review_gate, runtime, MissionRunConfig, MissionStartConfig};

mod core;
mod supervision;

pub(super) async fn spawn_and_initialize_worker(
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
) -> Result<(String, Arc<TokioMutex<WorkerProcess>>), AppError> {
    core::spawn_and_initialize_worker(project_path, project_path_str, mission_id).await
}

pub(super) async fn schedule_ready_features(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    mission_id: &str,
    project_path: &std::path::Path,
    project_path_str: &str,
    start_config: &MissionStartConfig,
    emit_orchestrator_transition: bool,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, AppError> {
    core::schedule_ready_features(
        orch,
        emitter,
        mission_id,
        project_path,
        project_path_str,
        start_config,
        emit_orchestrator_transition,
        app_handle,
    )
    .await
}

pub(super) async fn start_feature_on_worker(
    orch: &Orchestrator<'_>,
    worker: &WorkerProcess,
    emitter: &MissionEventEmitter,
    project_path: &std::path::Path,
    mission_id: &str,
    feature: Feature,
    run_config: &MissionRunConfig,
    worker_id: &str,
    attempt: u32,
    worker_profile: WorkerProfile,
    rollback_to_pending_on_start_error: bool,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    core::start_feature_on_worker(
        orch,
        worker,
        emitter,
        project_path,
        mission_id,
        feature,
        run_config,
        worker_id,
        attempt,
        worker_profile,
        rollback_to_pending_on_start_error,
        emit_orchestrator_transition,
    )
    .await
}

pub(super) fn spawn_worker_supervision_tasks(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id_bg: String,
    run_config_bg: MissionRunConfig,
    max_workers_bg: usize,
) {
    supervision::spawn_worker_supervision_tasks(
        app_handle,
        mission_id,
        project_path_bg,
        worker_id_bg,
        run_config_bg,
        max_workers_bg,
    )
}
