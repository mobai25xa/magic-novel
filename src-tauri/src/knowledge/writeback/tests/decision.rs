use super::*;

use crate::knowledge::types::{
    KnowledgeConflict, KnowledgeDecisionActor, KnowledgeDecisionInput, KnowledgeDelta, KnowledgeDeltaStatus,
    KNOWLEDGE_POLICY_CONFLICT,
};

fn mk_proposed_delta(bundle: &KnowledgeProposalBundle, delta_id: &str) -> KnowledgeDelta {
    KnowledgeDelta {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        knowledge_delta_id: delta_id.to_string(),
        status: KnowledgeDeltaStatus::Proposed,
        scope_ref: bundle.scope_ref.clone(),
        branch_id: bundle.branch_id.clone(),
        source_session_id: bundle.source_session_id.clone(),
        source_review_id: bundle.source_review_id.clone(),
        generated_at: bundle.generated_at,
        targets: Vec::new(),
        changes: Vec::new(),
        evidence_refs: Vec::new(),
        conflicts: Vec::new(),
        accepted_item_ids: None,
        rejected_item_ids: None,
        applied_at: None,
        rollback: None,
    }
}

#[test]
fn decide_rejects_accepting_conflicted_item() {
    let item = mk_item(
        "chapter_summary",
        KnowledgeOp::Create,
        "chapter_summaries/vol1/ch1.json",
        None,
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"summary": "x"}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let mut delta = mk_proposed_delta(&bundle, "kdelta_decide");
    delta.conflicts = vec![KnowledgeConflict {
        conflict_type: "KNOWLEDGE_CANON_CONFLICT".to_string(),
        message: "x".to_string(),
        item_id: Some(item.item_id.clone()),
        target_ref: Some("chapter_summaries/vol1/ch1.json".to_string()),
    }];

    let decision = KnowledgeDecisionInput {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        actor: KnowledgeDecisionActor::User,
        accepted_item_ids: vec![item.item_id.clone()],
        rejected_item_ids: Vec::new(),
    };

    let res = super::super::apply_decision_to_delta(&bundle, delta.clone(), &decision);
    assert!(res.is_err());

    // Rejecting should be allowed and clears item conflict.
    decision_reject(&bundle, &mut delta, &item.item_id);
}

fn decision_reject(bundle: &KnowledgeProposalBundle, delta: &mut KnowledgeDelta, item_id: &str) {
    let decision = KnowledgeDecisionInput {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        actor: KnowledgeDecisionActor::User,
        accepted_item_ids: Vec::new(),
        rejected_item_ids: vec![item_id.to_string()],
    };
    let updated = super::super::apply_decision_to_delta(bundle, delta.clone(), &decision).unwrap();
    assert!(updated
        .conflicts
        .iter()
        .all(|c| c.item_id.as_deref() != Some(item_id)));
}

#[test]
fn decide_disallows_user_accepting_orchestrator_only_item() {
    let item = mk_item(
        "term",
        KnowledgeOp::Create,
        "terms/foo.json",
        None,
        KnowledgeAcceptPolicy::OrchestratorOnly,
        serde_json::json!({"summary": "x"}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = mk_proposed_delta(&bundle, "kdelta_orch_only");

    let decision = KnowledgeDecisionInput {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        actor: KnowledgeDecisionActor::User,
        accepted_item_ids: vec![item.item_id.clone()],
        rejected_item_ids: Vec::new(),
    };

    let err = super::super::apply_decision_to_delta(&bundle, delta, &decision).unwrap_err();
    assert!(err.message.contains(KNOWLEDGE_POLICY_CONFLICT));
}

#[test]
fn decide_allows_orchestrator_accepting_orchestrator_only_item() {
    let item = mk_item(
        "term",
        KnowledgeOp::Create,
        "terms/foo.json",
        None,
        KnowledgeAcceptPolicy::OrchestratorOnly,
        serde_json::json!({"summary": "x"}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = mk_proposed_delta(&bundle, "kdelta_orch_only_ok");

    let decision = KnowledgeDecisionInput {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        actor: KnowledgeDecisionActor::Orchestrator,
        accepted_item_ids: vec![item.item_id.clone()],
        rejected_item_ids: Vec::new(),
    };

    let updated = super::super::apply_decision_to_delta(&bundle, delta, &decision).unwrap();
    assert_eq!(updated.accepted_item_ids.unwrap_or_default().len(), 1);
}

