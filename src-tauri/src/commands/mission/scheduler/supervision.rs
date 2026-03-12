use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex as TokioMutex;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::{ProcessManager, WorkerProcess};
use crate::mission::types::*;
use crate::mission::worker_protocol::{FeatureCompletedPayload, WorkerEventType};
use crate::models::AppError;
use crate::services::global_config;

use crate::review::types as review_types;

use super::review_gate::*;
use super::runtime::*;
use super::{MissionRunConfig, MissionStartConfig};

const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_TIMEOUT_SECS: u64 = 20;
const WORKER_MAX_RECOVERY_ATTEMPTS: u32 = 2;
const WORKER_RECOVERY_BACKOFF_MS: u64 = 1500;

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

pub(super) fn spawn_worker_supervision_tasks(
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

                super::core::update_worker_assignment_heartbeat(
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
                                            if completed.ok {
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
                                                                let review_types =
                                                                    default_chapter_review_types(
                                                                        std::path::Path::new(
                                                                            &project_path_bg,
                                                                        ),
                                                                        &mission_id,
                                                                    );

                                                                match run_review_gate_with_p1_policies(
                                                                std::path::Path::new(&project_path_bg),
                                                                &mission_id,
                                                                scope_ref,
                                                                chapter_targets,
                                                                review_types,
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

                                                                        match report.overall_status {
                                                                            review_types::ReviewOverallStatus::Pass
                                                                            | review_types::ReviewOverallStatus::Warn => {
                                                                                // Clear any pending block state.
                                                                                review_fixup_registry()
                                                                                    .remove(mission_id.as_str());
                                                                                let _ = artifacts::clear_pending_review_decision(
                                                                                    std::path::Path::new(&project_path_bg),
                                                                                    &mission_id,
                                                                                );
                                                                            }
                                                                            review_types::ReviewOverallStatus::Block => {
                                                                                gate_blocked = true;

                                                                                // Stop other workers and pause scheduling.
                                                                                stop_all_workers_for_review_block(
                                                                                    std::path::Path::new(&project_path_bg),
                                                                                    &mission_id,
                                                                                    None,
                                                                                )
                                                                                .await;

                                                                                if review_block_is_auto_fixable(&report)
                                                                                    && !review_has_non_auto_fixable_block(&report)
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
                                                                                    // If auto-fix isn't eligible, remove the option.
                                                                                    req.options.retain(|o| o != "auto_fix");
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
