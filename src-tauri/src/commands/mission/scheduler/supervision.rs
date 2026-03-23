use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;

use crate::knowledge::{types as knowledge_types, writeback as knowledge_writeback};
use crate::mission::agent_profile::AgentProfile;
use crate::mission::artifacts;
use crate::mission::delegate_types::{DelegateInputRef, DelegateRequest, ExpectedOutputRef};
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::{
    AttachedWorkerProcessTransport, DelegateRunContext, DelegateRunner, InProcessDelegateRunner,
    ProcessDelegateRunner, ProcessManager, WorkerProcess,
};
use crate::mission::result_types::{AgentTaskResult, OpenIssue, TaskResultStatus, TaskStopReason};
use crate::mission::role_profile::RoleProfile;
use crate::mission::types::*;
use crate::mission::worker_protocol::{FeatureCompletedPayload, WorkerEventType};
use crate::models::AppError;
use crate::services::agent_session::{
    self as agent_session, CanonUpdatesAcceptedEntry, CanonUpdatesProposedEntry,
};
use crate::services::global_config;

use crate::review::types as review_types;

use super::super::review_gate::*;
use super::super::runtime::*;
use super::{MissionRunConfig, MissionStartConfig};

const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_TIMEOUT_SECS: u64 = 20;
const WORKER_MAX_RECOVERY_ATTEMPTS: u32 = 2;
const WORKER_RECOVERY_BACKOFF_MS: u64 = 1500;
const DELEGATE_RUNTIME_FAILURE_CODE: &str = "E_DELEGATE_RUNTIME_FAILED";

struct InProcessDelegateSignals {
    finished: AtomicBool,
    ignore_result: Arc<AtomicBool>,
}

impl InProcessDelegateSignals {
    fn new(ignore_result: Arc<AtomicBool>) -> Self {
        Self {
            finished: AtomicBool::new(false),
            ignore_result,
        }
    }
}

fn build_recovery_task_result(
    feature_id: &str,
    worker_id: &str,
    summary: impl Into<String>,
    issue: impl Into<String>,
) -> AgentTaskResult {
    let summary = summary.into();
    let issue_code = issue.into();
    AgentTaskResult {
        task_id: feature_id.to_string(),
        actor_id: worker_id.to_string(),
        goal: format!("Recover failed feature {feature_id}"),
        status: TaskResultStatus::Failed,
        stop_reason: TaskStopReason::Error,
        result_summary: summary.clone(),
        open_issues: vec![OpenIssue {
            code: Some(issue_code),
            summary,
            blocking: true,
        }],
        ..AgentTaskResult::default()
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
            let result = build_recovery_task_result(&feature_id, worker_id, summary, issue);
            let _ = orch.complete_feature_result(&feature_id, worker_id, &result);
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
    let new_worker = pm.spawn(&new_worker_id)?;

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
    let mut feature = {
        let features_doc = orch.get_features()?;
        features_doc
            .features
            .iter()
            .find(|f| f.id == ctx.feature_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("feature not found: {}", ctx.feature_id)))?
    };

    let _ = super::core::enrich_integrator_feature_context(
        std::path::Path::new(&ctx.project_path),
        &ctx.mission_id,
        &mut feature,
    );

    let worker_defs = global_config::load_worker_definitions();
    let worker_profile = super::core::select_worker_profile_for_feature(&feature, &worker_defs);

    let start_result = {
        let w = new_worker_arc.lock().await;
        super::core::start_feature_on_worker(
            orch,
            &*w,
            emitter,
            std::path::Path::new(&ctx.project_path),
            &ctx.mission_id,
            feature,
            &ctx.run_config,
            &new_worker_id,
            next_attempt,
            worker_profile,
            crate::mission::agent_profile::SessionSource::WorkflowJob,
            None,
            None,
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

fn spawn_worker_heartbeat_task(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id_bg: String,
    start_cfg_bg: MissionStartConfig,
) {
    let mission_id_hb = mission_id.clone();
    let worker_id_hb = worker_id_bg;
    let emitter_hb = MissionEventEmitter::new(app_handle, mission_id.clone());
    let project_path_hb = project_path_bg;

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
                paused_config_registry().insert(mission_id_hb.clone(), start_cfg_bg.clone());
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
                    &start_cfg_bg,
                    format!("worker heartbeat timeout: {worker_id_hb}"),
                );
                break;
            }

            super::core::update_worker_assignment_heartbeat(
                std::path::Path::new(&project_path_hb),
                &mission_id_hb,
                &worker_id_hb,
            );
            let _ = emitter_hb.heartbeat(&worker_id_hb);
        }
    });
}

fn mission_state_label(state: &MissionState) -> String {
    serde_json::to_string(state)
        .unwrap_or_else(|_| format!("\"{:?}\"", state))
        .trim_matches('"')
        .to_string()
}

fn mission_state_accepts_delegate_result(state: Option<MissionState>) -> bool {
    matches!(
        state,
        Some(MissionState::Running | MissionState::OrchestratorTurn)
    )
}

