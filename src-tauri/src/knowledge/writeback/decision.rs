use std::collections::HashSet;

use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeDecisionActor, KnowledgeDecisionInput, KnowledgeDelta,
    KnowledgeDeltaStatus, KnowledgeProposalBundle, PendingKnowledgeDecision, KNOWLEDGE_POLICY_CONFLICT,
};
use crate::models::AppError;

pub fn build_pending_decision(
    bundle: &KnowledgeProposalBundle,
    delta: &KnowledgeDelta,
) -> PendingKnowledgeDecision {
    PendingKnowledgeDecision {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        scope_ref: delta.scope_ref.clone(),
        conflicts: delta.conflicts.clone(),
        created_at: chrono::Utc::now().timestamp_millis(),
    }
}

pub(super) fn apply_decision_to_delta(
    bundle: &KnowledgeProposalBundle,
    mut delta: KnowledgeDelta,
    decision: &KnowledgeDecisionInput,
) -> Result<KnowledgeDelta, AppError> {
    if decision.bundle_id != bundle.bundle_id {
        return Err(AppError::invalid_argument("bundle_id mismatch"));
    }
    if decision.delta_id != delta.knowledge_delta_id {
        return Err(AppError::invalid_argument("delta_id mismatch"));
    }

    let mut accepted: Vec<String> = decision
        .accepted_item_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut rejected: Vec<String> = decision
        .rejected_item_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    accepted.sort();
    accepted.dedup();
    rejected.sort();
    rejected.dedup();

    let accepted_set: HashSet<String> = accepted.iter().cloned().collect();
    let rejected_set: HashSet<String> = rejected.iter().cloned().collect();
    if accepted_set.intersection(&rejected_set).next().is_some() {
        return Err(AppError::invalid_argument(
            "accepted_item_ids and rejected_item_ids overlap",
        ));
    }

    let bundle_ids: HashSet<String> = bundle
        .proposal_items
        .iter()
        .map(|i| i.item_id.clone())
        .collect();
    for id in accepted_set.iter().chain(rejected_set.iter()) {
        if !bundle_ids.contains(id) {
            return Err(AppError::invalid_argument("decision references unknown item_id"));
        }
    }

    // Enforce accept_policy=orchestrator_only.
    if decision.actor != KnowledgeDecisionActor::Orchestrator {
        for item_id in &accepted {
            let policy = bundle
                .proposal_items
                .iter()
                .find(|it| it.item_id == *item_id)
                .map(|it| it.accept_policy.clone())
                .unwrap_or(KnowledgeAcceptPolicy::Manual);

            if policy == KnowledgeAcceptPolicy::OrchestratorOnly {
                return Err(AppError::invalid_argument(format!(
                    "{KNOWLEDGE_POLICY_CONFLICT}: accept_policy=orchestrator_only cannot be accepted by user (item_id={item_id})"
                )));
            }
        }
    }

    // Do not allow accepting items that still have conflicts.
    for c in &delta.conflicts {
        if let Some(item_id) = c.item_id.as_ref() {
            if accepted_set.contains(item_id) {
                return Err(AppError::invalid_argument(format!(
                    "cannot accept conflicted item: {}",
                    item_id
                )));
            }
        }
    }

    // Remove conflicts for rejected items (treat as resolved-by-reject).
    delta.conflicts.retain(|c| {
        c.item_id
            .as_ref()
            .map(|id| !rejected_set.contains(id))
            .unwrap_or(true)
    });

    if !accepted.is_empty() {
        delta.accepted_item_ids = Some(accepted.clone());
    } else {
        delta.accepted_item_ids = None;
    }
    if !rejected.is_empty() {
        delta.rejected_item_ids = Some(rejected.clone());
    } else {
        delta.rejected_item_ids = None;
    }

    if delta.conflicts.is_empty() {
        let decided = accepted.len() + rejected.len();
        if decided == bundle.proposal_items.len() {
            delta.status = if !accepted.is_empty() {
                KnowledgeDeltaStatus::Accepted
            } else {
                KnowledgeDeltaStatus::Rejected
            };
        }
    }

    Ok(delta)
}

