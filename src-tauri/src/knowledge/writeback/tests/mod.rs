use std::path::{Path, PathBuf};

use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeDelta, KnowledgeDeltaStatus, KnowledgeOp, KnowledgeProposalBundle,
    KnowledgeProposalItem,
};
use crate::review::types as review_types;

use super::storage::{StoredKnowledgeObject, STORED_OBJECT_SCHEMA_VERSION};

mod apply;
mod decision;
mod gating;
mod serde;

fn temp_project_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn mk_item(
    kind: &str,
    op: KnowledgeOp,
    target_ref: &str,
    target_revision: Option<i64>,
    accept_policy: KnowledgeAcceptPolicy,
    fields: serde_json::Value,
) -> KnowledgeProposalItem {
    KnowledgeProposalItem {
        item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
        kind: kind.to_string(),
        op,
        target_ref: Some(target_ref.to_string()),
        target_revision,
        fields,
        evidence_refs: vec!["evidence:a".to_string()],
        source_refs: vec!["source:chapter".to_string()],
        change_reason: "test".to_string(),
        accept_policy,
    }
}

fn mk_bundle(items: Vec<KnowledgeProposalItem>) -> KnowledgeProposalBundle {
    KnowledgeProposalBundle {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: format!("kbundle_{}", uuid::Uuid::new_v4()),
        scope_ref: "chapter:vol1/ch1.json".to_string(),
        branch_id: Some("branch/main".to_string()),
        source_session_id: "sess_test".to_string(),
        source_review_id: Some("rev_test".to_string()),
        generated_at: chrono::Utc::now().timestamp_millis(),
        proposal_items: items,
    }
}

fn mk_review(status: review_types::ReviewOverallStatus) -> review_types::ReviewReport {
    review_types::ReviewReport {
        schema_version: review_types::REVIEW_SCHEMA_VERSION,
        review_id: "rev_test".to_string(),
        scope_ref: "chapter:vol1/ch1.json".to_string(),
        target_refs: vec!["manuscripts/vol1/ch1.json".to_string()],
        review_types: vec![review_types::ReviewType::WordCount],
        overall_status: status,
        issues: Vec::new(),
        evidence_summary: Vec::new(),
        recommended_action: review_types::ReviewRecommendedAction::Accept,
        generated_at: chrono::Utc::now().timestamp_millis(),
    }
}

fn mk_accepted_delta(
    bundle: &KnowledgeProposalBundle,
    delta_id: &str,
    accepted_item_ids: Vec<String>,
) -> KnowledgeDelta {
    KnowledgeDelta {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        knowledge_delta_id: delta_id.to_string(),
        status: KnowledgeDeltaStatus::Accepted,
        scope_ref: bundle.scope_ref.clone(),
        branch_id: bundle.branch_id.clone(),
        source_session_id: bundle.source_session_id.clone(),
        source_review_id: bundle.source_review_id.clone(),
        generated_at: bundle.generated_at,
        targets: Vec::new(),
        changes: Vec::new(),
        evidence_refs: Vec::new(),
        conflicts: Vec::new(),
        accepted_item_ids: Some(accepted_item_ids),
        rejected_item_ids: None,
        applied_at: None,
        rollback: None,
    }
}

fn mk_stored_object(kind: &str, revision: i64, fields: serde_json::Value) -> StoredKnowledgeObject {
    StoredKnowledgeObject {
        schema_version: STORED_OBJECT_SCHEMA_VERSION,
        r#ref: format!("{kind}:test"),
        kind: kind.to_string(),
        status: "accepted".to_string(),
        branch_id: None,
        revision,
        source_session_ids: vec!["s".to_string()],
        source_refs: vec!["r".to_string()],
        source_review_id: None,
        accepted_by: None,
        accepted_at: None,
        archived_at: None,
        superseded_by: None,
        superseded_at: None,
        created_at: 1,
        updated_at: 2,
        fields,
    }
}

fn write_stored_object(project_path: &Path, target_ref: &str, obj: &StoredKnowledgeObject) -> PathBuf {
    let root = crate::services::knowledge_paths::resolve_knowledge_root_for_write(project_path).unwrap();
    let full = root.join(target_ref);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    crate::utils::atomic_write::atomic_write_json(&full, obj).unwrap();
    full
}

