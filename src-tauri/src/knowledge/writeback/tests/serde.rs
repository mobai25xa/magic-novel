use super::*;

use crate::knowledge::types::{KnowledgeDelta, KnowledgeDeltaStatus};

#[test]
fn serde_roundtrip_bundle_and_delta() {
    let item = mk_item(
        "chapter_summary",
        KnowledgeOp::Create,
        "chapter_summaries/vol1/ch1.json",
        None,
        KnowledgeAcceptPolicy::AutoIfPass,
        serde_json::json!({"summary": "x"}),
    );
    let bundle = mk_bundle(vec![item]);
    let raw = serde_json::to_string(&bundle).unwrap();
    let parsed: KnowledgeProposalBundle = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed.bundle_id, bundle.bundle_id);
    assert_eq!(parsed.proposal_items.len(), 1);

    let delta = KnowledgeDelta {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        knowledge_delta_id: "kdelta_test".to_string(),
        status: KnowledgeDeltaStatus::Proposed,
        scope_ref: bundle.scope_ref.clone(),
        branch_id: None,
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
    };
    let raw = serde_json::to_string(&delta).unwrap();
    let parsed: KnowledgeDelta = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed.knowledge_delta_id, "kdelta_test");
    assert_eq!(parsed.scope_ref, bundle.scope_ref);
}

