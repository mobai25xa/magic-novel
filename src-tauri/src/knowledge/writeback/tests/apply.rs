use super::*;

use crate::knowledge::types::{KnowledgeDecisionActor, KNOWLEDGE_CANON_CONFLICT};

#[test]
fn apply_and_rollback_create() {
    let project = temp_project_dir();
    let mission_id = "mis_test_apply_create";

    let item = mk_item(
        "term",
        KnowledgeOp::Create,
        "terms/foo.json",
        None,
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"summary": "hello"}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = mk_accepted_delta(
        &bundle,
        "kdelta_apply_create",
        vec![item.item_id.clone()],
    );

    let applied = super::super::apply_accepted(
        &project,
        mission_id,
        &bundle,
        &delta,
        KnowledgeDecisionActor::User,
    )
    .unwrap();
    let token = applied
        .rollback
        .as_ref()
        .and_then(|r| r.token.clone())
        .unwrap();

    let root = crate::services::knowledge_paths::resolve_knowledge_root_for_read(&project);
    assert!(root.join("terms/foo.json").exists());

    let (_restored, _deleted) = super::super::rollback(&project, mission_id, &token).unwrap();
    assert!(!root.join("terms/foo.json").exists());
}

#[test]
fn apply_update_then_rollback_restores_previous_content() {
    let project = temp_project_dir();
    let mission_id = "mis_test_apply_update";

    let target_ref = "terms/foo.json";
    let full = write_stored_object(&project, target_ref, &mk_stored_object("term", 5, serde_json::json!({"a": 1})));

    let item = mk_item(
        "term",
        KnowledgeOp::Update,
        target_ref,
        Some(5),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"a": 2}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = mk_accepted_delta(
        &bundle,
        "kdelta_apply_update",
        vec![item.item_id.clone()],
    );

    let applied = super::super::apply_accepted(
        &project,
        mission_id,
        &bundle,
        &delta,
        KnowledgeDecisionActor::User,
    )
    .unwrap();
    let token = applied
        .rollback
        .as_ref()
        .and_then(|r| r.token.clone())
        .unwrap();

    let raw = std::fs::read_to_string(&full).unwrap();
    let obj: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
    assert_eq!(obj.revision, 6);
    assert_eq!(obj.fields["a"], serde_json::json!(2));
    assert!(obj.archived_at.is_none());

    let root = crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project).unwrap();
    let history = root.join("_history/terms/foo.rev_5.json");
    let raw = std::fs::read_to_string(&history).unwrap();
    let superseded: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
    assert_eq!(superseded.status, "superseded");
    assert_eq!(
        superseded.superseded_by.as_deref(),
        Some("term:terms/foo.json")
    );
    assert!(superseded.superseded_at.is_some());

    super::super::rollback(&project, mission_id, &token).unwrap();
    let raw = std::fs::read_to_string(&full).unwrap();
    let obj: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
    assert_eq!(obj.revision, 5);
    assert_eq!(obj.fields["a"], serde_json::json!(1));
    assert!(!history.exists());
}

#[test]
fn apply_archive_sets_archived_at_and_preserves_superseded_snapshot() {
    let project = temp_project_dir();
    let mission_id = "mis_test_apply_archive";

    let target_ref = "terms/foo.json";
    let full = write_stored_object(&project, target_ref, &mk_stored_object("term", 2, serde_json::json!({"a": 1})));

    let item = mk_item(
        "term",
        KnowledgeOp::Archive,
        target_ref,
        Some(2),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"a": 1}),
    );
    let bundle = mk_bundle(vec![item.clone()]);
    let delta = mk_accepted_delta(
        &bundle,
        "kdelta_apply_archive",
        vec![item.item_id.clone()],
    );

    super::super::apply_accepted(
        &project,
        mission_id,
        &bundle,
        &delta,
        KnowledgeDecisionActor::User,
    )
    .unwrap();

    let raw = std::fs::read_to_string(&full).unwrap();
    let obj: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
    assert_eq!(obj.status, "archived");
    assert!(obj.archived_at.is_some());
    assert!(obj.superseded_at.is_none());

    let root = crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project).unwrap();
    let history = root.join("_history/terms/foo.rev_2.json");
    let raw = std::fs::read_to_string(&history).unwrap();
    let superseded: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
    assert_eq!(superseded.status, "superseded");
    assert_eq!(
        superseded.superseded_by.as_deref(),
        Some("term:terms/foo.json")
    );
    assert!(superseded.superseded_at.is_some());
}

#[test]
fn apply_preflight_prevents_partial_writes() {
    let project = temp_project_dir();
    let mission_id = "mis_test_preflight";

    let i1 = mk_item(
        "term",
        KnowledgeOp::Create,
        "terms/one.json",
        None,
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"a": 1}),
    );
    let i2 = mk_item(
        "term",
        KnowledgeOp::Update,
        "terms/missing.json",
        Some(1),
        KnowledgeAcceptPolicy::Manual,
        serde_json::json!({"a": 2}),
    );
    let bundle = mk_bundle(vec![i1.clone(), i2.clone()]);
    let delta = mk_accepted_delta(
        &bundle,
        "kdelta_preflight",
        vec![i1.item_id.clone(), i2.item_id.clone()],
    );

    let err = super::super::apply_accepted(
        &project,
        mission_id,
        &bundle,
        &delta,
        KnowledgeDecisionActor::User,
    )
    .unwrap_err();
    assert!(err.message.contains(KNOWLEDGE_CANON_CONFLICT));

    let root = crate::services::knowledge_paths::resolve_knowledge_root_for_read(&project);
    assert!(!root.join("terms/one.json").exists());
}

