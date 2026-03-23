use crate::knowledge::{types as knowledge_types, writeback as knowledge_writeback};
use crate::mission::artifacts;
use crate::mission::contextpack_types::TokenBudget as ContextPackTokenBudget;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::models::AppError;
use crate::review::types as review_types;

use crate::commands::mission::{macro_commands, MissionStartConfig};

use super::super::super::review_gate::*;

use std::fs;

fn read_macro_chapter_completed_indices(
    project_path: &std::path::Path,
    mission_id: &str,
) -> std::collections::HashSet<usize> {
    use std::collections::HashSet;
    let path = artifacts::macro_checkpoints_path(project_path, mission_id);
    if !path.exists() {
        return HashSet::new();
    }
    let Ok(content) = fs::read_to_string(&path) else {
        return HashSet::new();
    };

    let mut completed = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(line.as_bytes()) else {
            continue;
        };
        if v.get("event").and_then(|e| e.as_str()) != Some("chapter_completed") {
            continue;
        }
        let Some(idx) = v.get("chapter_idx").and_then(|n| n.as_u64()) else {
            continue;
        };
        completed.insert(idx as usize);
    }
    completed
}

fn macro_scope_ref_for_chapter(
    cfg: &crate::mission::macro_types::MacroWorkflowConfig,
    chapter_idx: usize,
) -> Option<String> {
    cfg.chapter_targets
        .get(chapter_idx)
        .map(|t| format!("macro:{}:{}", cfg.macro_id, t.chapter_ref))
}

