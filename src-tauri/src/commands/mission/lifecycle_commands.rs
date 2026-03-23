//! Mission lifecycle Tauri commands.

use std::collections::BTreeSet;
use std::time::Duration;

use tauri::command;

use crate::mission::artifacts;
use crate::mission::blockers::{WorkflowBlockerKind, WorkflowBlockersDoc};
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::mission::workflow_types::{
    MissionWorkflowKind, SummaryJobPolicy, WorkflowCreationReason,
};
use crate::models::AppError;
use crate::review::types as review_types;

use super::lifecycle::{interrupt_mission, recover_mission, resume_mission_with_config};
use super::review_gate::*;
use super::runtime::*;
use super::scheduler;
use super::{
    apply_summary_job_policy, dto, MissionControlInput, MissionCreateInput, MissionCreateOutput,
    MissionGetStatusInput, MissionGetStatusOutput, MissionListInput, MissionStartInput,
};

#[command]
pub async fn mission_create(input: MissionCreateInput) -> Result<MissionCreateOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);

    let mut features = input.features;
    let workflow_kind = input.workflow_kind.unwrap_or(MissionWorkflowKind::AdHoc);
    let creation_reason = input
        .creation_reason
        .unwrap_or(WorkflowCreationReason::ExplicitMissionRequest);
    let summary_job_policy = input
        .summary_job_policy
        .unwrap_or(SummaryJobPolicy::ParentSessionSummary);
    apply_summary_job_policy(&summary_job_policy, &mut features);

    let mission_id = Orchestrator::create_mission(
        project_path,
        &input.title,
        &input.mission_text,
        features,
        workflow_kind,
        creation_reason,
        summary_job_policy,
    )?;
    let workflow = artifacts::read_workflow(project_path, &mission_id)?;

    Ok(MissionCreateOutput {
        schema_version: MISSION_SCHEMA_VERSION,
        mission_id,
        workflow,
    })
}

#[command]
pub async fn mission_list(input: MissionListInput) -> Result<Vec<String>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::list_missions(project_path)
}

fn resume_blocker_error(blockers: &WorkflowBlockersDoc) -> Option<AppError> {
    if blockers.blockers.is_empty() {
        return None;
    }

    if blockers
        .blockers
        .iter()
        .any(|blocker| blocker.kind == WorkflowBlockerKind::ReviewGate)
    {
        return Some(AppError::invalid_argument(
            "mission blocked: pending review decision required",
        ));
    }

    if blockers
        .blockers
        .iter()
        .any(|blocker| blocker.kind == WorkflowBlockerKind::KnowledgeDecision)
    {
        return Some(AppError::invalid_argument(
            "mission blocked: pending knowledge decision required",
        ));
    }

    if blockers
        .blockers
        .iter()
        .any(|blocker| blocker.kind == WorkflowBlockerKind::UserClarification)
    {
        return Some(AppError::invalid_argument(
            "mission blocked: pending user clarification required",
        ));
    }

    Some(AppError::invalid_argument(
        "mission blocked: external dependency required",
    ))
}

