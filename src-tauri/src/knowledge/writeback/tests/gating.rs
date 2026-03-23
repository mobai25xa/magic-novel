use super::*;

use crate::knowledge::types::{
    KnowledgeDeltaStatus, KNOWLEDGE_BRANCH_STALE, KNOWLEDGE_CANON_CONFLICT, KNOWLEDGE_REVISION_CONFLICT,
};
use crate::review::types as review_types;

#[test]
fn gate_auto_accepts_only_on_review_pass() {
    let i1 = mk_item(
        "chapter_summary",
        KnowledgeOp::Create,
        "chapter_summaries/vol1/ch1.json",
        None,
        KnowledgeAcceptPolicy::AutoIfPass,
        serde_json::json!({"summary": "x"}),
    );
    let i2 = mk_item(
        "recent_fact",
        KnowledgeOp::Create,
        "recent_facts/vol1_ch1/f1.json",
        None,
        KnowledgeAcceptPolicy::AutoIfPass,
        serde_json::json!({"summary": "y"}),
    );
    let bundle = mk_bundle(vec![i1, i2]);

    let pass = mk_review(review_types::ReviewOverallStatus::Pass);
    let delta = super::super::gate_bundle(&temp_project_dir(), &bundle, Some(&pass)).unwrap();
    assert_eq!(delta.status, KnowledgeDeltaStatus::Accepted);
    assert_eq!(
        delta.accepted_item_ids.unwrap_or_default().len(),
        bundle.proposal_items.len()
    );

    let warn = mk_review(review_types::ReviewOverallStatus::Warn);
    let delta = super::super::gate_bundle(&temp_project_dir(), &bundle, Some(&warn)).unwrap();
    assert_eq!(delta.status, KnowledgeDeltaStatus::Proposed);
    assert!(delta.accepted_item_ids.is_none());
}

#[test]
fn gate_detects_revision_conflict() {
    let project = temp_project_dir();
    let target_ref = "chapter_summaries/vol1/ch1.json";
    write_stored_object(
        &project,
        target_ref,
        &mk_stored_object("chapter_summary", 2, serde_json::json!({"summary": "old"})),
    );

    let item = mk_item(
        "chapter_summary",
        KnowledgeOp::Update,
        target_ref,
        Some(1),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"summary": "new"}),
    );
    let bundle = mk_bundle(vec![item]);
    let delta = super::super::gate_bundle(&project, &bundle, None).unwrap();
    assert!(delta
        .conflicts
        .iter()
        .any(|c| c.conflict_type == KNOWLEDGE_REVISION_CONFLICT));
}

#[test]
fn gate_detects_foreshadow_status_regression() {
    let project = temp_project_dir();
    let target_ref = "foreshadow/foo.json";
    write_stored_object(
        &project,
        target_ref,
        &mk_stored_object(
            "foreshadow",
            3,
            serde_json::json!({
                "seed_ref": "seed:a",
                "status_label": "paid",
                "current_notes": ""
            }),
        ),
    );

    let item = mk_item(
        "foreshadow",
        KnowledgeOp::Update,
        target_ref,
        Some(3),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({
            "seed_ref": "seed:a",
            "status_label": "active",
            "current_notes": ""
        }),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = super::super::gate_bundle(&project, &bundle, None).unwrap();

    assert!(delta.conflicts.iter().any(|c| {
        c.conflict_type == KNOWLEDGE_CANON_CONFLICT
            && c.item_id.as_deref() == Some(item.item_id.as_str())
    }));
}

#[test]
fn gate_detects_duplicate_recent_fact_summary_against_existing() {
    let project = temp_project_dir();

    let existing_ref = "recent_facts/vol1_ch1/f1.json";
    write_stored_object(
        &project,
        existing_ref,
        &mk_stored_object("recent_fact", 1, serde_json::json!({"summary": "Same fact"})),
    );

    let item = mk_item(
        "recent_fact",
        KnowledgeOp::Create,
        "recent_facts/vol1_ch1/f2.json",
        None,
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"summary": "Same fact"}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = super::super::gate_bundle(&project, &bundle, None).unwrap();

    assert!(delta.conflicts.iter().any(|c| {
        c.conflict_type == KNOWLEDGE_CANON_CONFLICT
            && c.item_id.as_deref() == Some(item.item_id.as_str())
    }));
}

#[test]
fn repropose_refreshes_target_revision_and_clears_revision_conflict() {
    let project = temp_project_dir();

    let target_ref = "terms/foo.json";
    write_stored_object(&project, target_ref, &mk_stored_object("term", 2, serde_json::json!({"a": 1})));

    let item = mk_item(
        "term",
        KnowledgeOp::Update,
        target_ref,
        Some(1),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"a": 2}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = super::super::gate_bundle(&project, &bundle, None).unwrap();
    assert!(delta
        .conflicts
        .iter()
        .any(|c| c.conflict_type == KNOWLEDGE_REVISION_CONFLICT));

    let rebased = super::super::repropose_bundle_refresh_target_revisions(&project, &bundle).unwrap();
    let rebased_item = rebased
        .proposal_items
        .iter()
        .find(|it| it.item_id == item.item_id)
        .unwrap();
    assert_eq!(rebased_item.target_revision, Some(2));

    let delta2 = super::super::gate_bundle(&project, &rebased, None).unwrap();
    assert!(!delta2
        .conflicts
        .iter()
        .any(|c| c.conflict_type == KNOWLEDGE_REVISION_CONFLICT));
}

#[test]
fn gate_adds_branch_stale_conflict_when_bundle_branch_mismatches_active() {
    let item = mk_item(
        "chapter_summary",
        KnowledgeOp::Create,
        "chapter_summaries/vol1/ch1.json",
        None,
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"summary": "x"}),
    );
    let mut bundle = mk_bundle(vec![item]);
    bundle.branch_id = Some("branch/other".to_string());

    let delta = super::super::gate_bundle(&temp_project_dir(), &bundle, None).unwrap();
    assert!(delta
        .conflicts
        .iter()
        .any(|c| c.conflict_type == KNOWLEDGE_BRANCH_STALE && c.item_id.is_none()));
}