pub(super) async fn try_closeout_macro_chapter_on_resume(
    project_path: &std::path::Path,
    mission_id: &str,
    orch: &Orchestrator<'_>,
    emitter: Option<&MissionEventEmitter>,
    start_config: &MissionStartConfig,
    macro_cfg: &crate::mission::macro_types::MacroWorkflowConfig,
) -> Result<(), AppError> {
    let completed_by_checkpoint = read_macro_chapter_completed_indices(project_path, mission_id);
    let features_doc = orch.get_features()?;

    let mut candidate: Option<(usize, Feature)> = None;
    for (i, _t) in macro_cfg.chapter_targets.iter().enumerate() {
        let draft_id = format!("ch{}_draft", i + 1);
        if let Some(f) = features_doc.features.iter().find(|f| f.id == draft_id) {
            if f.status == FeatureStatus::Completed && !completed_by_checkpoint.contains(&i) {
                candidate = Some((i, f.clone()));
            }
        }
    }

    let Some((chapter_idx, feature)) = candidate else {
        return Ok(());
    };

    let Some(expected_scope_ref) = macro_scope_ref_for_chapter(macro_cfg, chapter_idx) else {
        return Ok(());
    };

    // Fast-path: if writeback already applied for this chapter (but checkpoint/state wasn't written),
    // mark chapter completed without re-running gates.
    if let Ok(Some(delta)) = artifacts::read_knowledge_delta_latest(project_path, mission_id) {
        if delta.applied_at.is_some() && delta.scope_ref == expected_scope_ref {
            let status_str = serde_json::to_string(&delta.status)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            let _ = macro_commands::macro_on_writeback_completed(
                project_path,
                mission_id,
                chapter_idx,
                feature.id.as_str(),
                Some(delta.knowledge_delta_id.as_str()),
                status_str.as_str(),
                emitter,
            );
            let _ = macro_commands::macro_mark_chapter_completed(
                project_path,
                mission_id,
                chapter_idx,
                feature.id.as_str(),
                emitter,
            );
            return Ok(());
        }
    }

    // Closeout gates: ReviewGate → KnowledgeWriteback → Integrate.
    let chapter_targets = filter_chapter_write_targets(project_path, &feature.write_paths);
    if chapter_targets.is_empty() {
        let reason = "Macro blocked: chapter write target missing/invalid (must be manuscripts-relative .json that parses as Chapter)";
        let _ = macro_commands::macro_block_invalid_chapter_write_targets(
            project_path,
            mission_id,
            chapter_idx,
            feature.id.as_str(),
            reason,
            emitter,
        );
        return Err(AppError::invalid_argument(reason));
    }

    let macro_token_budget: Option<ContextPackTokenBudget> = Some(match macro_cfg.token_budget {
        crate::mission::macro_types::TokenBudget::Small => ContextPackTokenBudget::Small,
        crate::mission::macro_types::TokenBudget::Medium => ContextPackTokenBudget::Medium,
        crate::mission::macro_types::TokenBudget::Large => ContextPackTokenBudget::Large,
    });

    let gate_policy = resolve_chapter_gate_policy(
        project_path,
        mission_id,
        &expected_scope_ref,
        macro_cfg.strict_review,
        macro_cfg.auto_fix_on_block,
    );
    let _ = macro_commands::macro_on_review_started(
        project_path,
        mission_id,
        chapter_idx,
        feature.id.as_str(),
        emitter,
    );

    let (report, meta) = run_review_gate_with_p1_policies(
        project_path,
        mission_id,
        expected_scope_ref.clone(),
        chapter_targets,
        gate_policy.clone(),
        macro_token_budget,
        Some(&start_config.run_config),
    )
    .await?;

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

    let strict_warn_block =
        gate_policy.strict_warn && report.overall_status == review_types::ReviewOverallStatus::Warn;
    let review_blocked =
        report.overall_status == review_types::ReviewOverallStatus::Block || strict_warn_block;

    let overall_status_str = serde_json::to_string(&report.overall_status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let _ = macro_commands::macro_on_review_completed(
        project_path,
        mission_id,
        chapter_idx,
        feature.id.as_str(),
        report.review_id.as_str(),
        overall_status_str.as_str(),
        review_blocked,
        emitter,
    );

    if review_blocked {
        let strict_report = if strict_warn_block {
            let mut strict = report.clone();
            for issue in &mut strict.issues {
                if issue.severity == review_types::ReviewSeverity::Warn {
                    issue.severity = review_types::ReviewSeverity::Block;
                }
            }
            strict.overall_status = review_types::ReviewOverallStatus::Block;
            strict
        } else {
            report.clone()
        };

        let req = build_review_decision_request(&strict_report, Some(feature.id.clone()));
        let _ = artifacts::write_pending_review_decision(project_path, mission_id, &req);
        if let Some(emitter) = emitter {
            let _ = emitter.review_decision_required(&req);
        }
        return Err(AppError::invalid_argument(
            "mission blocked: pending review decision required",
        ));
    }

    // Clear any stale fixup tracker/decision.
    review_fixup_registry().remove(mission_id);
    let _ = artifacts::clear_pending_review_decision(project_path, mission_id);

    let _ = macro_commands::macro_on_writeback_started(
        project_path,
        mission_id,
        chapter_idx,
        feature.id.as_str(),
        Some(report.review_id.as_str()),
        emitter,
    );

    let source_session_id = format!(
        "mission:{}/feature:{}/resume_closeout",
        mission_id,
        feature.id.trim()
    );

    let bundle = knowledge_writeback::generate_proposal_bundle_after_closeout(
        project_path,
        mission_id,
        report.scope_ref.clone(),
        feature.write_paths.clone(),
        source_session_id,
        Some(report.review_id.clone()),
    )?;

    let review_for_knowledge = if !gate_policy.strict_warn
        && report.overall_status == review_types::ReviewOverallStatus::Warn
    {
        let mut relaxed = report.clone();
        relaxed.overall_status = review_types::ReviewOverallStatus::Pass;
        relaxed
    } else {
        report.clone()
    };

    let delta =
        knowledge_writeback::gate_bundle(project_path, &bundle, Some(&review_for_knowledge))?;

    artifacts::write_knowledge_bundle_latest(project_path, mission_id, &bundle)?;
    let _ = artifacts::append_knowledge_bundle(project_path, mission_id, &bundle);
    artifacts::write_knowledge_delta_latest(project_path, mission_id, &delta)?;
    let _ = artifacts::append_knowledge_delta(project_path, mission_id, &delta);
    if let Some(emitter) = emitter {
        let _ = emitter.knowledge_proposed(&bundle);
    }

    if !delta.conflicts.is_empty() {
        let pending = knowledge_writeback::build_pending_decision(&bundle, &delta);
        let _ = artifacts::write_pending_knowledge_decision(project_path, mission_id, &pending);
        if let Some(emitter) = emitter {
            let _ = emitter.knowledge_decision_required(&delta);
        }
        let _ = macro_commands::macro_on_writeback_blocked(
            project_path,
            mission_id,
            chapter_idx,
            feature.id.as_str(),
            Some(delta.knowledge_delta_id.as_str()),
            "KnowledgeWriteback blocked: conflicts detected; user decision required",
            emitter,
        );
        return Err(AppError::invalid_argument(
            "mission blocked: pending knowledge decision required",
        ));
    }

    let _ = artifacts::clear_pending_knowledge_decision(project_path, mission_id);

    // Default writeback: auto-apply when fully accepted.
    if delta.status == knowledge_types::KnowledgeDeltaStatus::Accepted {
        let applied = knowledge_writeback::apply_accepted(
            project_path,
            mission_id,
            &bundle,
            &delta,
            knowledge_types::KnowledgeDecisionActor::Orchestrator,
        )?;

        artifacts::write_knowledge_delta_latest(project_path, mission_id, &applied)?;
        let _ = artifacts::append_knowledge_delta(project_path, mission_id, &applied);
        if let Some(emitter) = emitter {
            let _ = emitter.knowledge_applied(&applied);
        }

        let status_str = serde_json::to_string(&applied.status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let _ = macro_commands::macro_on_writeback_completed(
            project_path,
            mission_id,
            chapter_idx,
            feature.id.as_str(),
            Some(applied.knowledge_delta_id.as_str()),
            status_str.as_str(),
            emitter,
        );

        // Integrate (v0): mark Layer1 chapter completed + minimal previous_summary.
        let now = chrono::Utc::now().timestamp_millis();
        if let Ok(Some(mut cc)) = artifacts::read_layer1_chapter_card(project_path, mission_id) {
            cc.status = crate::mission::layer1_types::ChapterCardStatus::Completed;
            cc.updated_at = now;
            let _ = artifacts::write_layer1_chapter_card(project_path, mission_id, &cc);
            if let Some(emitter) = emitter {
                let _ = emitter.layer1_updated("chapter_card");
            }
        }

        let prev_path = artifacts::layer1_previous_summary_path(project_path, mission_id);
        if let Some(parent) = prev_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let write_path = macro_cfg
            .chapter_targets
            .get(chapter_idx)
            .map(|t| t.write_path.clone());
        let payload = serde_json::json!({
                "schema_version": crate::mission::layer1_types::LAYER1_SCHEMA_VERSION,
                "kind": "macro_chapter_closeout",
                "chapter_idx": chapter_idx,
                "write_path": write_path,
                "review_id": report.review_id.clone(),
                "knowledge_delta_id": applied.knowledge_delta_id.clone(),
                "updated_at": now,
        });
        if let Ok(content) = serde_json::to_string_pretty(&payload) {
            let _ = crate::utils::atomic_write::atomic_write(&prev_path, &content);
        }
        if let Some(emitter) = emitter {
            let _ = emitter.layer1_updated("previous_summary");
        }

        let _ = macro_commands::macro_mark_chapter_completed(
            project_path,
            mission_id,
            chapter_idx,
            feature.id.as_str(),
            emitter,
        );
    }

    Ok(())
}
