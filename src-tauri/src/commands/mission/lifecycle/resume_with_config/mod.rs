use crate::mission::artifacts;
use crate::mission::blockers::{WorkflowBlockerKind, WorkflowBlockersDoc};
use crate::mission::contextpack_types::TokenBudget as ContextPackTokenBudget;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::models::AppError;
use crate::review::types as review_types;

use super::super::review_gate::*;
use super::super::runtime::{append_mission_recovery_log, list_worker_handles};
use super::super::{scheduler, MissionStartConfig};

mod closeout;

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

pub(in crate::commands::mission) async fn resume_mission_with_config(
    app_handle: Option<tauri::AppHandle>,
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
    orch: &Orchestrator<'_>,
    emitter: Option<&MissionEventEmitter>,
    start_config: &MissionStartConfig,
) -> Result<(), AppError> {
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

    let blockers = artifacts::refresh_workflow_blockers(project_path, mission_id)?;
    if let Some(err) = resume_blocker_error(&blockers) {
        return Err(err);
    }

    // Gate: if latest review is still blocking, re-run review before resuming scheduling.
    if let Ok(Some(latest)) = artifacts::read_review_latest(project_path, mission_id) {
        if latest.overall_status == review_types::ReviewOverallStatus::Block {
            let scope_ref = if latest.scope_ref.trim().is_empty() {
                infer_review_scope_ref(project_path, mission_id)
            } else {
                latest.scope_ref.clone()
            };

            let chapter_targets = filter_chapter_write_targets(project_path, &latest.target_refs);
            if chapter_targets.is_empty() {
                return Err(AppError::invalid_argument(
                    "mission blocked: review targets missing or invalid",
                ));
            }

            let macro_cfg_opt = artifacts::read_macro_config(project_path, mission_id)
                .ok()
                .flatten();
            let macro_token_budget: Option<ContextPackTokenBudget> =
                macro_cfg_opt.as_ref().map(|cfg| match cfg.token_budget {
                    crate::mission::macro_types::TokenBudget::Small => {
                        ContextPackTokenBudget::Small
                    }
                    crate::mission::macro_types::TokenBudget::Medium => {
                        ContextPackTokenBudget::Medium
                    }
                    crate::mission::macro_types::TokenBudget::Large => {
                        ContextPackTokenBudget::Large
                    }
                });
            let gate_policy = resolve_chapter_gate_policy(
                project_path,
                mission_id,
                &scope_ref,
                macro_cfg_opt
                    .as_ref()
                    .map(|cfg| cfg.strict_review)
                    .unwrap_or(false),
                macro_cfg_opt
                    .as_ref()
                    .map(|cfg| cfg.auto_fix_on_block)
                    .unwrap_or(true),
            );

            match run_review_gate_with_p1_policies(
                project_path,
                mission_id,
                scope_ref,
                chapter_targets,
                gate_policy.clone(),
                macro_token_budget,
                Some(&start_config.run_config),
            )
            .await
            {
                Ok((report, meta)) => {
                    if meta.staleness.stale {
                        if let Some(emitter) = emitter {
                            let _ = emitter.progress_entry(
                                "ReviewGate: contextpack was stale and review inputs were refreshed",
                            );
                        }
                    }
                    if meta.rebuilt {
                        if let Some(cp) = meta.contextpack.as_ref() {
                            if let Some(emitter) = emitter {
                                let _ = emitter.contextpack_built(
                                    report.scope_ref.as_str(),
                                    token_budget_as_str(&cp.token_budget),
                                    cp.generated_at,
                                );
                            }
                        }
                    }

                    persist_review_report(project_path, mission_id, &report)?;
                    let _ = upsert_risk_ledger_from_review(project_path, mission_id, &report);
                    if let Some(emitter) = emitter {
                        let _ = emitter.layer1_updated("risk_ledger");
                        let _ = emitter.review_recorded(&report);
                    }

                    let strict_warn_block = gate_policy.strict_warn
                        && report.overall_status == review_types::ReviewOverallStatus::Warn;
                    if report.overall_status == review_types::ReviewOverallStatus::Block
                        || strict_warn_block
                    {
                        return Err(AppError::invalid_argument(
                            "mission still blocked by ReviewGate",
                        ));
                    }

                    // Clear any stale fixup tracker/decision.
                    review_fixup_registry().remove(mission_id);
                    let _ = artifacts::clear_pending_review_decision(project_path, mission_id);
                    let _ = artifacts::refresh_workflow_blockers(project_path, mission_id);
                }
                Err(e) => return Err(e),
            }
        }
    }

    // M5/P0: If a macro chapter draft completed but gates didn't run (crash window),
    // close out ReviewGate + KnowledgeWriteback before scheduling the next chapter.
    if let Ok(Some(macro_cfg)) = artifacts::read_macro_config(project_path, mission_id) {
        closeout::try_closeout_macro_chapter_on_resume(
            project_path,
            mission_id,
            orch,
            emitter,
            start_config,
            &macro_cfg,
        )
        .await?;
    }

    let old_state_str = serde_json::to_string(&current_state.state)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    if !list_worker_handles(mission_id).is_empty() {
        append_mission_recovery_log(
            project_path,
            mission_id,
            "mission_resume rejected: mission already has active workers",
        );
        return Err(AppError::invalid_argument(
            "mission already has active workers",
        ));
    }

    let Some(emitter) = emitter else {
        return Err(AppError::invalid_argument(
            "mission resume requires event emitter",
        ));
    };

    orch.transition(MissionState::Running)?;
    emitter.state_changed(&old_state_str, "running")?;

    // M5/C6: recover macro state on resume (rebuild from features if stale/missing)
    super::super::macro_commands::try_recover_macro_state_on_resume(
        project_path,
        mission_id,
        Some(emitter),
    );

    let Some(app_handle) = app_handle else {
        orch.transition(MissionState::Paused)?;
        return Err(AppError::invalid_argument(
            "mission resume requires app handle",
        ));
    };

    match scheduler::schedule_ready_features(
        orch,
        emitter,
        mission_id,
        project_path,
        project_path_str,
        start_config,
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
            append_mission_recovery_log(
                project_path,
                mission_id,
                format!("mission_resume scheduling failed: {e}"),
            );
            return Err(e);
        }
    }

    append_mission_recovery_log(project_path, mission_id, "mission resumed");

    tracing::info!(
        target: "mission",
        mission_id = %mission_id,
        max_workers = start_config.max_workers,
        "mission resumed"
    );

    Ok(())
}
