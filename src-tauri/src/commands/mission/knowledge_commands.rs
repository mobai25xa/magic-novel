//! Mission Knowledge Writeback Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::knowledge::{types as knowledge_types, writeback as knowledge_writeback};
use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::models::AppError;
use crate::services::agent_session::{
    self as agent_session, CanonUpdatesAcceptedEntry, CanonUpdatesProposedEntry,
};
use crate::review::types as review_types;

use super::runtime::acquire_mission_runtime_lock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeGetLatestInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_knowledge_get_latest(
    input: MissionKnowledgeGetLatestInput,
) -> Result<knowledge_types::MissionKnowledgeLatest, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?;
    Ok(knowledge_types::MissionKnowledgeLatest { bundle, delta })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeListInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeListOutput {
    pub bundles: Vec<knowledge_types::KnowledgeProposalBundle>,
    pub deltas: Vec<knowledge_types::KnowledgeDelta>,
}

#[command]
pub async fn mission_knowledge_list(
    input: MissionKnowledgeListInput,
) -> Result<MissionKnowledgeListOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let mut bundles = artifacts::read_knowledge_bundles(project_path, &input.mission_id)?;
    let mut deltas = artifacts::read_knowledge_deltas(project_path, &input.mission_id)?;

    let limit = input.limit.unwrap_or(0);
    if limit > 0 {
        if bundles.len() > limit {
            bundles = bundles.into_iter().rev().take(limit).collect::<Vec<_>>();
            bundles.reverse();
        }
        if deltas.len() > limit {
            deltas = deltas.into_iter().rev().take(limit).collect::<Vec<_>>();
            deltas.reverse();
        }
    }

    Ok(MissionKnowledgeListOutput { bundles, deltas })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeReproposeInput {
    pub project_path: String,
    pub mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeReproposeOutput {
    pub bundle: knowledge_types::KnowledgeProposalBundle,
    pub delta: knowledge_types::KnowledgeDelta,
}

#[command]
pub async fn mission_knowledge_repropose(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeReproposeInput,
) -> Result<MissionKnowledgeReproposeOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge proposal bundle found"))?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge delta found"))?;

    knowledge_writeback::validate_bundle_branch_active(project_path, &bundle)?;

    let has_revision_conflict = delta
        .conflicts
        .iter()
        .any(|c| c.conflict_type == knowledge_types::KNOWLEDGE_REVISION_CONFLICT);
    if !has_revision_conflict {
        return Err(AppError::invalid_argument(
            "no KNOWLEDGE_REVISION_CONFLICT present; repropose is not needed",
        ));
    }

    let review_latest = artifacts::read_review_latest(project_path, &input.mission_id)?;
    let review_for_gate: Option<review_types::ReviewReport> = match (
        bundle.source_review_id.as_deref(),
        review_latest.as_ref(),
    ) {
        (Some(src), Some(r)) if r.review_id == src => Some(r.clone()),
        _ => None,
    };

    let rebased = knowledge_writeback::repropose_bundle_refresh_target_revisions(project_path, &bundle)?;
    let regated = knowledge_writeback::gate_bundle(
        project_path,
        &rebased,
        review_for_gate.as_ref(),
    )?;

    artifacts::write_knowledge_bundle_latest(project_path, &input.mission_id, &rebased)?;
    let _ = artifacts::append_knowledge_bundle(project_path, &input.mission_id, &rebased);
    artifacts::write_knowledge_delta_latest(project_path, &input.mission_id, &regated)?;
    let _ = artifacts::append_knowledge_delta(project_path, &input.mission_id, &regated);

    let _ = agent_session::record_canon_updates_proposed(
        project_path,
        &rebased.source_session_id,
        rebased.branch_id.as_ref(),
        CanonUpdatesProposedEntry {
            bundle_id: rebased.bundle_id.clone(),
            delta_id: regated.knowledge_delta_id.clone(),
            scope_ref: rebased.scope_ref.clone(),
            kinds: knowledge_writeback::proposal_kinds(&rebased),
            ts: rebased.generated_at,
        },
    );

    if !regated.conflicts.is_empty() {
        let pending = knowledge_writeback::build_pending_decision(&rebased, &regated);
        let _ = artifacts::write_pending_knowledge_decision(project_path, &input.mission_id, &pending);
    } else {
        let _ = artifacts::clear_pending_knowledge_decision(project_path, &input.mission_id);
    }

    // Best-effort event emission.
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    let _ = emitter.knowledge_proposed(&rebased);
    if !regated.conflicts.is_empty() {
        let _ = emitter.knowledge_decision_required(&regated);
    }

    Ok(MissionKnowledgeReproposeOutput {
        bundle: rebased,
        delta: regated,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeDecideInput {
    pub project_path: String,
    pub mission_id: String,
    pub decision: knowledge_types::KnowledgeDecisionInput,
}

#[command]
pub async fn mission_knowledge_decide(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeDecideInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge proposal bundle found"))?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge delta found"))?;

    knowledge_writeback::validate_bundle_branch_active(project_path, &bundle)?;

    let updated = knowledge_writeback::apply_decision_to_delta(&bundle, delta, &input.decision)?;

    artifacts::write_knowledge_delta_latest(project_path, &input.mission_id, &updated)?;
    let _ = artifacts::append_knowledge_delta(project_path, &input.mission_id, &updated);
    if updated.conflicts.is_empty() {
        let _ = artifacts::clear_pending_knowledge_decision(project_path, &input.mission_id);
    }

    // Best-effort event emission.
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    if !updated.conflicts.is_empty() {
        let _ = emitter.knowledge_decision_required(&updated);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeApplyInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_knowledge_apply(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeApplyInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;

    let bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge proposal bundle found"))?;
    let delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?
        .ok_or_else(|| AppError::not_found("no knowledge delta found"))?;

    let applied = knowledge_writeback::apply_accepted(
        project_path,
        &input.mission_id,
        &bundle,
        &delta,
        knowledge_types::KnowledgeDecisionActor::User,
    )?;

    artifacts::write_knowledge_delta_latest(project_path, &input.mission_id, &applied)?;
    let _ = artifacts::append_knowledge_delta(project_path, &input.mission_id, &applied);
    let _ = artifacts::clear_pending_knowledge_decision(project_path, &input.mission_id);

    let targets = knowledge_writeback::accepted_target_refs(&bundle, &applied);
    let _ = agent_session::record_canon_updates_accepted(
        project_path,
        &bundle.source_session_id,
        bundle.branch_id.as_ref(),
        CanonUpdatesAcceptedEntry {
            delta_id: applied.knowledge_delta_id.clone(),
            applied_at: applied.applied_at.unwrap_or_default(),
            targets: targets.clone(),
            rollback_token: applied.rollback.as_ref().and_then(|rb| rb.token.clone()),
            rolled_back_at: None,
        },
        &targets,
    );

    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    let _ = emitter.knowledge_applied(&applied);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeRollbackInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub token: Option<String>,
}

#[command]
pub async fn mission_knowledge_rollback(
    app_handle: tauri::AppHandle,
    input: MissionKnowledgeRollbackInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let _mission_lock = acquire_mission_runtime_lock(&input.mission_id).await;
    let latest_delta = artifacts::read_knowledge_delta_latest(project_path, &input.mission_id)?;
    let latest_bundle = artifacts::read_knowledge_bundle_latest(project_path, &input.mission_id)?;

    let token = input
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            latest_delta
                .as_ref()
                .and_then(|d| d.rollback.as_ref())
                .and_then(|rb| rb.token.clone())
        })
        .ok_or_else(|| AppError::invalid_argument("rollback token is required"))?;

    let (restored, deleted) =
        knowledge_writeback::rollback(project_path, &input.mission_id, &token)?;

    if let (Some(bundle), Some(delta)) = (latest_bundle.as_ref(), latest_delta.as_ref()) {
        let _ = agent_session::record_canon_updates_rolled_back(
            project_path,
            &bundle.source_session_id,
            bundle.branch_id.as_ref(),
            &delta.knowledge_delta_id,
            Some(token.as_str()),
            chrono::Utc::now().timestamp_millis(),
        );
    }

    let emitter = MissionEventEmitter::new(app_handle, input.mission_id);
    let _ = emitter.knowledge_rolled_back(&token, restored, deleted);

    Ok(())
}
