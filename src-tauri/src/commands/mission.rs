//! Tauri commands for Mission system
//!
//! Provides UI-facing commands: create, list, get_status, start, pause, cancel.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tauri::command;
use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::{ProcessManager, WorkerProcess};
use crate::mission::types::INTEGRATOR_FEATURE_ID;
use crate::mission::types::*;
use crate::mission::worker_protocol::{
    FeatureCompletedPayload, StartFeaturePayload, WorkerEventType,
};
use crate::models::{AppError, ErrorCode};

use crate::agent_engine::types::{DEFAULT_MODEL, DEFAULT_PROVIDER};

const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_TIMEOUT_SECS: u64 = 20;
const WORKER_MAX_RECOVERY_ATTEMPTS: u32 = 2;
const WORKER_RECOVERY_BACKOFF_MS: u64 = 1500;
const MISSION_PROGRESS_LOG_MAX_ENTRIES: usize = 200;
const DEFAULT_MISSION_MAX_WORKERS: usize = 1;

#[derive(Debug, Clone)]
struct MissionRunConfig {
    model: String,
    provider: String,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Clone)]
struct MissionStartConfig {
    run_config: MissionRunConfig,
    max_workers: usize,
}

async fn start_feature_on_worker(
    orch: &Orchestrator<'_>,
    worker: &WorkerProcess,
    emitter: &MissionEventEmitter,
    mission_id: &str,
    feature: Feature,
    run_config: &MissionRunConfig,
    worker_id: &str,
    attempt: u32,
    rollback_to_pending_on_start_error: bool,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    let session_id = format!(
        "worker_{}_{}_{}_{}",
        mission_id,
        feature.id,
        attempt,
        chrono::Utc::now().timestamp_millis()
    );

    orch.start_feature(&feature.id, worker_id, attempt)?;
    emitter.worker_started(worker_id, &feature.id)?;
    if emit_orchestrator_transition {
        emitter.state_changed("orchestrator_turn", "running")?;
    }

    if let Err(e) = worker
        .start_feature(StartFeaturePayload {
            feature: feature.clone(),
            session_id,
            model: run_config.model.clone(),
            provider: run_config.provider.clone(),
            base_url: run_config.base_url.clone(),
            api_key: run_config.api_key.clone(),
        })
        .await
    {
        if rollback_to_pending_on_start_error {
            let _ = orch.update_feature_status(&feature.id, FeatureStatus::Pending);
        }
        return Err(e);
    }

    Ok(feature.id)
}

// ── Global worker registry ───────────────────────────────────────

