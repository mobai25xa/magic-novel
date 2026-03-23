//! Mission command runtime state (worker registries, locks, recovery log).

use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::WorkerProcess;
use crate::mission::types::*;

use super::{MissionRunConfig, MissionStartConfig};

const MISSION_PROGRESS_LOG_MAX_ENTRIES: usize = 200;

// ── Global worker registry ───────────────────────────────────────

#[derive(Clone)]
pub(super) struct MissionWorkerHandle {
    pub(super) worker: Arc<TokioMutex<WorkerProcess>>,
    pub(super) attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MissionProgressLogEntry {
    pub ts: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MissionRecoveryLog {
    schema_version: i32,
    mission_id: String,
    entries: Vec<MissionProgressLogEntry>,
}

impl MissionRecoveryLog {
    fn new(mission_id: &str) -> Self {
        Self {
            schema_version: MISSION_SCHEMA_VERSION,
            mission_id: mission_id.to_string(),
            entries: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub(super) struct WorkerRecoveryContext {
    pub(super) mission_id: String,
    pub(super) project_path: String,
    pub(super) worker_id: String,
    pub(super) feature_id: String,
    pub(super) run_config: MissionRunConfig,
    pub(super) attempt: u32,
}

pub(super) struct MissionWorkerLockGuard {
    _guard: OwnedMutexGuard<()>,
}

/// Maps mission_id → worker_id → WorkerProcess handle.
/// Uses nested Arc<DashMap<...>> to support multi-worker scheduling per mission.
type WorkerRegistry = DashMap<String, Arc<DashMap<String, MissionWorkerHandle>>>;
pub(super) type PausedConfigRegistry = DashMap<String, MissionStartConfig>;

fn worker_registry() -> &'static WorkerRegistry {
    static REGISTRY: std::sync::OnceLock<WorkerRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

fn mission_worker_map(mission_id: &str) -> Arc<DashMap<String, MissionWorkerHandle>> {
    worker_registry()
        .entry(mission_id.to_string())
        .or_insert_with(|| Arc::new(DashMap::new()))
        .clone()
}

pub(super) fn get_worker_handle(mission_id: &str, worker_id: &str) -> Option<MissionWorkerHandle> {
    let workers = worker_registry().get(mission_id)?.value().clone();
    workers.get(worker_id).map(|entry| entry.clone())
}

pub(super) fn insert_worker_handle(
    mission_id: &str,
    worker_id: String,
    handle: MissionWorkerHandle,
) {
    let workers = mission_worker_map(mission_id);
    workers.insert(worker_id, handle);
}

pub(super) fn remove_worker_handle(
    mission_id: &str,
    worker_id: &str,
) -> Option<MissionWorkerHandle> {
    let workers = worker_registry().get(mission_id)?.value().clone();
    let removed = workers.remove(worker_id).map(|(_, handle)| handle);
    if workers.is_empty() {
        worker_registry().remove(mission_id);
    }
    removed
}

pub(super) fn list_worker_handles(mission_id: &str) -> Vec<(String, MissionWorkerHandle)> {
    match worker_registry().get(mission_id) {
        Some(entry) => {
            let workers = entry.value().clone();
            workers
                .iter()
                .map(|kv| (kv.key().clone(), kv.value().clone()))
                .collect()
        }
        None => Vec::new(),
    }
}

pub(super) fn paused_config_registry() -> &'static PausedConfigRegistry {
    static REGISTRY: std::sync::OnceLock<PausedConfigRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

fn mission_runtime_lock(mission_id: &str) -> Arc<TokioMutex<()>> {
    static LOCKS: std::sync::OnceLock<DashMap<String, Arc<TokioMutex<()>>>> =
        std::sync::OnceLock::new();
    LOCKS
        .get_or_init(DashMap::new)
        .entry(mission_id.to_string())
        .or_insert_with(|| Arc::new(TokioMutex::new(())))
        .clone()
}

pub(super) async fn acquire_mission_runtime_lock(mission_id: &str) -> MissionWorkerLockGuard {
    let lock = mission_runtime_lock(mission_id);
    let guard = lock.lock_owned().await;
    MissionWorkerLockGuard { _guard: guard }
}

fn mission_recovery_log_path(
    project_path: &std::path::Path,
    mission_id: &str,
) -> std::path::PathBuf {
    artifacts::mission_dir(project_path, mission_id).join("recovery_log.json")
}

pub(super) fn append_mission_recovery_log(
    project_path: &std::path::Path,
    mission_id: &str,
    message: impl Into<String>,
) {
    let path = mission_recovery_log_path(project_path, mission_id);
    let mut log = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<MissionRecoveryLog>(&raw).ok())
        .unwrap_or_else(|| MissionRecoveryLog::new(mission_id));

    log.entries.push(MissionProgressLogEntry {
        ts: chrono::Utc::now().timestamp_millis(),
        message: message.into(),
    });
    if log.entries.len() > MISSION_PROGRESS_LOG_MAX_ENTRIES {
        let keep_from = log.entries.len() - MISSION_PROGRESS_LOG_MAX_ENTRIES;
        log.entries = log.entries.split_off(keep_from);
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crate::utils::atomic_write::atomic_write_json(&path, &log);
}

pub(crate) fn read_mission_recovery_log(
    project_path: &std::path::Path,
    mission_id: &str,
) -> Vec<MissionProgressLogEntry> {
    let path = mission_recovery_log_path(project_path, mission_id);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<MissionRecoveryLog>(&raw).ok())
        .map(|log| log.entries)
        .unwrap_or_default()
}

pub(super) fn clear_worker_from_state(
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: &str,
) {
    if let Ok(mut state_doc) = artifacts::read_state(project_path, mission_id) {
        state_doc.worker_pids.remove(worker_id);
        state_doc.assignments.remove(worker_id);
        if state_doc.current_worker_id.as_deref() == Some(worker_id) {
            if let Some((wid, assignment)) = state_doc
                .assignments
                .iter()
                .max_by_key(|(_, assignment)| assignment.started_at)
            {
                state_doc.current_worker_id = Some(wid.clone());
                state_doc.current_feature_id = Some(assignment.feature_id.clone());
            } else {
                state_doc.current_worker_id = None;
                state_doc.current_feature_id = None;
            }
        }
        state_doc.updated_at = chrono::Utc::now().timestamp_millis();
        let _ = artifacts::write_state(project_path, mission_id, &state_doc);
    }
}

pub(super) fn rollback_active_feature_to_pending(
    orch: &Orchestrator<'_>,
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: Option<&str>,
) {
    if let Ok(state_doc) = artifacts::read_state(project_path, mission_id) {
        let feature_id = if let Some(worker_id) = worker_id {
            state_doc
                .assignments
                .get(worker_id)
                .map(|a| a.feature_id.clone())
                .or_else(|| {
                    if state_doc.current_worker_id.as_deref() == Some(worker_id) {
                        state_doc.current_feature_id.clone()
                    } else {
                        None
                    }
                })
        } else {
            state_doc.current_feature_id.clone()
        };

        if let Some(feature_id) = feature_id {
            let _ = orch.update_feature_status(&feature_id, FeatureStatus::Pending);
        }
    }
}

pub(super) fn pause_mission_with_log(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    project_path: &std::path::Path,
    mission_id: &str,
    start_config: &MissionStartConfig,
    message: impl Into<String>,
) {
    let msg = message.into();
    append_mission_recovery_log(project_path, mission_id, msg.clone());
    let _ = emitter.progress_entry(&msg);
    let old_state = orch
        .get_state()
        .ok()
        .map(|s| s.state)
        .unwrap_or(MissionState::Paused);

    match old_state {
        MissionState::Completed | MissionState::Cancelled | MissionState::Failed => {
            paused_config_registry().remove(mission_id);
        }
        MissionState::Paused => {
            paused_config_registry().insert(mission_id.to_string(), start_config.clone());
        }
        _ => {
            let old_state_str = serde_json::to_string(&old_state)
                .unwrap_or_else(|_| "\"running\"".to_string())
                .trim_matches('"')
                .to_string();
            let _ = orch.transition(MissionState::Paused);
            let _ = emitter.state_changed(&old_state_str, "paused");
            paused_config_registry().insert(mission_id.to_string(), start_config.clone());
        }
    }
}
