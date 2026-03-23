//! Mission scheduling and worker supervision logic.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;

use dashmap::DashMap;

use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;

use super::{MissionRunConfig, MissionStartConfig};
use crate::mission::agent_profile::{AgentProfile, SessionSource};
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::WorkerProcess;
use crate::mission::types::Feature;
use crate::models::AppError;

mod core;
mod supervision;

#[derive(Debug, Clone)]
pub(super) struct InProcessDelegateControl {
    mission_id: String,
    worker_id: String,
    #[cfg_attr(not(test), allow(dead_code))]
    feature_id: String,
    cancel_token: CancellationToken,
    ignore_result: Arc<AtomicBool>,
}

type InProcessDelegateRegistry = DashMap<String, InProcessDelegateControl>;

fn in_process_delegate_registry() -> &'static InProcessDelegateRegistry {
    static REGISTRY: OnceLock<InProcessDelegateRegistry> = OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

fn in_process_delegate_key(mission_id: &str, worker_id: &str) -> String {
    format!("{}::{}", mission_id.trim(), worker_id.trim())
}

pub(super) fn register_in_process_delegate(
    mission_id: &str,
    worker_id: &str,
    feature_id: &str,
    cancel_token: CancellationToken,
    ignore_result: Arc<AtomicBool>,
) {
    let key = in_process_delegate_key(mission_id, worker_id);
    if let Some((_, existing)) = in_process_delegate_registry().remove(&key) {
        existing.ignore_result.store(true, Ordering::SeqCst);
        existing.cancel_token.cancel();
    }

    in_process_delegate_registry().insert(
        key,
        InProcessDelegateControl {
            mission_id: mission_id.trim().to_string(),
            worker_id: worker_id.trim().to_string(),
            feature_id: feature_id.trim().to_string(),
            cancel_token,
            ignore_result,
        },
    );
}

pub(super) fn unregister_in_process_delegate(
    mission_id: &str,
    worker_id: &str,
) -> Option<InProcessDelegateControl> {
    in_process_delegate_registry()
        .remove(&in_process_delegate_key(mission_id, worker_id))
        .map(|(_, control)| control)
}

pub(super) fn active_in_process_delegate_worker_ids(mission_id: &str) -> Vec<String> {
    let mission_id = mission_id.trim();
    let mut worker_ids = in_process_delegate_registry()
        .iter()
        .filter(|entry| entry.value().mission_id == mission_id)
        .map(|entry| entry.value().worker_id.clone())
        .collect::<Vec<_>>();
    worker_ids.sort();
    worker_ids.dedup();
    worker_ids
}

pub(super) fn has_active_in_process_delegates(mission_id: &str) -> bool {
    let mission_id = mission_id.trim();
    in_process_delegate_registry()
        .iter()
        .any(|entry| entry.value().mission_id == mission_id)
}

pub(super) fn cancel_in_process_delegates(
    mission_id: &str,
    exclude_worker_id: Option<&str>,
    ignore_result: bool,
) -> Vec<String> {
    let mission_id = mission_id.trim();
    let exclude_worker_id = exclude_worker_id.map(str::trim);
    let controls = in_process_delegate_registry()
        .iter()
        .filter(|entry| {
            let value = entry.value();
            value.mission_id == mission_id
                && exclude_worker_id
                    .map(|exclude| value.worker_id != exclude)
                    .unwrap_or(true)
        })
        .map(|entry| entry.value().clone())
        .collect::<Vec<_>>();

    let mut worker_ids = Vec::with_capacity(controls.len());
    for control in controls {
        if ignore_result {
            control.ignore_result.store(true, Ordering::SeqCst);
        }
        control.cancel_token.cancel();
        worker_ids.push(control.worker_id);
    }

    worker_ids.sort();
    worker_ids.dedup();
    worker_ids
}

pub(super) async fn spawn_and_initialize_worker(
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
) -> Result<(String, Arc<TokioMutex<WorkerProcess>>), AppError> {
    let worker_id = format!("wk_{}", uuid::Uuid::new_v4());
    let worker = core::spawn_and_initialize_worker(
        project_path,
        project_path_str,
        mission_id,
        Some(worker_id.clone()),
    )
    .await?;
    Ok((worker_id, worker))
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
    agent_profile: AgentProfile,
    session_source: SessionSource,
    parent_session_id: Option<&str>,
    parent_turn_id: Option<u32>,
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
        agent_profile,
        session_source,
        parent_session_id,
        parent_turn_id,
        rollback_to_pending_on_start_error,
        emit_orchestrator_transition,
    )
    .await
}

pub(super) async fn start_feature_in_process(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    project_path: &std::path::Path,
    mission_id: &str,
    feature: Feature,
    run_config: &MissionRunConfig,
    worker_id: &str,
    attempt: u32,
    agent_profile: AgentProfile,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    core::start_feature_in_process(
        orch,
        emitter,
        project_path,
        mission_id,
        feature,
        run_config,
        worker_id,
        attempt,
        agent_profile,
        emit_orchestrator_transition,
    )
    .await
}

pub(super) fn spawn_process_delegate_supervision_task(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id: String,
    worker: WorkerProcess,
    feature: Feature,
    agent_profile: AgentProfile,
    start_cfg_bg: MissionStartConfig,
    attempt: u32,
) {
    supervision::spawn_process_delegate_supervision_task(
        app_handle,
        mission_id,
        project_path_bg,
        worker_id,
        worker,
        feature,
        agent_profile,
        start_cfg_bg,
        attempt,
    )
}

pub(super) fn spawn_in_process_delegate_supervision_task(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id: String,
    feature: Feature,
    agent_profile: AgentProfile,
    start_cfg_bg: MissionStartConfig,
    attempt: u32,
) {
    supervision::spawn_in_process_delegate_supervision_task(
        app_handle,
        mission_id,
        project_path_bg,
        worker_id,
        feature,
        agent_profile,
        start_cfg_bg,
        attempt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_in_process_delegates_cancels_token_and_sets_ignore_result() {
        let mission_id = "mission_scheduler_cancel";
        let worker_id = "wk_cancel";
        let token = CancellationToken::new();
        let child = token.child_token();
        let ignore_result = Arc::new(AtomicBool::new(false));

        register_in_process_delegate(
            mission_id,
            worker_id,
            "feature_cancel",
            token,
            Arc::clone(&ignore_result),
        );

        let cancelled = cancel_in_process_delegates(mission_id, None, true);

        assert_eq!(cancelled, vec![worker_id.to_string()]);
        assert!(child.is_cancelled());
        assert!(ignore_result.load(Ordering::SeqCst));

        let _ = unregister_in_process_delegate(mission_id, worker_id);
    }

    #[test]
    fn unregister_in_process_delegate_removes_active_worker() {
        let mission_id = "mission_scheduler_unregister";
        let worker_id = "wk_unregister";

        register_in_process_delegate(
            mission_id,
            worker_id,
            "feature_unregister",
            CancellationToken::new(),
            Arc::new(AtomicBool::new(false)),
        );

        assert!(has_active_in_process_delegates(mission_id));
        assert_eq!(
            active_in_process_delegate_worker_ids(mission_id),
            vec![worker_id.to_string()]
        );

        let removed = unregister_in_process_delegate(mission_id, worker_id)
            .expect("control should exist before unregister");

        assert_eq!(removed.worker_id, worker_id);
        assert_eq!(removed.feature_id, "feature_unregister");
        assert!(!has_active_in_process_delegates(mission_id));
        assert!(active_in_process_delegate_worker_ids(mission_id).is_empty());
    }
}