#[derive(Clone)]
struct MissionWorkerHandle {
    worker: Arc<TokioMutex<WorkerProcess>>,
    attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MissionProgressLogEntry {
    ts: i64,
    message: String,
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
struct WorkerRecoveryContext {
    mission_id: String,
    project_path: String,
    worker_id: String,
    feature_id: String,
    run_config: MissionRunConfig,
    attempt: u32,
}

struct MissionWorkerLockGuard {
    _guard: OwnedMutexGuard<()>,
}

/// Maps mission_id → worker_id → WorkerProcess handle.
/// Uses nested Arc<DashMap<...>> to support multi-worker scheduling per mission.
type WorkerRegistry = DashMap<String, Arc<DashMap<String, MissionWorkerHandle>>>;
type PausedConfigRegistry = DashMap<String, MissionStartConfig>;

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

fn get_worker_handle(mission_id: &str, worker_id: &str) -> Option<MissionWorkerHandle> {
    let workers = worker_registry().get(mission_id)?.value().clone();
    workers.get(worker_id).map(|entry| entry.clone())
}

fn insert_worker_handle(mission_id: &str, worker_id: String, handle: MissionWorkerHandle) {
    let workers = mission_worker_map(mission_id);
    workers.insert(worker_id, handle);
}

fn remove_worker_handle(mission_id: &str, worker_id: &str) -> Option<MissionWorkerHandle> {
    let workers = worker_registry().get(mission_id)?.value().clone();
    let removed = workers.remove(worker_id).map(|(_, handle)| handle);
    if workers.is_empty() {
        worker_registry().remove(mission_id);
    }
    removed
}

fn list_worker_handles(mission_id: &str) -> Vec<(String, MissionWorkerHandle)> {
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

fn paused_config_registry() -> &'static PausedConfigRegistry {
    static REGISTRY: std::sync::OnceLock<PausedConfigRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

fn clamp_max_workers(max_workers: Option<usize>) -> usize {
    max_workers
        .map(|v| v.max(1))
        .unwrap_or(DEFAULT_MISSION_MAX_WORKERS)
}

fn append_integrator_feature_if_missing(features: &mut Vec<Feature>) {
    if features.iter().any(|f| f.id == INTEGRATOR_FEATURE_ID) {
        return;
    }

    let depends_on = features
        .iter()
        .map(|f| f.id.clone())
        .filter(|id| id != INTEGRATOR_FEATURE_ID)
        .collect::<Vec<_>>();

    features.push(Feature {
        id: INTEGRATOR_FEATURE_ID.to_string(),
        status: FeatureStatus::Pending,
        description: "Converge mission results and produce final handoff summary".to_string(),
        skill: "integrator".to_string(),
        preconditions: Vec::new(),
        depends_on,
        expected_behavior: vec![
            "Produce final handoff summary covering all features".to_string(),
            "Highlight unresolved failures and potential conflicts".to_string(),
        ],
        verification_steps: vec![
            "Summarize handoffs.jsonl into final mission conclusion".to_string(),
            "List unresolved failures and blockers".to_string(),
        ],
        write_paths: Vec::new(),
    });
}

fn enrich_integrator_feature_context(
    project_path: &std::path::Path,
    mission_id: &str,
    feature: &mut Feature,
) -> Result<(), AppError> {
    if feature.id != INTEGRATOR_FEATURE_ID {
        return Ok(());
    }

    let features_doc = artifacts::read_features(project_path, mission_id)?;
    let handoffs = artifacts::read_handoffs(project_path, mission_id)?;

    let completed = features_doc
        .features
        .iter()
        .filter(|f| f.id != INTEGRATOR_FEATURE_ID && f.status == FeatureStatus::Completed)
        .count();
    let failed = features_doc
        .features
        .iter()
        .filter(|f| f.id != INTEGRATOR_FEATURE_ID && f.status == FeatureStatus::Failed)
        .count();

    let recent_handoffs = handoffs
        .iter()
        .rev()
        .take(20)
        .map(|h| {
            format!(
                "- {} [{}]: {}",
                h.feature_id,
                if h.ok { "ok" } else { "failed" },
                h.summary
            )
        })
        .collect::<Vec<_>>();

    feature.preconditions = vec![
        format!("Mission id: {mission_id}"),
        format!("Completed features: {completed}"),
        format!("Failed features: {failed}"),
    ];
    feature.expected_behavior = vec![
        "Produce mission-level final summary".to_string(),
        "Highlight feature-level unresolved issues".to_string(),
        "Point out possible write conflict symptoms if observed".to_string(),
    ];
    feature.verification_steps = vec![
        "Read mission artifacts and compile final handoff summary".to_string(),
        "Ensure unresolved items are explicit and actionable".to_string(),
        if recent_handoffs.is_empty() {
            "No handoffs available yet".to_string()
        } else {
            format!("Recent handoffs:\n{}", recent_handoffs.join("\n"))
        },
    ];

    Ok(())
}

fn update_worker_assignment_heartbeat(
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: &str,
) {
    if let Ok(mut state_doc) = artifacts::read_state(project_path, mission_id) {
        let now = chrono::Utc::now().timestamp_millis();
        if let Some(assignment) = state_doc.assignments.get_mut(worker_id) {
            assignment.last_heartbeat_at = now;
            state_doc.updated_at = now;
            let _ = artifacts::write_state(project_path, mission_id, &state_doc);
        }
    }
}

async fn spawn_and_initialize_worker(
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
) -> Result<(String, Arc<TokioMutex<WorkerProcess>>), AppError> {
    let worker_binary = ProcessManager::find_worker_binary()?;
    let mission_dir = artifacts::mission_dir(project_path, mission_id)
        .to_string_lossy()
        .to_string();
    let worker_id = format!("wk_{}", uuid::Uuid::new_v4());
    let pm = ProcessManager::new(worker_binary);
    let mut worker = pm.spawn(&worker_id)?;

    if let Ok(mut state_doc) = artifacts::read_state(project_path, mission_id) {
        state_doc
            .worker_pids
            .insert(worker_id.clone(), worker.pid());
        state_doc.updated_at = chrono::Utc::now().timestamp_millis();
        let _ = artifacts::write_state(project_path, mission_id, &state_doc);
    }

    if let Err(e) = worker.initialize(project_path_str, &mission_dir).await {
        clear_worker_from_state(project_path, mission_id, &worker_id);
        return Err(e);
    }

    Ok((worker_id, Arc::new(TokioMutex::new(worker))))
}

async fn schedule_ready_features(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    mission_id: &str,
    project_path: &std::path::Path,
    project_path_str: &str,
    start_config: &MissionStartConfig,
    emit_orchestrator_transition: bool,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, AppError> {
    let active_workers = list_worker_handles(mission_id).len();
    if active_workers >= start_config.max_workers {
        return Ok(Vec::new());
    }

    let available_slots = start_config.max_workers.saturating_sub(active_workers);
    if available_slots == 0 {
        return Ok(Vec::new());
    }

    let ready_features = orch.ready_pending_features(available_slots)?;
    if ready_features.is_empty() {
        return Ok(Vec::new());
    }

    let mut started = Vec::new();

    for (idx, feature) in ready_features.into_iter().enumerate() {
        let mut feature = feature;
        let _ = enrich_integrator_feature_context(project_path, mission_id, &mut feature);

        let (worker_id, worker_arc) =
            spawn_and_initialize_worker(project_path, project_path_str, mission_id).await?;

        let attempt = 0_u32;
        let start_result = {
            let worker = worker_arc.lock().await;
            start_feature_on_worker(
                orch,
                &*worker,
                emitter,
                mission_id,
                feature.clone(),
                &start_config.run_config,
                &worker_id,
                attempt,
                true,
                emit_orchestrator_transition && idx == 0,
            )
            .await
        };

        match start_result {
            Ok(feature_id) => {
                insert_worker_handle(
                    mission_id,
                    worker_id.clone(),
                    MissionWorkerHandle {
                        worker: Arc::clone(&worker_arc),
                        attempt,
                    },
                );

                spawn_worker_supervision_tasks(
                    app_handle.clone(),
                    mission_id.to_string(),
                    project_path_str.to_string(),
                    worker_id,
                    start_config.run_config.clone(),
                    start_config.max_workers,
                );

                started.push(feature_id);
            }
            Err(e) => {
                clear_worker_from_state(project_path, mission_id, &worker_id);
                rollback_active_feature_to_pending(
                    orch,
                    project_path,
                    mission_id,
                    Some(&worker_id),
                );
                return Err(e);
            }
        }
    }

    Ok(started)
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

async fn acquire_mission_runtime_lock(mission_id: &str) -> MissionWorkerLockGuard {
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

fn append_mission_recovery_log(
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

fn clear_worker_from_state(project_path: &std::path::Path, mission_id: &str, worker_id: &str) {
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

fn rollback_active_feature_to_pending(
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

fn pause_mission_with_log(
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
        MissionState::Completed => {
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

fn build_recovery_handoff(
    feature_id: &str,
    worker_id: &str,
    summary: impl Into<String>,
    issue: impl Into<String>,
) -> HandoffEntry {
    HandoffEntry {
        feature_id: feature_id.to_string(),
        worker_id: worker_id.to_string(),
        ok: false,
        summary: summary.into(),
        commands_run: Vec::new(),
        artifacts: Vec::new(),
        issues: vec![issue.into()],
    }
}

fn mark_active_feature_failed(
    orch: &Orchestrator<'_>,
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: &str,
    issue: &str,
    summary: &str,
) {
    if let Ok(state_doc) = artifacts::read_state(project_path, mission_id) {
        let feature_id = state_doc
            .assignments
            .get(worker_id)
            .map(|a| a.feature_id.clone())
            .or_else(|| {
                if state_doc.current_worker_id.as_deref() == Some(worker_id) {
                    state_doc.current_feature_id.clone()
                } else {
                    None
                }
            });
        if let Some(feature_id) = feature_id {
            let handoff = build_recovery_handoff(&feature_id, worker_id, summary, issue);
            let _ = orch.complete_feature(&handoff);
        }
    }
}

async fn try_recover_worker(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    ctx: &WorkerRecoveryContext,
) -> Result<Option<(String, Arc<TokioMutex<WorkerProcess>>)>, AppError> {
    if ctx.attempt >= WORKER_MAX_RECOVERY_ATTEMPTS {
        return Ok(None);
    }

    tokio::time::sleep(Duration::from_millis(WORKER_RECOVERY_BACKOFF_MS)).await;
    let next_attempt = ctx.attempt + 1;

    let worker_binary = match ProcessManager::find_worker_binary() {
        Ok(path) => path,
        Err(e) => {
            append_mission_recovery_log(
                std::path::Path::new(&ctx.project_path),
                &ctx.mission_id,
                format!("recovery worker binary not found: {e}"),
            );
            return Err(e);
        }
    };
    let mission_dir =
        artifacts::mission_dir(std::path::Path::new(&ctx.project_path), &ctx.mission_id)
            .to_string_lossy()
            .to_string();

    let new_worker_id = format!("wk_{}", uuid::Uuid::new_v4());
    let pm = ProcessManager::new(worker_binary);
    let mut new_worker = pm.spawn(&new_worker_id)?;

    if let Ok(mut state_doc) = orch.get_state() {
        state_doc
            .worker_pids
            .insert(new_worker_id.clone(), new_worker.pid());
        state_doc.updated_at = chrono::Utc::now().timestamp_millis();
        let _ = artifacts::write_state(
            std::path::Path::new(&ctx.project_path),
            &ctx.mission_id,
            &state_doc,
        );
    }

    if let Err(e) = new_worker.initialize(&ctx.project_path, &mission_dir).await {
        clear_worker_from_state(
            std::path::Path::new(&ctx.project_path),
            &ctx.mission_id,
            &new_worker_id,
        );
        rollback_active_feature_to_pending(
            orch,
            std::path::Path::new(&ctx.project_path),
            &ctx.mission_id,
            Some(&ctx.worker_id),
        );
        append_mission_recovery_log(
            std::path::Path::new(&ctx.project_path),
            &ctx.mission_id,
            format!("recovery worker initialize failed: {e}"),
        );
        return Err(e);
    }

    let new_worker_arc = Arc::new(TokioMutex::new(new_worker));
    let feature = {
        let features_doc = orch.get_features()?;
        features_doc
            .features
            .iter()
            .find(|f| f.id == ctx.feature_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("feature not found: {}", ctx.feature_id)))?
    };

    let start_result = {
        let w = new_worker_arc.lock().await;
        start_feature_on_worker(
            orch,
            &*w,
            emitter,
            &ctx.mission_id,
            feature,
            &ctx.run_config,
            &new_worker_id,
            next_attempt,
            true,
            false,
        )
        .await
    };

    match start_result {
        Ok(next_feature_id) => {
            insert_worker_handle(
                &ctx.mission_id,
                new_worker_id.clone(),
                MissionWorkerHandle {
                    worker: Arc::clone(&new_worker_arc),
                    attempt: next_attempt,
                },
            );
            let _ = orch.transition(MissionState::Running);
            let _ = emitter.progress_entry(&format!(
                "recovery worker started feature: {}",
                next_feature_id
            ));
            append_mission_recovery_log(
                std::path::Path::new(&ctx.project_path),
                &ctx.mission_id,
                format!(
                    "recovery attempt {next_attempt}/{} succeeded on feature {next_feature_id}",
                    WORKER_MAX_RECOVERY_ATTEMPTS
                ),
            );
            Ok(Some((new_worker_id, new_worker_arc)))
        }
        Err(e) => {
            clear_worker_from_state(
                std::path::Path::new(&ctx.project_path),
                &ctx.mission_id,
                &new_worker_id,
            );
            rollback_active_feature_to_pending(
                orch,
                std::path::Path::new(&ctx.project_path),
                &ctx.mission_id,
                Some(&ctx.worker_id),
            );
            append_mission_recovery_log(
                std::path::Path::new(&ctx.project_path),
                &ctx.mission_id,
                format!("recovery worker failed to start feature: {e}"),
            );
            Err(e)
        }
    }
}

fn spawn_worker_supervision_tasks(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id_bg: String,
    run_config_bg: MissionRunConfig,
    max_workers_bg: usize,
) {
    // Heartbeat task
    {
        let mission_id_hb = mission_id.clone();
        let worker_id_hb = worker_id_bg.clone();
        let emitter_hb = MissionEventEmitter::new(app_handle.clone(), mission_id.clone());
        let start_cfg_hb = MissionStartConfig {
            run_config: run_config_bg.clone(),
            max_workers: max_workers_bg,
        };
        let project_path_hb = project_path_bg.clone();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            loop {
                ticker.tick().await;

                let handle = match get_worker_handle(&mission_id_hb, &worker_id_hb) {
                    Some(h) => h,
                    None => break,
                };

                let timed_out = {
                    let worker = handle.worker.lock().await;
                    if worker.send_ping().await.is_err() {
                        true
                    } else {
                        worker.is_timed_out(Duration::from_secs(HEARTBEAT_TIMEOUT_SECS))
                    }
                };

                if timed_out {
                    let _runtime_lock = acquire_mission_runtime_lock(&mission_id_hb).await;
                    let _ = remove_worker_handle(&mission_id_hb, &worker_id_hb);
                    paused_config_registry().insert(mission_id_hb.clone(), start_cfg_hb.clone());
                    clear_worker_from_state(
                        std::path::Path::new(&project_path_hb),
                        &mission_id_hb,
                        &worker_id_hb,
                    );

                    let orch_hb = Orchestrator::new(
                        std::path::Path::new(&project_path_hb),
                        mission_id_hb.clone(),
                    );
                    pause_mission_with_log(
                        &orch_hb,
                        &emitter_hb,
                        std::path::Path::new(&project_path_hb),
                        &mission_id_hb,
                        &start_cfg_hb,
                        format!("worker heartbeat timeout: {worker_id_hb}"),
                    );
                    break;
                }

                update_worker_assignment_heartbeat(
                    std::path::Path::new(&project_path_hb),
                    &mission_id_hb,
                    &worker_id_hb,
                );
                let _ = emitter_hb.heartbeat(&worker_id_hb);
            }
        });
    }

    // Worker event monitor task
    tokio::spawn(async move {
        let worker_id = worker_id_bg;
        let orch_bg = Orchestrator::new(std::path::Path::new(&project_path_bg), mission_id.clone());
        let emitter_bg = MissionEventEmitter::new(app_handle.clone(), mission_id.clone());
        let start_cfg_bg = MissionStartConfig {
            run_config: run_config_bg.clone(),
            max_workers: max_workers_bg,
        };

        loop {
            let registry_handle = match get_worker_handle(&mission_id, &worker_id) {
                Some(h) => h,
                None => break,
            };
            let recovery_attempt = registry_handle.attempt;

            let event_result = {
                let mut w = registry_handle.worker.lock().await;
                w.recv().await
            };

            match event_result {
                None => {
                    let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;
                    tracing::warn!(
                        target: "mission",
                        mission_id = %mission_id,
                        worker_id = %worker_id,
                        attempt = recovery_attempt,
                        "worker stdout channel closed unexpectedly"
                    );

                    let state_now = orch_bg.get_state().ok().map(|s| s.state);
                    let should_recover = matches!(
                        state_now,
                        Some(
                            MissionState::Running
                                | MissionState::OrchestratorTurn
                                | MissionState::Paused
                        )
                    );

                    if should_recover {
                        let feature_id = orch_bg
                            .get_state()
                            .ok()
                            .and_then(|s| {
                                s.assignments.get(&worker_id).map(|a| a.feature_id.clone())
                            })
                            .unwrap_or_default();
                        let ctx = WorkerRecoveryContext {
                            mission_id: mission_id.clone(),
                            project_path: project_path_bg.clone(),
                            worker_id: worker_id.clone(),
                            feature_id,
                            run_config: run_config_bg.clone(),
                            attempt: recovery_attempt,
                        };

                        match try_recover_worker(&orch_bg, &emitter_bg, &ctx).await {
                            Ok(Some((new_worker_id, _new_worker_arc))) => {
                                spawn_worker_supervision_tasks(
                                    app_handle.clone(),
                                    mission_id.clone(),
                                    project_path_bg.clone(),
                                    new_worker_id,
                                    run_config_bg.clone(),
                                    max_workers_bg,
                                );
                                break;
                            }
                            Ok(None) => {
                                mark_active_feature_failed(
                                    &orch_bg,
                                    std::path::Path::new(&project_path_bg),
                                    &mission_id,
                                    &worker_id,
                                    "E_WORKER_RECOVERY_EXHAUSTED",
                                    "worker crashed and recovery exhausted",
                                );
                                let _ = remove_worker_handle(&mission_id, &worker_id);
                                paused_config_registry()
                                    .insert(mission_id.clone(), start_cfg_bg.clone());
                                pause_mission_with_log(
                                    &orch_bg,
                                    &emitter_bg,
                                    std::path::Path::new(&project_path_bg),
                                    &mission_id,
                                    &start_cfg_bg,
                                    "worker crashed and recovery exhausted",
                                );
                                break;
                            }
                            Err(e) => {
                                mark_active_feature_failed(
                                    &orch_bg,
                                    std::path::Path::new(&project_path_bg),
                                    &mission_id,
                                    &worker_id,
                                    "E_WORKER_RECOVERY_FAILED",
                                    &format!("worker recovery failed: {e}"),
                                );
                                let _ = remove_worker_handle(&mission_id, &worker_id);
                                paused_config_registry()
                                    .insert(mission_id.clone(), start_cfg_bg.clone());
                                pause_mission_with_log(
                                    &orch_bg,
                                    &emitter_bg,
                                    std::path::Path::new(&project_path_bg),
                                    &mission_id,
                                    &start_cfg_bg,
                                    format!("worker crashed and recovery failed: {e}"),
                                );
                                break;
                            }
                        }
                    }

                    let _ = remove_worker_handle(&mission_id, &worker_id);
                    paused_config_registry().insert(mission_id.clone(), start_cfg_bg.clone());
                    pause_mission_with_log(
                        &orch_bg,
                        &emitter_bg,
                        std::path::Path::new(&project_path_bg),
                        &mission_id,
                        &start_cfg_bg,
                        format!("worker crashed and mission paused: {worker_id}"),
                    );
                    break;
                }
                Some(Err(e)) => {
                    tracing::warn!(
                        target: "mission",
                        mission_id = %mission_id,
                        worker_id = %worker_id,
                        error = %e,
                        "worker event parse error"
                    );
                    append_mission_recovery_log(
                        std::path::Path::new(&project_path_bg),
                        &mission_id,
                        format!("worker event parse error ({worker_id}): {e}"),
                    );
                }
                Some(Ok(worker_event)) => match worker_event.event_type {
                    WorkerEventType::AgentEvent => {
                        use tauri::Emitter;
                        let _ = app_handle.emit(
                            crate::agent_engine::events::AGENT_EVENT_CHANNEL,
                            &worker_event.payload,
                        );
                    }
                    WorkerEventType::FeatureCompleted => {
                        let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;

                        match serde_json::from_value::<FeatureCompletedPayload>(
                            worker_event.payload.clone(),
                        ) {
                            Ok(completed) => {
                                tracing::info!(
                                    target: "mission",
                                    mission_id = %mission_id,
                                    worker_id = %worker_id,
                                    feature_id = %completed.feature_id,
                                    ok = completed.ok,
                                    "worker reported feature completed"
                                );

                                let _ = emitter_bg.worker_completed(
                                    &worker_id,
                                    &completed.feature_id,
                                    completed.ok,
                                    &completed.handoff.summary,
                                );

                                match orch_bg.complete_feature(&completed.handoff) {
                                    Ok(next_state) => {
                                        let next_state_str = serde_json::to_string(&next_state)
                                            .unwrap_or_default()
                                            .trim_matches('"')
                                            .to_string();
                                        let _ =
                                            emitter_bg.state_changed("running", &next_state_str);

                                        let _ = remove_worker_handle(&mission_id, &worker_id);
                                        clear_worker_from_state(
                                            std::path::Path::new(&project_path_bg),
                                            &mission_id,
                                            &worker_id,
                                        );

                                        let actual_state =
                                            orch_bg.get_state().ok().map(|s| s.state);
                                        let should_schedule = matches!(
                                            actual_state,
                                            Some(
                                                MissionState::Running
                                                    | MissionState::OrchestratorTurn
                                            )
                                        );

                                        if should_schedule
                                            && (next_state == MissionState::OrchestratorTurn
                                                || next_state == MissionState::Running)
                                        {
                                            let _ = emitter_bg.progress_entry(
                                                "Feature completed, scheduling next feature...",
                                            );

                                            match schedule_ready_features(
                                                &orch_bg,
                                                &emitter_bg,
                                                &mission_id,
                                                std::path::Path::new(&project_path_bg),
                                                &project_path_bg,
                                                &start_cfg_bg,
                                                true,
                                                app_handle.clone(),
                                            )
                                            .await
                                            {
                                                Ok(started) => {
                                                    if started.is_empty() {
                                                        let finished =
                                                            orch_bg.is_finished().unwrap_or(false);
                                                        if finished {
                                                            let _ = orch_bg.transition(
                                                                MissionState::Completed,
                                                            );
                                                            let _ = emitter_bg.state_changed(
                                                                "orchestrator_turn",
                                                                "completed",
                                                            );
                                                        }
                                                    } else {
                                                        let _ =
                                                            emitter_bg.progress_entry(&format!(
                                                                "Started next features: {}",
                                                                started.join(", ")
                                                            ));
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        target: "mission",
                                                        mission_id = %mission_id,
                                                        error = %e,
                                                        "failed to schedule next features"
                                                    );
                                                    let _ = emitter_bg.progress_entry(
                                                        "failed to schedule next features",
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            target: "mission",
                                            mission_id = %mission_id,
                                            error = %e,
                                            "failed to advance orchestrator after feature complete"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    target: "mission",
                                    mission_id = %mission_id,
                                    worker_id = %worker_id,
                                    error = %e,
                                    "failed to parse FeatureCompleted payload"
                                );
                            }
                        }

                        let state_now = orch_bg.get_state().ok().map(|s| s.state);
                        if !matches!(
                            state_now,
                            Some(MissionState::Running | MissionState::OrchestratorTurn)
                        ) {
                            let _ = remove_worker_handle(&mission_id, &worker_id);
                            clear_worker_from_state(
                                std::path::Path::new(&project_path_bg),
                                &mission_id,
                                &worker_id,
                            );
                            break;
                        }
                    }
                    WorkerEventType::Pong => {
                        if let Some(handle) = get_worker_handle(&mission_id, &worker_id) {
                            let w = handle.worker.lock().await;
                            w.record_pong();
                            update_worker_assignment_heartbeat(
                                std::path::Path::new(&project_path_bg),
                                &mission_id,
                                &worker_id,
                            );
                            let _ = emitter_bg.heartbeat(&worker_id);
                        } else {
                            break;
                        }
                    }
                    WorkerEventType::Ack => {
                        tracing::debug!(
                            target: "mission",
                            mission_id = %mission_id,
                            worker_id = %worker_id,
                            "received ack from worker"
                        );
                    }
                },
            }
        }

        tracing::info!(
            target: "mission",
            mission_id = %mission_id,
            worker_id = %worker_id,
            "worker event monitor task exiting"
        );
    });
}

// ── Input/Output DTOs ───────────────────────────────────────────

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

fn resolve_start_config(input: &MissionStartInput) -> Result<MissionStartConfig, AppError> {
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

// ── Commands ────────────────────────────────────────────────────

#[command]
pub async fn mission_create(input: MissionCreateInput) -> Result<MissionCreateOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);

    let mut features = input.features;
    append_integrator_feature_if_missing(&mut features);

    let mission_id =
        Orchestrator::create_mission(project_path, &input.title, &input.mission_text, features)?;

    Ok(MissionCreateOutput {
        schema_version: MISSION_SCHEMA_VERSION,
        mission_id,
    })
}

#[command]
pub async fn mission_list(input: MissionListInput) -> Result<Vec<String>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::list_missions(project_path)
}

#[command]
pub async fn mission_get_status(
    input: MissionGetStatusInput,
) -> Result<MissionGetStatusOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let state = artifacts::read_state(project_path, &input.mission_id)?;
    let features = artifacts::read_features(project_path, &input.mission_id)?;
    let handoffs = artifacts::read_handoffs(project_path, &input.mission_id)?;

    Ok(MissionGetStatusOutput {
        state,
        features,
        handoffs,
    })
}

#[command]
pub async fn mission_start(
    app_handle: tauri::AppHandle,
    input: MissionStartInput,
) -> Result<(), AppError> {
    let project_path_str = input.project_path.clone();
    let project_path = std::path::Path::new(&project_path_str);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle.clone(), input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let current_state = orch.get_state()?;
    if matches!(
        current_state.state,
        MissionState::Running | MissionState::Initializing
    ) {
        return Err(AppError::invalid_argument("mission already running"));
    }

    let start_config = resolve_start_config(&input)?;
    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    orch.transition(MissionState::Initializing)?;
    emitter.state_changed(&old_state_str, "initializing")?;

    let started = match schedule_ready_features(
        &orch,
        &emitter,
        &input.mission_id,
        project_path,
        &project_path_str,
        &start_config,
        false,
        app_handle.clone(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            let _ = orch.transition(MissionState::Paused);
            paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
            append_mission_recovery_log(
                project_path,
                &input.mission_id,
                format!("mission_start scheduling failed: {e}"),
            );
            return Err(e);
        }
    };

    if started.is_empty() {
        if orch.is_finished()? {
            orch.transition(MissionState::Completed)?;
            emitter.state_changed("initializing", "completed")?;
            paused_config_registry().remove(&input.mission_id);
            return Ok(());
        }

        let _ = orch.transition(MissionState::Paused);
        paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
        return Err(AppError::invalid_argument(
            "no schedulable pending features (dependency or write_paths conflict)",
        ));
    }

    emitter.state_changed("initializing", "running")?;
    paused_config_registry().insert(input.mission_id.clone(), start_config.clone());

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        max_workers = start_config.max_workers,
        started_features = %started.join(","),
        "mission started"
    );

    Ok(())
}

#[command]
pub async fn mission_pause(
    app_handle: tauri::AppHandle,
    input: MissionControlInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let current_state = orch.get_state()?;
    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    if current_state.state != MissionState::Running {
        return Err(AppError::invalid_argument("mission is not running"));
    }

    let start_config = paused_config_registry()
        .get(&input.mission_id)
        .map(|cfg| cfg.clone())
        .ok_or_else(|| {
            AppError::invalid_argument(
                "mission pause requires mission_start to provide run configuration first",
            )
        })?;

    let workers = list_worker_handles(&input.mission_id);
    if workers.is_empty() {
        return Err(AppError::invalid_argument(
            "mission has no active worker to pause",
        ));
    }

    for (worker_id, worker_entry) in workers {
        let worker = worker_entry.worker.lock().await;
        if let Err(e) = worker.kill(Duration::from_secs(2)).await {
            tracing::warn!(
                target: "mission",
                mission_id = %input.mission_id,
                worker_id = %worker_id,
                error = %e,
                "failed to stop worker during pause (may have already exited)"
            );
            append_mission_recovery_log(
                project_path,
                &input.mission_id,
                format!("mission_pause worker stop failed for {worker_id}: {e}"),
            );
        }
        clear_worker_from_state(project_path, &input.mission_id, &worker_id);
        let _ = remove_worker_handle(&input.mission_id, &worker_id);
    }

    paused_config_registry().insert(input.mission_id.clone(), start_config);

    orch.transition(MissionState::Paused)?;
    emitter.state_changed(&old_state_str, "paused")?;
    append_mission_recovery_log(project_path, &input.mission_id, "mission paused by user");

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        "mission paused"
    );

    Ok(())
}

#[command]
pub async fn mission_resume(
    app_handle: tauri::AppHandle,
    input: MissionControlInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle.clone(), input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let current_state = orch.get_state()?;
    if current_state.state != MissionState::Paused {
        return Err(AppError::invalid_argument("mission is not paused"));
    }

    let start_config = match paused_config_registry().remove(&input.mission_id) {
        Some((_, cfg)) => cfg,
        None => {
            return Err(AppError::invalid_argument(
                "mission resume requires mission_start to provide run configuration first",
            ))
        }
    };

    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    if !list_worker_handles(&input.mission_id).is_empty() {
        paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
        append_mission_recovery_log(
            project_path,
            &input.mission_id,
            "mission_resume rejected: mission already has active workers",
        );
        return Err(AppError::invalid_argument(
            "mission already has active workers",
        ));
    }

    orch.transition(MissionState::Running)?;
    emitter.state_changed(&old_state_str, "running")?;

    match schedule_ready_features(
        &orch,
        &emitter,
        &input.mission_id,
        project_path,
        &input.project_path,
        &start_config,
        false,
        app_handle.clone(),
    )
    .await
    {
        Ok(started) => {
            if started.is_empty() {
                if orch.is_finished()? {
                    orch.transition(MissionState::Completed)?;
                    emitter.state_changed("running", "completed")?;
                } else {
                    orch.transition(MissionState::Paused)?;
                    paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
                    return Err(AppError::invalid_argument(
                        "no schedulable pending features on resume",
                    ));
                }
            } else {
                emitter.progress_entry(&format!("resumed features: {}", started.join(", ")))?;
            }
        }
        Err(e) => {
            orch.transition(MissionState::Paused)?;
            paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
            append_mission_recovery_log(
                project_path,
                &input.mission_id,
                format!("mission_resume scheduling failed: {e}"),
            );
            return Err(e);
        }
    }

    append_mission_recovery_log(project_path, &input.mission_id, "mission resumed");

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        max_workers = start_config.max_workers,
        "mission resumed"
    );

    Ok(())
}

#[command]
pub async fn mission_cancel(
    app_handle: tauri::AppHandle,
    input: MissionControlInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    paused_config_registry().remove(&input.mission_id);

    for (worker_id, worker_entry) in list_worker_handles(&input.mission_id) {
        let worker = worker_entry.worker.lock().await;
        if let Err(e) = worker.kill(Duration::from_secs(5)).await {
            tracing::warn!(
                target: "mission",
                mission_id = %input.mission_id,
                worker_id = %worker_id,
                error = %e,
                "failed to kill worker during cancel (may have already exited)"
            );
        }
        clear_worker_from_state(project_path, &input.mission_id, &worker_id);
        let _ = remove_worker_handle(&input.mission_id, &worker_id);
    }

    let features_doc = orch.get_features()?;
    for feature in &features_doc.features {
        if feature.status == FeatureStatus::InProgress || feature.status == FeatureStatus::Pending {
            orch.update_feature_status(&feature.id, FeatureStatus::Cancelled)?;
            emitter.features_changed(&feature.id, "cancelled")?;
        }
    }

    let current_state = orch.get_state()?;
    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    orch.transition(MissionState::Completed)?;
    emitter.state_changed(&old_state_str, "completed")?;
    append_mission_recovery_log(
        project_path,
        &input.mission_id,
        "mission cancelled and marked completed",
    );

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        "mission cancelled"
    );

    Ok(())
}