#[command]
pub async fn mission_get_status(
    input: MissionGetStatusInput,
) -> Result<MissionGetStatusOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let blockers = artifacts::refresh_workflow_blockers(project_path, &input.mission_id)?;
    let state = artifacts::read_state(project_path, &input.mission_id)?;
    let features = artifacts::read_features(project_path, &input.mission_id)?;
    let task_results = artifacts::read_task_results(project_path, &input.mission_id)?;
    let handoffs = artifacts::read_handoffs(project_path, &input.mission_id)?;
    let workflow = artifacts::read_workflow(project_path, &input.mission_id)?;
    let job_snapshot = artifacts::refresh_job_snapshot(project_path, &input.mission_id)?;
    let recovery_log = super::runtime::read_mission_recovery_log(project_path, &input.mission_id);

    Ok(MissionGetStatusOutput {
        state,
        features,
        task_results,
        handoffs,
        workflow,
        blockers,
        job_snapshot,
        recovery_log,
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

    let start_config = dto::resolve_start_config(&input)?;
    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    orch.transition(MissionState::Initializing)?;
    emitter.state_changed(&old_state_str, "initializing")?;

    let started = match scheduler::schedule_ready_features(
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
        delegate_transport = start_config.delegate_transport.as_str(),
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

    let active_feature_ids = current_state
        .assignments
        .values()
        .map(|assignment| assignment.feature_id.trim().to_string())
        .chain(
            current_state
                .current_feature_id
                .iter()
                .map(|feature_id| feature_id.trim().to_string()),
        )
        .filter(|feature_id| !feature_id.is_empty())
        .collect::<BTreeSet<_>>();

    let workers = list_worker_handles(&input.mission_id);
    let in_process_workers = scheduler::active_in_process_delegate_worker_ids(&input.mission_id);
    if workers.is_empty() && in_process_workers.is_empty() && active_feature_ids.is_empty() {
        return Err(AppError::invalid_argument(
            "mission has no active task to pause",
        ));
    }

    for feature_id in &active_feature_ids {
        orch.update_feature_status(feature_id, FeatureStatus::Pending)?;
        emitter.features_changed(feature_id, "pending")?;
    }

    for worker_id in scheduler::cancel_in_process_delegates(&input.mission_id, None, true) {
        clear_worker_from_state(project_path, &input.mission_id, &worker_id);
        append_mission_recovery_log(
            project_path,
            &input.mission_id,
            format!("mission_pause requested in-process delegate stop for {worker_id}"),
        );
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

    // M5: update macro state on pause
    super::macro_commands::update_macro_stage_on_lifecycle(
        project_path,
        &input.mission_id,
        crate::mission::macro_types::MacroStage::Blocked,
        Some(&emitter),
    );

    append_mission_recovery_log(project_path, &input.mission_id, "mission paused by user");

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        "mission paused"
    );

    Ok(())
}

#[command]
pub async fn mission_interrupt(
    app_handle: tauri::AppHandle,
    input: MissionControlInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    interrupt_mission(&orch, Some(&emitter), project_path, &input.mission_id).await
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
    if !matches!(
        current_state.state,
        MissionState::Paused
            | MissionState::Blocked
            | MissionState::WaitingUser
            | MissionState::WaitingReview
            | MissionState::WaitingKnowledgeDecision
    ) {
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

    let blockers = match artifacts::refresh_workflow_blockers(project_path, &input.mission_id) {
        Ok(blockers) => blockers,
        Err(e) => {
            paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
            return Err(e);
        }
    };
    if let Some(err) = resume_blocker_error(&blockers) {
        paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
        return Err(err);
    }

    // Gate: if latest review is still blocking, re-run review before resuming scheduling.
    if let Ok(Some(latest)) = artifacts::read_review_latest(project_path, &input.mission_id) {
        if latest.overall_status == review_types::ReviewOverallStatus::Block {
            let scope_ref = if latest.scope_ref.trim().is_empty() {
                infer_review_scope_ref(project_path, &input.mission_id)
            } else {
                latest.scope_ref.clone()
            };

            let chapter_targets = filter_chapter_write_targets(project_path, &latest.target_refs);
            if chapter_targets.is_empty() {
                paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
                return Err(AppError::invalid_argument(
                    "mission blocked: review targets missing or invalid",
                ));
            }

            let gate_policy = resolve_chapter_gate_policy(
                project_path,
                &input.mission_id,
                &scope_ref,
                false,
                true,
            );

            match run_review_gate_with_p1_policies(
                project_path,
                &input.mission_id,
                scope_ref,
                chapter_targets,
                gate_policy.clone(),
                None,
                Some(&start_config.run_config),
            )
            .await
            {
                Ok((report, meta)) => {
                    if meta.staleness.stale {
                        let _ = emitter.progress_entry(
                            "ReviewGate: contextpack was stale and review inputs were refreshed",
                        );
                    }
                    if meta.rebuilt {
                        if let Some(cp) = meta.contextpack.as_ref() {
                            let _ = emitter.contextpack_built(
                                report.scope_ref.as_str(),
                                token_budget_as_str(&cp.token_budget),
                                cp.generated_at,
                            );
                        }
                    }

                    if let Err(e) = persist_review_report(project_path, &input.mission_id, &report)
                    {
                        paused_config_registry()
                            .insert(input.mission_id.clone(), start_config.clone());
                        return Err(e);
                    }
                    let _ =
                        upsert_risk_ledger_from_review(project_path, &input.mission_id, &report);
                    let _ = emitter.layer1_updated("risk_ledger");
                    let _ = emitter.review_recorded(&report);

                    let strict_warn_block = gate_policy.strict_warn
                        && report.overall_status == review_types::ReviewOverallStatus::Warn;
                    if report.overall_status == review_types::ReviewOverallStatus::Block
                        || strict_warn_block
                    {
                        paused_config_registry()
                            .insert(input.mission_id.clone(), start_config.clone());
                        return Err(AppError::invalid_argument(
                            "mission still blocked by ReviewGate",
                        ));
                    }

                    // Clear any stale fixup tracker/decision.
                    review_fixup_registry().remove(input.mission_id.as_str());
                    let _ =
                        artifacts::clear_pending_review_decision(project_path, &input.mission_id);
                    let _ = artifacts::refresh_workflow_blockers(project_path, &input.mission_id);
                }
                Err(e) => {
                    paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
                    return Err(e);
                }
            }
        }
    }

    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    if !list_worker_handles(&input.mission_id).is_empty()
        || scheduler::has_active_in_process_delegates(&input.mission_id)
    {
        paused_config_registry().insert(input.mission_id.clone(), start_config.clone());
        append_mission_recovery_log(
            project_path,
            &input.mission_id,
            "mission_resume rejected: mission already has active workers or in-process delegates",
        );
        return Err(AppError::invalid_argument(
            "mission already has active workers or in-process delegates",
        ));
    }

    orch.transition(MissionState::Running)?;
    emitter.state_changed(&old_state_str, "running")?;

    // M5/C6: recover macro state on resume (rebuild from features if stale/missing)
    super::macro_commands::try_recover_macro_state_on_resume(
        project_path,
        &input.mission_id,
        Some(&emitter),
    );

    match scheduler::schedule_ready_features(
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

    // Keep run config available for subsequent pause/fixup actions.
    paused_config_registry().insert(input.mission_id.clone(), start_config.clone());

    append_mission_recovery_log(project_path, &input.mission_id, "mission resumed");

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        max_workers = start_config.max_workers,
        delegate_transport = start_config.delegate_transport.as_str(),
        "mission resumed"
    );

    Ok(())
}

#[command]
pub async fn mission_resume_with_config(
    app_handle: tauri::AppHandle,
    input: MissionStartInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle.clone(), input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let start_config = dto::resolve_start_config(&input)?;

    resume_mission_with_config(
        Some(app_handle),
        project_path,
        &input.project_path,
        &input.mission_id,
        &orch,
        Some(&emitter),
        &start_config,
    )
    .await?;

    paused_config_registry().insert(input.mission_id, start_config);
    Ok(())
}

#[command]
pub async fn mission_recover(
    app_handle: tauri::AppHandle,
    input: MissionControlInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let orch = Orchestrator::new(project_path, input.mission_id.clone());
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    recover_mission(&orch, Some(&emitter), project_path, &input.mission_id)
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

    for worker_id in scheduler::cancel_in_process_delegates(&input.mission_id, None, true) {
        clear_worker_from_state(project_path, &input.mission_id, &worker_id);
        append_mission_recovery_log(
            project_path,
            &input.mission_id,
            format!("mission_cancel requested in-process delegate stop for {worker_id}"),
        );
    }

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

    orch.transition(MissionState::Cancelled)?;
    emitter.state_changed(&old_state_str, "cancelled")?;

    // M5: update macro state on cancel
    super::macro_commands::update_macro_stage_on_lifecycle(
        project_path,
        &input.mission_id,
        crate::mission::macro_types::MacroStage::Cancelled,
        Some(&emitter),
    );

    append_mission_recovery_log(project_path, &input.mission_id, "mission cancelled");

    tracing::info!(
        target: "mission",
        mission_id = %input.mission_id,
        "mission cancelled"
    );

    Ok(())
}