fn spawn_in_process_delegate_watchdog_task(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id: String,
    signals: Arc<InProcessDelegateSignals>,
) {
    let emitter = MissionEventEmitter::new(app_handle, mission_id.clone());

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        loop {
            ticker.tick().await;

            if signals.finished.load(Ordering::SeqCst) {
                break;
            }

            let state_now =
                artifacts::read_state(std::path::Path::new(&project_path_bg), &mission_id)
                    .ok()
                    .map(|state| state.state);
            if mission_state_accepts_delegate_result(state_now.clone()) {
                super::core::update_worker_assignment_heartbeat(
                    std::path::Path::new(&project_path_bg),
                    &mission_id,
                    &worker_id,
                );
                let _ = emitter.heartbeat(&worker_id);
                continue;
            }

            let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;
            if signals.finished.load(Ordering::SeqCst) {
                break;
            }

            let latest_state =
                artifacts::read_state(std::path::Path::new(&project_path_bg), &mission_id)
                    .ok()
                    .map(|state| state.state);
            if mission_state_accepts_delegate_result(latest_state.clone()) {
                continue;
            }

            signals.ignore_result.store(true, Ordering::SeqCst);
            clear_worker_from_state(
                std::path::Path::new(&project_path_bg),
                &mission_id,
                &worker_id,
            );

            let state_label = latest_state
                .as_ref()
                .map(mission_state_label)
                .unwrap_or_else(|| "unknown".to_string());
            let message = format!(
                "in-process delegate detached after mission entered {state_label}: {worker_id}"
            );
            append_mission_recovery_log(
                std::path::Path::new(&project_path_bg),
                &mission_id,
                &message,
            );
            let _ = emitter.progress_entry(&message);
            break;
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn handle_feature_completed_payload(
    app_handle: &tauri::AppHandle,
    orch_bg: &Orchestrator<'_>,
    emitter_bg: &MissionEventEmitter,
    mission_id: &str,
    project_path_bg: &str,
    worker_id: &str,
    start_cfg_bg: &MissionStartConfig,
    completed: FeatureCompletedPayload,
) {
    let effective_ok = completed.result.is_ok();
    let effective_summary = completed.result_summary();
    let stop_reason = format!("{:?}", completed.result.stop_reason);
    let changed_path_count = completed.result.changed_paths.len();
    let open_issue_count = completed.result.open_issues.len();
    tracing::info!(
        target: "mission",
        mission_id = %mission_id,
        worker_id = %worker_id,
        feature_id = %completed.feature_id,
        ok = effective_ok,
        stop_reason = %stop_reason,
        changed_paths = changed_path_count,
        open_issues = open_issue_count,
        "delegate runtime reported feature completed"
    );

    let _ = emitter_bg.worker_completed(
        worker_id,
        &completed.feature_id,
        effective_ok,
        &effective_summary,
    );

    if open_issue_count > 0 {
        let _ = emitter_bg.progress_entry(&format!(
            "Worker reported {} open issue(s) for {}",
            open_issue_count, completed.feature_id
        ));
    }

    match orch_bg.complete_feature_result(&completed.feature_id, worker_id, &completed.result) {
        Ok(next_state) => {
            let completion_status = if effective_ok {
                FeatureStatus::Completed
            } else {
                FeatureStatus::Failed
            };
            let _ = super::super::macro_commands::update_macro_state_on_feature_event(
                std::path::Path::new(project_path_bg),
                mission_id,
                &completed.feature_id,
                &completion_status,
                Some(emitter_bg),
            );

            let next_state_str = serde_json::to_string(&next_state)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            let _ = emitter_bg.state_changed("running", &next_state_str);

            let _ = remove_worker_handle(mission_id, worker_id);
            clear_worker_from_state(std::path::Path::new(project_path_bg), mission_id, worker_id);

            let actual_state = orch_bg.get_state().ok().map(|state| state.state);
            let should_schedule = matches!(
                actual_state,
                Some(MissionState::Running | MissionState::OrchestratorTurn)
            );

            if should_schedule
                && matches!(
                    next_state,
                    MissionState::Running | MissionState::OrchestratorTurn
                )
            {
                let _ = emitter_bg.progress_entry("Feature completed, scheduling next feature...");

                match super::core::schedule_ready_features(
                    orch_bg,
                    emitter_bg,
                    mission_id,
                    std::path::Path::new(project_path_bg),
                    project_path_bg,
                    start_cfg_bg,
                    true,
                    app_handle.clone(),
                )
                .await
                {
                    Ok(started) => {
                        if started.is_empty() {
                            if orch_bg.is_finished().unwrap_or(false) {
                                let _ = orch_bg.transition(MissionState::Completed);
                                let _ = emitter_bg.state_changed("orchestrator_turn", "completed");
                            }
                        } else {
                            let _ = emitter_bg.progress_entry(&format!(
                                "Started next features: {}",
                                started.join(", ")
                            ));
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            target: "mission",
                            mission_id = %mission_id,
                            error = %error,
                            "failed to schedule next features after delegate completion"
                        );
                        let _ = emitter_bg.progress_entry("failed to schedule next features");
                    }
                }
            }
        }
        Err(error) => {
            tracing::error!(
                target: "mission",
                mission_id = %mission_id,
                error = %error,
                "failed to advance orchestrator after delegate feature complete"
            );
        }
    }
}

fn build_delegate_request(
    mission_id: &str,
    feature: &Feature,
    worker_id: &str,
    agent_profile: &AgentProfile,
    start_cfg: &MissionStartConfig,
) -> DelegateRequest {
    let input_refs = feature
        .write_paths
        .iter()
        .map(|path| DelegateInputRef {
            kind: "write_path".to_string(),
            value: path.trim().to_string(),
            description: None,
        })
        .chain(
            feature
                .depends_on
                .iter()
                .map(|feature_id| DelegateInputRef {
                    kind: "depends_on".to_string(),
                    value: feature_id.trim().to_string(),
                    description: None,
                }),
        )
        .collect::<Vec<_>>();

    let expected_outputs = feature
        .expected_behavior
        .iter()
        .map(|value| ExpectedOutputRef {
            kind: "expected_behavior".to_string(),
            value: value.trim().to_string(),
            description: None,
        })
        .chain(
            feature
                .verification_steps
                .iter()
                .map(|value| ExpectedOutputRef {
                    kind: "verification_step".to_string(),
                    value: value.trim().to_string(),
                    description: None,
                }),
        )
        .collect::<Vec<_>>();

    DelegateRequest {
        delegate_id: worker_id.to_string(),
        parent_session_id: start_cfg.parent_session_id.clone().unwrap_or_default(),
        parent_turn_id: start_cfg.parent_turn_id,
        parent_task_id: feature.id.clone(),
        job_id: mission_id.to_string(),
        goal: feature.description.clone(),
        input_refs,
        expected_outputs,
        selected_profile_id: agent_profile.name.clone(),
        resource_locks: super::core::feature_resource_locks(feature),
        session_source: crate::mission::agent_profile::SessionSource::WorkflowJob,
    }
    .normalized()
}

fn build_delegate_run_context(
    mission_id: &str,
    project_path_bg: &str,
    worker_id: &str,
    feature: &Feature,
    agent_profile: &AgentProfile,
    start_cfg: &MissionStartConfig,
) -> DelegateRunContext {
    let project_path = std::path::Path::new(project_path_bg);
    let mission_dir = artifacts::mission_dir(project_path, mission_id)
        .to_string_lossy()
        .to_string();

    DelegateRunContext {
        request: build_delegate_request(mission_id, feature, worker_id, agent_profile, start_cfg),
        role_profile: RoleProfile::from(agent_profile),
        project_path: project_path_bg.to_string(),
        mission_dir,
        mission_id: mission_id.to_string(),
        actor_id: worker_id.to_string(),
        provider: start_cfg.run_config.provider.clone(),
        model: start_cfg.run_config.model.clone(),
        base_url: start_cfg.run_config.base_url.clone(),
        api_key: start_cfg.run_config.api_key.clone(),
    }
    .normalized()
}

#[allow(clippy::too_many_arguments)]
async fn handle_in_process_delegate_run_result(
    app_handle: &tauri::AppHandle,
    orch_bg: &Orchestrator<'_>,
    emitter_bg: &MissionEventEmitter,
    mission_id: &str,
    project_path_bg: &str,
    worker_id: &str,
    feature_id: &str,
    session_id: String,
    start_cfg_bg: &MissionStartConfig,
    signals: &InProcessDelegateSignals,
    run_result: Result<crate::mission::delegate_types::DelegateResult, AppError>,
) {
    if signals.ignore_result.load(Ordering::SeqCst) {
        clear_worker_from_state(std::path::Path::new(project_path_bg), mission_id, worker_id);
        return;
    }

    let state_now = orch_bg.get_state().ok().map(|state| state.state);
    if !mission_state_accepts_delegate_result(state_now.clone()) {
        signals.ignore_result.store(true, Ordering::SeqCst);
        clear_worker_from_state(std::path::Path::new(project_path_bg), mission_id, worker_id);

        let state_label = state_now
            .as_ref()
            .map(mission_state_label)
            .unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "ignored in-process delegate completion after mission entered {state_label}: {worker_id}"
        );
        append_mission_recovery_log(std::path::Path::new(project_path_bg), mission_id, &message);
        let _ = emitter_bg.progress_entry(&message);
        return;
    }

    match run_result {
        Ok(delegate_result) => {
            let completed = FeatureCompletedPayload {
                feature_id: feature_id.to_string(),
                session_id,
                result: delegate_result.into_agent_task_result(),
            };
            handle_feature_completed_payload(
                app_handle,
                orch_bg,
                emitter_bg,
                mission_id,
                project_path_bg,
                worker_id,
                start_cfg_bg,
                completed,
            )
            .await;
        }
        Err(error) => {
            let error_message = error.to_string();
            append_mission_recovery_log(
                std::path::Path::new(project_path_bg),
                mission_id,
                format!("in-process delegate runtime failed ({worker_id}): {error_message}"),
            );
            let _ = emitter_bg.progress_entry(&format!(
                "in-process delegate failed for {feature_id}: {error_message}"
            ));

            mark_active_feature_failed(
                orch_bg,
                std::path::Path::new(project_path_bg),
                mission_id,
                worker_id,
                DELEGATE_RUNTIME_FAILURE_CODE,
                &format!("in-process delegate runtime failed: {error_message}"),
            );
            clear_worker_from_state(std::path::Path::new(project_path_bg), mission_id, worker_id);
            paused_config_registry().insert(mission_id.to_string(), start_cfg_bg.clone());
            pause_mission_with_log(
                orch_bg,
                emitter_bg,
                std::path::Path::new(project_path_bg),
                mission_id,
                start_cfg_bg,
                format!("in-process delegate runtime failed: {error_message}"),
            );
        }
    }
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
    let cancel_token = CancellationToken::new();
    let ignore_result = Arc::new(AtomicBool::new(false));
    let signals = Arc::new(InProcessDelegateSignals::new(Arc::clone(&ignore_result)));
    super::register_in_process_delegate(
        &mission_id,
        &worker_id,
        &feature.id,
        cancel_token.clone(),
        Arc::clone(&ignore_result),
    );
    spawn_in_process_delegate_watchdog_task(
        app_handle.clone(),
        mission_id.clone(),
        project_path_bg.clone(),
        worker_id.clone(),
        Arc::clone(&signals),
    );

    tokio::spawn(async move {
        let orch_bg = Orchestrator::new(std::path::Path::new(&project_path_bg), mission_id.clone());
        let emitter_bg = MissionEventEmitter::new(app_handle.clone(), mission_id.clone());
        let session_id =
            super::core::feature_session_id(&mission_id, &feature.id, &worker_id, attempt);
        let context = build_delegate_run_context(
            &mission_id,
            &project_path_bg,
            &worker_id,
            &feature,
            &agent_profile,
            &start_cfg_bg,
        );
        let runner = InProcessDelegateRunner::new().with_cancel_token(cancel_token);

        let _ =
            emitter_bg.progress_entry(&format!("running in-process delegate for {}", feature.id));

        let run_result = runner.run_delegate(context).await;
        signals.finished.store(true, Ordering::SeqCst);
        let _ = super::unregister_in_process_delegate(&mission_id, &worker_id);

        let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;
        handle_in_process_delegate_run_result(
            &app_handle,
            &orch_bg,
            &emitter_bg,
            &mission_id,
            &project_path_bg,
            &worker_id,
            &feature.id,
            session_id,
            &start_cfg_bg,
            &signals,
            run_result,
        )
        .await;
    });
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
    spawn_worker_heartbeat_task(
        app_handle.clone(),
        mission_id.clone(),
        project_path_bg.clone(),
        worker_id.clone(),
        start_cfg_bg.clone(),
    );

    tokio::spawn(async move {
        let orch_bg = Orchestrator::new(std::path::Path::new(&project_path_bg), mission_id.clone());
        let emitter_bg = MissionEventEmitter::new(app_handle.clone(), mission_id.clone());
        let session_id =
            super::core::feature_session_id(&mission_id, &feature.id, &worker_id, attempt);
        let context = build_delegate_run_context(
            &mission_id,
            &project_path_bg,
            &worker_id,
            &feature,
            &agent_profile,
            &start_cfg_bg,
        );
        let runner = ProcessDelegateRunner::new(
            AttachedWorkerProcessTransport::new(
                worker.clone(),
                app_handle.clone(),
                mission_id.clone(),
                worker_id.clone(),
            )
            .with_completion_timeout(Duration::from_secs(15 * 60)),
        );

        let run_result = runner.run_delegate(context).await;
        let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;

        if get_worker_handle(&mission_id, &worker_id).is_none() {
            clear_worker_from_state(
                std::path::Path::new(&project_path_bg),
                &mission_id,
                &worker_id,
            );
            return;
        }

        match run_result {
            Ok(delegate_result) => {
                let completed = FeatureCompletedPayload {
                    feature_id: feature.id.clone(),
                    session_id,
                    result: delegate_result.into_agent_task_result(),
                };
                handle_feature_completed_payload(
                    &app_handle,
                    &orch_bg,
                    &emitter_bg,
                    &mission_id,
                    &project_path_bg,
                    &worker_id,
                    &start_cfg_bg,
                    completed,
                )
                .await;
            }
            Err(error) => {
                let state_now = orch_bg.get_state().ok().map(|state| state.state);
                if !matches!(
                    state_now,
                    Some(
                        MissionState::Running
                            | MissionState::OrchestratorTurn
                            | MissionState::Paused
                    )
                ) {
                    let _ = remove_worker_handle(&mission_id, &worker_id);
                    clear_worker_from_state(
                        std::path::Path::new(&project_path_bg),
                        &mission_id,
                        &worker_id,
                    );
                    return;
                }

                append_mission_recovery_log(
                    std::path::Path::new(&project_path_bg),
                    &mission_id,
                    format!("delegate runtime failed ({worker_id}): {error}"),
                );

                let recovery_ctx = WorkerRecoveryContext {
                    mission_id: mission_id.clone(),
                    project_path: project_path_bg.clone(),
                    worker_id: worker_id.clone(),
                    feature_id: feature.id.clone(),
                    run_config: start_cfg_bg.run_config.clone(),
                    attempt,
                };

                match try_recover_worker(&orch_bg, &emitter_bg, &recovery_ctx).await {
                    Ok(Some((new_worker_id, _new_worker_arc))) => {
                        spawn_worker_supervision_tasks(
                            app_handle.clone(),
                            mission_id.clone(),
                            project_path_bg.clone(),
                            new_worker_id,
                            start_cfg_bg.run_config.clone(),
                            start_cfg_bg.max_workers,
                        );
                    }
                    Ok(None) => {
                        mark_active_feature_failed(
                            &orch_bg,
                            std::path::Path::new(&project_path_bg),
                            &mission_id,
                            &worker_id,
                            DELEGATE_RUNTIME_FAILURE_CODE,
                            &format!("delegate runtime failed and recovery exhausted: {error}"),
                        );
                        let _ = remove_worker_handle(&mission_id, &worker_id);
                        paused_config_registry().insert(mission_id.clone(), start_cfg_bg.clone());
                        pause_mission_with_log(
                            &orch_bg,
                            &emitter_bg,
                            std::path::Path::new(&project_path_bg),
                            &mission_id,
                            &start_cfg_bg,
                            format!("delegate runtime failed and recovery exhausted: {error}"),
                        );
                    }
                    Err(recovery_error) => {
                        mark_active_feature_failed(
                            &orch_bg,
                            std::path::Path::new(&project_path_bg),
                            &mission_id,
                            &worker_id,
                            DELEGATE_RUNTIME_FAILURE_CODE,
                            &format!("delegate runtime recovery failed: {recovery_error}"),
                        );
                        let _ = remove_worker_handle(&mission_id, &worker_id);
                        paused_config_registry().insert(mission_id.clone(), start_cfg_bg.clone());
                        pause_mission_with_log(
                            &orch_bg,
                            &emitter_bg,
                            std::path::Path::new(&project_path_bg),
                            &mission_id,
                            &start_cfg_bg,
                            format!("delegate runtime recovery failed: {recovery_error}"),
                        );
                    }
                }
            }
        }
    });
}

pub(super) fn spawn_worker_supervision_tasks(
    app_handle: tauri::AppHandle,
    mission_id: String,
    project_path_bg: String,
    worker_id_bg: String,
    run_config_bg: MissionRunConfig,
    max_workers_bg: usize,
) {
    spawn_worker_heartbeat_task(
        app_handle.clone(),
        mission_id.clone(),
        project_path_bg.clone(),
        worker_id_bg.clone(),
        MissionStartConfig {
            run_config: run_config_bg.clone(),
            max_workers: max_workers_bg,
            parent_session_id: None,
            parent_turn_id: None,
            delegate_transport: super::super::DelegateTransportMode::Process,
        },
    );

    // Worker event monitor task
    tokio::spawn(async move {
        let worker_id = worker_id_bg;
        let orch_bg = Orchestrator::new(std::path::Path::new(&project_path_bg), mission_id.clone());
        let emitter_bg = MissionEventEmitter::new(app_handle.clone(), mission_id.clone());
        let start_cfg_bg = MissionStartConfig {
            run_config: run_config_bg.clone(),
            max_workers: max_workers_bg,
            parent_session_id: None,
            parent_turn_id: None,
            delegate_transport: super::super::DelegateTransportMode::Process,
        };

        loop {
            let registry_handle = match get_worker_handle(&mission_id, &worker_id) {
                Some(h) => h,
                None => break,
            };
            let recovery_attempt = registry_handle.attempt;

            let event_result = {
                let w = registry_handle.worker.lock().await;
                w.recv().await
            };

            match event_result {
                None => {
                    let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;

                    // If the worker handle was already removed (e.g. intentional stop),
                    // do not attempt recovery.
                    if get_worker_handle(&mission_id, &worker_id).is_none() {
                        tracing::info!(
                            target: "mission",
                            mission_id = %mission_id,
                            worker_id = %worker_id,
                            "worker stdout closed after handle removal; skipping recovery"
                        );
                        break;
                    }
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
                Some(Ok(worker_event)) => {
                    match worker_event.event_type {
                        WorkerEventType::AgentEvent => {
                            use tauri::Emitter;
                            let enriched = enrich_worker_agent_event_payload(
                                worker_event.payload,
                                &mission_id,
                                &worker_id,
                            );
                            let _ = app_handle
                                .emit(crate::agent_engine::events::AGENT_EVENT_CHANNEL, &enriched);
                        }
                        WorkerEventType::FeatureCompleted => {
                            let _runtime_lock = acquire_mission_runtime_lock(&mission_id).await;

                            match serde_json::from_value::<FeatureCompletedPayload>(
                                worker_event.payload.clone(),
                            ) {
                                Ok(completed) => {
                                    let effective_ok = completed.result.is_ok();
                                    let effective_summary = completed.result_summary();
                                    let stop_reason = format!("{:?}", completed.result.stop_reason);
                                    let changed_path_count = completed.result.changed_paths.len();
                                    let open_issue_count = completed.result.open_issues.len();
                                    tracing::info!(
                                        target: "mission",
                                        mission_id = %mission_id,
                                        worker_id = %worker_id,
                                        feature_id = %completed.feature_id,
                                        ok = effective_ok,
                                        stop_reason = %stop_reason,
                                        changed_paths = changed_path_count,
                                        open_issues = open_issue_count,
                                        "worker reported feature completed"
                                    );

                                    let _ = emitter_bg.worker_completed(
                                        &worker_id,
                                        &completed.feature_id,
                                        effective_ok,
                                        &effective_summary,
                                    );

                                    if open_issue_count > 0 {
                                        let _ = emitter_bg.progress_entry(&format!(
                                            "Worker reported {} open issue(s) for {}",
                                            open_issue_count, completed.feature_id
                                        ));
                                    }

                                    match orch_bg.complete_feature_result(
                                        &completed.feature_id,
                                        &worker_id,
                                        &completed.result,
                                    ) {
                                        Ok(next_state) => {
                                            // M5: update macro state on feature completion
                                            let completion_status = if effective_ok {
                                                FeatureStatus::Completed
                                            } else {
                                                FeatureStatus::Failed
                                            };
                                            let _ = super::super::macro_commands::update_macro_state_on_feature_event(
                                                std::path::Path::new(&project_path_bg),
                                                &mission_id,
                                                &completed.feature_id,
                                                &completion_status,
                                                Some(&emitter_bg),
                                            );

                                            let next_state_str = serde_json::to_string(&next_state)
                                                .unwrap_or_default()
                                                .trim_matches('"')
                                                .to_string();
                                            let _ = emitter_bg
                                                .state_changed("running", &next_state_str);

                                            let _ = remove_worker_handle(&mission_id, &worker_id);
                                            clear_worker_from_state(
                                                std::path::Path::new(&project_path_bg),
                                                &mission_id,
                                                &worker_id,
                                            );

                                            // ── M3 Review Gate: run after chapter-level writes ─
                                            let mut gate_blocked = false;
                                            if effective_ok {
                                                if let Ok(features_doc) = orch_bg.get_features() {
                                                    let feature_opt = features_doc
                                                        .features
                                                        .iter()
                                                        .find(|f| f.id == completed.feature_id)
                                                        .cloned();
                                                    if let Some(feature) = feature_opt {
                                                        if !feature.write_paths.is_empty() {
                                                            let chapter_targets =
                                                                filter_chapter_write_targets(
                                                                    std::path::Path::new(
                                                                        &project_path_bg,
                                                                    ),
                                                                    &feature.write_paths,
                                                                );

                                                            if chapter_targets.is_empty() {
                                                                // Non-chapter writes should not trigger chapter-level ReviewGate.
                                                            } else {
                                                                let _ = emitter_bg.progress_entry(
                                                                "ReviewGate: running review after chapter write...",
                                                            );

                                                                let scope_ref =
                                                                    infer_review_scope_ref(
                                                                        std::path::Path::new(
                                                                            &project_path_bg,
                                                                        ),
                                                                        &mission_id,
                                                                    );
                                                                let gate_policy =
                                                                    resolve_chapter_gate_policy(
                                                                        std::path::Path::new(
                                                                            &project_path_bg,
                                                                        ),
                                                                        &mission_id,
                                                                        &scope_ref,
                                                                        false,
                                                                        true,
                                                                    );

                                                                match run_review_gate_with_p1_policies(
                                                                std::path::Path::new(&project_path_bg),
                                                                &mission_id,
                                                                scope_ref,
                                                                chapter_targets,
                                                                gate_policy.clone(),
                                                                None,
                                                                Some(&start_cfg_bg.run_config),
                                                            ).await {
                                                                Ok((report, meta)) => {
                                                                    if meta.staleness.stale {
                                                                        let _ = emitter_bg.progress_entry(
                                                                            "ReviewGate: contextpack was stale and review inputs were refreshed",
                                                                        );
                                                                    }
                                                                    if meta.rebuilt {
                                                                        if let Some(cp) =
                                                                            meta.contextpack.as_ref()
                                                                        {
                                                                            let _ = emitter_bg.contextpack_built(
                                                                                report.scope_ref.as_str(),
                                                                                token_budget_as_str(&cp.token_budget),
                                                                                cp.generated_at,
                                                                            );
                                                                        }
                                                                    }
                                                                    if let Err(e) = persist_review_report(
                                                                        std::path::Path::new(&project_path_bg),
                                                                        &mission_id,
                                                                        &report,
                                                                    ) {
                                                                        gate_blocked = true;
                                                                        pause_mission_with_log(
                                                                            &orch_bg,
                                                                            &emitter_bg,
                                                                            std::path::Path::new(&project_path_bg),
                                                                            &mission_id,
                                                                            &start_cfg_bg,
                                                                            format!(
                                                                                "ReviewGate failed to persist report: {e}"
                                                                            ),
                                                                        );
                                                                    } else {
                                                                        if let Err(e) = upsert_risk_ledger_from_review(
                                                                            std::path::Path::new(&project_path_bg),
                                                                            &mission_id,
                                                                            &report,
                                                                        ) {
                                                                            tracing::warn!(
                                                                                target: "mission",
                                                                                mission_id = %mission_id,
                                                                                error = %e,
                                                                                "failed to write risk_ledger from review"
                                                                            );
                                                                        } else {
                                                                            let _ = emitter_bg
                                                                                .layer1_updated("risk_ledger");
                                                                        }

                                                                        let _ =
                                                                            emitter_bg.review_recorded(&report);

                                                                        let strict_warn_block = gate_policy.strict_warn
                                                                            && report.overall_status
                                                                                == review_types::ReviewOverallStatus::Warn;

                                                                        match report.overall_status {
                                                                            review_types::ReviewOverallStatus::Pass
                                                                            | review_types::ReviewOverallStatus::Warn => {
                                                                                if strict_warn_block {
                                                                                    gate_blocked = true;

                                                                                    // Stop other workers and pause scheduling.
                                                                                    stop_all_workers_for_review_block(
                                                                                        std::path::Path::new(&project_path_bg),
                                                                                        &mission_id,
                                                                                        Some(&worker_id),
                                                                                    )
                                                                                    .await;

                                                                                    let mut strict_report = report.clone();
                                                                                    for issue in &mut strict_report.issues {
                                                                                        if issue.severity
                                                                                            == review_types::ReviewSeverity::Warn
                                                                                        {
                                                                                            issue.severity =
                                                                                                review_types::ReviewSeverity::Block;
                                                                                        }
                                                                                    }
                                                                                    strict_report.overall_status =
                                                                                        review_types::ReviewOverallStatus::Block;

                                                                                    let auto_fix_eligible =
                                                                                        review_block_is_auto_fixable(&strict_report)
                                                                                            && !review_has_non_auto_fixable_block(
                                                                                                &strict_report,
                                                                                            );

                                                                                    if gate_policy.auto_fix_on_block
                                                                                        && auto_fix_eligible
                                                                                    {
                                                                                        match start_review_fixup_attempt(
                                                                                            app_handle.clone(),
                                                                                            &orch_bg,
                                                                                            &emitter_bg,
                                                                                            &start_cfg_bg,
                                                                                            std::path::Path::new(&project_path_bg),
                                                                                            &project_path_bg,
                                                                                            &mission_id,
                                                                                            &completed.feature_id,
                                                                                            &strict_report,
                                                                                        )
                                                                                        .await
                                                                                        {
                                                                                            Ok(()) => {
                                                                                                let _ = emitter_bg.progress_entry(
                                                                                                    "ReviewGate: strict_warn warnings triggered fixup",
                                                                                                );
                                                                                            }
                                                                                            Err(e) => {
                                                                                                let mut req = build_review_decision_request(
                                                                                                    &strict_report,
                                                                                                    Some(completed.feature_id.clone()),
                                                                                                );
                                                                                                if !auto_fix_eligible {
                                                                                                    req.options.retain(|o| o != "auto_fix");
                                                                                                }
                                                                                                let _ = artifacts::write_pending_review_decision(
                                                                                                    std::path::Path::new(&project_path_bg),
                                                                                                    &mission_id,
                                                                                                    &req,
                                                                                                );
                                                                                                let _ = emitter_bg.review_decision_required(&req);
                                                                                                pause_mission_with_log(
                                                                                                    &orch_bg,
                                                                                                    &emitter_bg,
                                                                                                    std::path::Path::new(&project_path_bg),
                                                                                                    &mission_id,
                                                                                                    &start_cfg_bg,
                                                                                                    format!(
                                                                                                        "ReviewGate warn (strict_warn) blocked; auto fixup failed: {e}"
                                                                                                    ),
                                                                                                );
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        let mut req = build_review_decision_request(
                                                                                            &strict_report,
                                                                                            Some(completed.feature_id.clone()),
                                                                                        );
                                                                                        if !auto_fix_eligible {
                                                                                            req.options.retain(|o| o != "auto_fix");
                                                                                        }
                                                                                        let _ = artifacts::write_pending_review_decision(
                                                                                            std::path::Path::new(&project_path_bg),
                                                                                            &mission_id,
                                                                                            &req,
                                                                                        );
                                                                                        let _ = emitter_bg.review_decision_required(&req);
                                                                                        pause_mission_with_log(
                                                                                            &orch_bg,
                                                                                            &emitter_bg,
                                                                                            std::path::Path::new(&project_path_bg),
                                                                                            &mission_id,
                                                                                            &start_cfg_bg,
                                                                                            "ReviewGate warn (strict_warn) blocked; user decision required",
                                                                                        );
                                                                                    }
                                                                                } else {
                                                                                // Clear any pending block state.
                                                                                review_fixup_registry()
                                                                                    .remove(mission_id.as_str());
                                                                                let _ = artifacts::clear_pending_review_decision(
                                                                                    std::path::Path::new(&project_path_bg),
                                                                                    &mission_id,
                                                                                );

                                                                                // ── M4 Knowledge Writeback: propose → gate (safe auto-accept) ─
                                                                                // Contract: must run after ReviewGate and must persist artifacts.
                                                                                let project_path = std::path::Path::new(&project_path_bg);
                                                                                let source_session_id = completed.session_id.trim().to_string();
                                                                                let bundle = if source_session_id.is_empty() {
                                                                                    gate_blocked = true;
                                                                                    pause_mission_with_log(
                                                                                        &orch_bg,
                                                                                        &emitter_bg,
                                                                                        project_path,
                                                                                        &mission_id,
                                                                                        &start_cfg_bg,
                                                                                        "KnowledgeWriteback aborted: worker completed payload missing session_id",
                                                                                    );
                                                                                    None
                                                                                } else {
                                                                                    match knowledge_writeback::generate_proposal_bundle_after_closeout(
                                                                                        project_path,
                                                                                        &mission_id,
                                                                                        report.scope_ref.clone(),
                                                                                        feature.write_paths.clone(),
                                                                                        source_session_id,
                                                                                        Some(report.review_id.clone()),
                                                                                    ) {
                                                                                        Ok(b) => Some(b),
                                                                                        Err(e) => {
                                                                                            gate_blocked = true;
                                                                                            pause_mission_with_log(
                                                                                                &orch_bg,
                                                                                                &emitter_bg,
                                                                                                project_path,
                                                                                                &mission_id,
                                                                                                &start_cfg_bg,
                                                                                                format!(
                                                                                                    "KnowledgeWriteback failed to generate proposals: {e}"
                                                                                                ),
                                                                                            );
                                                                                            None
                                                                                        }
                                                                                    }
                                                                                };

                                                                                if let Some(bundle) = bundle {
                                                                                    let delta = match knowledge_writeback::gate_bundle(
                                                                                        project_path,
                                                                                        &bundle,
                                                                                        Some(&report),
                                                                                    ) {
                                                                                        Ok(d) => Some(d),
                                                                                        Err(e) => {
                                                                                            gate_blocked = true;
                                                                                            pause_mission_with_log(
                                                                                                &orch_bg,
                                                                                                &emitter_bg,
                                                                                                project_path,
                                                                                                &mission_id,
                                                                                                &start_cfg_bg,
                                                                                                format!(
                                                                                                    "KnowledgeWriteback failed to gate proposals: {e}"
                                                                                                ),
                                                                                            );
                                                                                            None
                                                                                        }
                                                                                    };

                                                                                    if let Some(delta) = delta {
                                                                                        if let Err(e) = artifacts::write_knowledge_bundle_latest(
                                                                                            project_path,
                                                                                            &mission_id,
                                                                                            &bundle,
                                                                                        ) {
                                                                                            gate_blocked = true;
                                                                                            pause_mission_with_log(
                                                                                                &orch_bg,
                                                                                                &emitter_bg,
                                                                                                project_path,
                                                                                                &mission_id,
                                                                                                &start_cfg_bg,
                                                                                                format!(
                                                                                                    "KnowledgeWriteback failed to persist bundle: {e}"
                                                                                                ),
                                                                                            );
                                                                                        } else {
                                                                                            let _ = artifacts::append_knowledge_bundle(
                                                                                                project_path,
                                                                                                &mission_id,
                                                                                                &bundle,
                                                                                            );
                                                                                        }

                                                                                        if !gate_blocked {
                                                                                            if let Err(e) = artifacts::write_knowledge_delta_latest(
                                                                                                project_path,
                                                                                                &mission_id,
                                                                                                &delta,
                                                                                            ) {
                                                                                                gate_blocked = true;
                                                                                                pause_mission_with_log(
                                                                                                    &orch_bg,
                                                                                                    &emitter_bg,
                                                                                                    project_path,
                                                                                                    &mission_id,
                                                                                                    &start_cfg_bg,
                                                                                                    format!(
                                                                                                        "KnowledgeWriteback failed to persist delta: {e}"
                                                                                                    ),
                                                                                                );
                                                                                            } else {
                                                                                                let _ = artifacts::append_knowledge_delta(
                                                                                                    project_path,
                                                                                                    &mission_id,
                                                                                                    &delta,
                                                                                                );
                                                                                            }
                                                                                        }

                                                                                        if !gate_blocked {
                                                                                            let _ = agent_session::record_canon_updates_proposed(
                                                                                                project_path,
                                                                                                &bundle.source_session_id,
                                                                                                bundle.branch_id.as_ref(),
                                                                                                CanonUpdatesProposedEntry {
                                                                                                    bundle_id: bundle.bundle_id.clone(),
                                                                                                    delta_id: delta.knowledge_delta_id.clone(),
                                                                                                    scope_ref: bundle.scope_ref.clone(),
                                                                                                    kinds: knowledge_writeback::proposal_kinds(&bundle),
                                                                                                    ts: bundle.generated_at,
                                                                                                },
                                                                                            );

                                                                                            // Best-effort: notify UI.
                                                                                            let _ = emitter_bg.knowledge_proposed(&bundle);

                                                                                            if !delta.conflicts.is_empty() {
                                                                                                gate_blocked = true;

                                                                                                let pending = knowledge_writeback::build_pending_decision(&bundle, &delta);
                                                                                                let _ = artifacts::write_pending_knowledge_decision(
                                                                                                    project_path,
                                                                                                    &mission_id,
                                                                                                    &pending,
                                                                                                );
                                                                                                let _ = emitter_bg.knowledge_decision_required(&delta);

                                                                                                pause_mission_with_log(
                                                                                                    &orch_bg,
                                                                                                    &emitter_bg,
                                                                                                    project_path,
                                                                                                    &mission_id,
                                                                                                    &start_cfg_bg,
                                                                                                    "KnowledgeWriteback blocked: conflicts detected; user decision required",
                                                                                                );
                                                                                            } else {
                                                                                                // No conflicts: clear any stale pending decision marker.
                                                                                                let _ = artifacts::clear_pending_knowledge_decision(
                                                                                                    project_path,
                                                                                                    &mission_id,
                                                                                                );

                                                                                                // Default writeback: auto-apply when the gate fully accepted.
                                                                                                if delta.status == knowledge_types::KnowledgeDeltaStatus::Accepted {
                                                                                                    match knowledge_writeback::apply_accepted(
                                                                                                        project_path,
                                                                                                        &mission_id,
                                                                                                        &bundle,
                                                                                                        &delta,
                                                                                                        knowledge_types::KnowledgeDecisionActor::Orchestrator,
                                                                                                    ) {
                                                                                                        Ok(applied) => {
                                                                                                            if let Err(e) = artifacts::write_knowledge_delta_latest(
                                                                                                                project_path,
                                                                                                                &mission_id,
                                                                                                                &applied,
                                                                                                            ) {
                                                                                                                gate_blocked = true;
                                                                                                                pause_mission_with_log(
                                                                                                                    &orch_bg,
                                                                                                                    &emitter_bg,
                                                                                                                    project_path,
                                                                                                                    &mission_id,
                                                                                                                    &start_cfg_bg,
                                                                                                                    format!(
                                                                                                                        "KnowledgeWriteback applied but failed to persist delta: {e}"
                                                                                                                    ),
                                                                                                                );
                                                                                                            } else {
                                                                                                                let _ = artifacts::append_knowledge_delta(
                                                                                                                    project_path,
                                                                                                                    &mission_id,
                                                                                                                    &applied,
                                                                                                                );
                                                                                                                let targets = knowledge_writeback::accepted_target_refs(&bundle, &applied);
                                                                                                                let _ = agent_session::record_canon_updates_accepted(
                                                                                                                    project_path,
                                                                                                                    &bundle.source_session_id,
                                                                                                                    bundle.branch_id.as_ref(),
                                                                                                                    CanonUpdatesAcceptedEntry {
                                                                                                                        delta_id: applied.knowledge_delta_id.clone(),
                                                                                                                        applied_at: applied.applied_at.unwrap_or_default(),
                                                                                                                        targets: targets.clone(),
                                                                                                                        rollback_token: applied
                                                                                                                            .rollback
                                                                                                                            .as_ref()
                                                                                                                            .and_then(|rb| rb.token.clone()),
                                                                                                                        rolled_back_at: None,
                                                                                                                    },
                                                                                                                    &targets,
                                                                                                                );
                                                                                                                let _ = emitter_bg.knowledge_applied(&applied);
                                                                                                            }
                                                                                                        }
                                                                                                        Err(e) => {
                                                                                                            gate_blocked = true;
                                                                                                            pause_mission_with_log(
                                                                                                                &orch_bg,
                                                                                                                &emitter_bg,
                                                                                                                project_path,
                                                                                                                &mission_id,
                                                                                                                &start_cfg_bg,
                                                                                                                format!(
                                                                                                                    "KnowledgeWriteback failed to apply accepted delta: {e}"
                                                                                                                ),
                                                                                                            );
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                }
                                                                            }
                                                                            review_types::ReviewOverallStatus::Block => {
                                                                                gate_blocked = true;

                                                                                // Stop other workers and pause scheduling.
                                                                                stop_all_workers_for_review_block(
                                                                                    std::path::Path::new(&project_path_bg),
                                                                                    &mission_id,
                                                                                    Some(&worker_id),
                                                                                )
                                                                                .await;

                                                                                let auto_fix_eligible =
                                                                                    review_block_is_auto_fixable(&report)
                                                                                        && !review_has_non_auto_fixable_block(
                                                                                            &report,
                                                                                        );

                                                                                if gate_policy.auto_fix_on_block
                                                                                    && auto_fix_eligible
                                                                                {
                                                                                    match start_review_fixup_attempt(
                                                                                        app_handle.clone(),
                                                                                        &orch_bg,
                                                                                        &emitter_bg,
                                                                                        &start_cfg_bg,
                                                                                        std::path::Path::new(&project_path_bg),
                                                                                        &project_path_bg,
                                                                                        &mission_id,
                                                                                        &completed.feature_id,
                                                                                        &report,
                                                                                    )
                                                                                    .await
                                                                                    {
                                                                                        Ok(()) => {
                                                                                            let _ = emitter_bg.progress_entry(
                                                                                                "ReviewGate: auto fixup dispatched",
                                                                                            );
                                                                                        }
                                                                                        Err(e) => {
                                                                                            let req = build_review_decision_request(
                                                                                                &report,
                                                                                                Some(completed.feature_id.clone()),
                                                                                            );
                                                                                            let _ = artifacts::write_pending_review_decision(
                                                                                                std::path::Path::new(&project_path_bg),
                                                                                                &mission_id,
                                                                                                &req,
                                                                                            );
                                                                                            let _ = emitter_bg.review_decision_required(&req);
                                                                                            pause_mission_with_log(
                                                                                                &orch_bg,
                                                                                                &emitter_bg,
                                                                                                std::path::Path::new(&project_path_bg),
                                                                                                &mission_id,
                                                                                                &start_cfg_bg,
                                                                                                format!(
                                                                                                    "ReviewGate blocked; auto fixup failed: {e}"
                                                                                                ),
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                } else {
                                                                                    let mut req = build_review_decision_request(
                                                                                        &report,
                                                                                        Some(completed.feature_id.clone()),
                                                                                    );
                                                                                    if !auto_fix_eligible {
                                                                                        // If auto-fix isn't eligible, remove the option.
                                                                                        req.options.retain(|o| o != "auto_fix");
                                                                                    }
                                                                                    let _ = artifacts::write_pending_review_decision(
                                                                                        std::path::Path::new(&project_path_bg),
                                                                                        &mission_id,
                                                                                        &req,
                                                                                    );
                                                                                    let _ = emitter_bg.review_decision_required(&req);
                                                                                    pause_mission_with_log(
                                                                                        &orch_bg,
                                                                                        &emitter_bg,
                                                                                        std::path::Path::new(&project_path_bg),
                                                                                        &mission_id,
                                                                                        &start_cfg_bg,
                                                                                        "ReviewGate blocked; user decision required",
                                                                                    );
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    gate_blocked = true;
                                                                    pause_mission_with_log(
                                                                        &orch_bg,
                                                                        &emitter_bg,
                                                                        std::path::Path::new(&project_path_bg),
                                                                        &mission_id,
                                                                        &start_cfg_bg,
                                                                        format!("ReviewGate execution failed: {e}"),
                                                                    );
                                                                }
                                                            }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            let actual_state =
                                                orch_bg.get_state().ok().map(|s| s.state);
                                            let should_schedule = matches!(
                                                actual_state,
                                                Some(
                                                    MissionState::Running
                                                        | MissionState::OrchestratorTurn
                                                )
                                            );

                                            if !gate_blocked
                                                && should_schedule
                                                && (next_state == MissionState::OrchestratorTurn
                                                    || next_state == MissionState::Running)
                                            {
                                                let _ = emitter_bg.progress_entry(
                                                    "Feature completed, scheduling next feature...",
                                                );

                                                match super::core::schedule_ready_features(
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
                                                            let finished = orch_bg
                                                                .is_finished()
                                                                .unwrap_or(false);
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
                                                            let _ = emitter_bg.progress_entry(
                                                                &format!(
                                                                    "Started next features: {}",
                                                                    started.join(", ")
                                                                ),
                                                            );
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
                                super::core::update_worker_assignment_heartbeat(
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
                    }
                }
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

fn enrich_worker_agent_event_payload(
    mut payload: serde_json::Value,
    mission_id: &str,
    worker_id: &str,
) -> serde_json::Value {
    let Some(obj) = payload.as_object_mut() else {
        return payload;
    };

    // Ensure source object exists.
    let source = obj
        .entry("source".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !source.is_object() {
        *source = serde_json::json!({});
    }

    if let Some(src) = source.as_object_mut() {
        src.insert("kind".to_string(), serde_json::json!("worker"));
        src.insert("worker_id".to_string(), serde_json::json!(worker_id));
        src.insert("mission_id".to_string(), serde_json::json!(mission_id));
    }

    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_state_accepts_delegate_result_only_for_active_scheduler_states() {
        assert!(mission_state_accepts_delegate_result(Some(
            MissionState::Running
        )));
        assert!(mission_state_accepts_delegate_result(Some(
            MissionState::OrchestratorTurn
        )));

        for state in [
            MissionState::AwaitingInput,
            MissionState::Initializing,
            MissionState::Blocked,
            MissionState::WaitingUser,
            MissionState::WaitingReview,
            MissionState::WaitingKnowledgeDecision,
            MissionState::Paused,
            MissionState::Failed,
            MissionState::Completed,
            MissionState::Cancelled,
        ] {
            assert!(!mission_state_accepts_delegate_result(Some(state)));
        }

        assert!(!mission_state_accepts_delegate_result(None));
    }

    #[test]
    fn mission_state_label_uses_serialized_state_name() {
        assert_eq!(
            mission_state_label(&MissionState::WaitingReview),
            "waiting_review"
        );
        assert_eq!(
            mission_state_label(&MissionState::OrchestratorTurn),
            "orchestrator_turn"
        );
    }
}
