mod foreshadow;
mod policy;
mod recent_fact;

use std::collections::HashMap;
use std::path::Path;

use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeConflict, KnowledgeDelta, KnowledgeDeltaChange,
    KnowledgeDeltaStatus, KnowledgeDeltaTarget, KnowledgeOp, KnowledgeProposalBundle,
    KNOWLEDGE_BRANCH_STALE, KNOWLEDGE_CANON_CONFLICT, KNOWLEDGE_POLICY_CONFLICT,
    KNOWLEDGE_PROPOSAL_INVALID, KNOWLEDGE_REVIEW_BLOCKED, KNOWLEDGE_REVISION_CONFLICT,
    KNOWLEDGE_SOURCE_MISSING,
};
use crate::models::AppError;
use crate::review::types as review_types;

use super::branch::branch_stale_reason;
use super::path::{ensure_safe_relative_path, normalize_path};
use super::storage::{read_stored_object, stored_object_path};
use super::util::merge_unique;

use foreshadow::foreshadow_status_regresses;
use policy::{kind_allows_auto_if_pass, validate_auto_policy_fields};
use recent_fact::{load_existing_recent_fact_index, normalize_summary_key, recent_fact_dir_ref};

fn add_conflict(
    conflicts: &mut Vec<KnowledgeConflict>,
    conflict_type: &str,
    message: impl Into<String>,
    item_id: Option<String>,
    target_ref: Option<String>,
) {
    conflicts.push(KnowledgeConflict {
        conflict_type: conflict_type.to_string(),
        message: message.into(),
        item_id,
        target_ref,
    });
}

pub(super) fn gate_bundle(
    project_path: &Path,
    bundle: &KnowledgeProposalBundle,
    review: Option<&review_types::ReviewReport>,
) -> Result<KnowledgeDelta, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut delta = KnowledgeDelta {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        knowledge_delta_id: format!("kdelta_{}", uuid::Uuid::new_v4()),
        status: KnowledgeDeltaStatus::Proposed,
        scope_ref: bundle.scope_ref.clone(),
        branch_id: bundle.branch_id.clone(),
        source_session_id: bundle.source_session_id.clone(),
        source_review_id: bundle.source_review_id.clone(),
        generated_at: now,
        targets: Vec::new(),
        changes: Vec::new(),
        evidence_refs: Vec::new(),
        conflicts: Vec::new(),
        accepted_item_ids: None,
        rejected_item_ids: None,
        applied_at: None,
        rollback: None,
    };

    if let Some(reason) = branch_stale_reason(project_path, bundle.branch_id.as_ref()) {
        add_conflict(
            &mut delta.conflicts,
            KNOWLEDGE_BRANCH_STALE,
            reason,
            None,
            None,
        );
    }

    if bundle.source_session_id.trim().is_empty() {
        add_conflict(
            &mut delta.conflicts,
            KNOWLEDGE_SOURCE_MISSING,
            "bundle.source_session_id is missing",
            None,
            None,
        );
    }

    // Review must not be block (if present).
    if let (Some(src_review_id), Some(report)) = (bundle.source_review_id.as_ref(), review) {
        if report.review_id == *src_review_id && report.overall_status == review_types::ReviewOverallStatus::Block
        {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_REVIEW_BLOCKED,
                "review overall_status=block; cannot accept/apply",
                None,
                None,
            );
        }
    }

    // Build targets/changes and detect conflicts.
    let mut recent_fact_index_cache: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut recent_fact_seen_in_bundle: HashMap<String, HashMap<String, String>> = HashMap::new();
    for item in &bundle.proposal_items {
        let target_ref = item
            .target_ref
            .as_ref()
            .map(|s| normalize_path(s))
            .filter(|s| !s.is_empty());

        if let Some(tr) = target_ref.as_ref() {
            delta.targets.push(KnowledgeDeltaTarget {
                r#ref: tr.to_string(),
                kind: item.kind.clone(),
                path: Some(format!(".magic_novel/{tr}")),
            });
        }

        delta.changes.push(KnowledgeDeltaChange {
            item_id: item.item_id.clone(),
            op: serde_json::to_string(&item.op)
                .unwrap_or_else(|_| "\"create\"".to_string())
                .trim_matches('"')
                .to_string(),
            kind: item.kind.clone(),
            target_ref: target_ref.as_ref().map(|s| s.to_string()),
            summary: item.change_reason.clone(),
        });

        if item.source_refs.is_empty() {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_SOURCE_MISSING,
                "proposal item missing source_refs",
                Some(item.item_id.clone()),
                target_ref.as_ref().map(|s| s.to_string()),
            );
        }

        if !item.fields.is_object() {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_PROPOSAL_INVALID,
                "proposal item fields must be an object",
                Some(item.item_id.clone()),
                target_ref.as_ref().map(|s| s.to_string()),
            );
        }

        if item.accept_policy == KnowledgeAcceptPolicy::AutoIfPass
            && (!kind_allows_auto_if_pass(&item.kind)
                || !validate_auto_policy_fields(&item.kind, &item.fields))
        {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_POLICY_CONFLICT,
                "accept_policy=auto_if_pass is not allowed for this kind/fields",
                Some(item.item_id.clone()),
                target_ref.as_ref().map(|s| s.to_string()),
            );
        }

        let mut evidence = item
            .evidence_refs
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if evidence.is_empty() {
            evidence = item
                .source_refs
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
        }
        delta.evidence_refs = merge_unique(delta.evidence_refs, &evidence);

        let Some(tr) = target_ref.as_ref() else {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_PROPOSAL_INVALID,
                "proposal item missing target_ref",
                Some(item.item_id.clone()),
                None,
            );
            continue;
        };

        if ensure_safe_relative_path(tr).is_err() {
            add_conflict(
                &mut delta.conflicts,
                KNOWLEDGE_PROPOSAL_INVALID,
                "unsafe target_ref",
                Some(item.item_id.clone()),
                Some(tr.to_string()),
            );
            continue;
        }

        // Semantic dedupe: recent_fact summary must not duplicate accepted truth within the same dir.
        if item.kind == "recent_fact" {
            if let Some(summary) = item.fields.get("summary").and_then(|v| v.as_str()) {
                let key = normalize_summary_key(summary);
                if !key.is_empty() {
                    if let Some(dir_ref) = recent_fact_dir_ref(tr) {
                        let dir_seen = recent_fact_seen_in_bundle.entry(dir_ref.clone()).or_default();
                        if let Some(prev_item_id) = dir_seen.get(&key) {
                            add_conflict(
                                &mut delta.conflicts,
                                KNOWLEDGE_CANON_CONFLICT,
                                format!(
                                    "duplicate recent_fact summary within bundle (matches item_id={prev_item_id})"
                                ),
                                Some(item.item_id.clone()),
                                Some(tr.to_string()),
                            );
                        } else {
                            dir_seen.insert(key.clone(), item.item_id.clone());
                        }

                        let idx = recent_fact_index_cache
                            .entry(dir_ref.clone())
                            .or_insert_with(|| load_existing_recent_fact_index(project_path, &dir_ref));
                        if let Some((existing_ref, _)) = idx
                            .iter()
                            .find(|(r, s)| s == &key && r.as_str() != tr)
                        {
                            add_conflict(
                                &mut delta.conflicts,
                                KNOWLEDGE_CANON_CONFLICT,
                                format!(
                                    "duplicate recent_fact summary already accepted at {existing_ref}"
                                ),
                                Some(item.item_id.clone()),
                                Some(tr.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Target existence and revision conflict checks.
        let p = stored_object_path(project_path, tr);
        match item.op {
            KnowledgeOp::Create => {
                if p.exists() {
                    add_conflict(
                        &mut delta.conflicts,
                        KNOWLEDGE_CANON_CONFLICT,
                        "target exists for create",
                        Some(item.item_id.clone()),
                        Some(tr.to_string()),
                    );
                }
            }
            KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore => {
                match read_stored_object(&p) {
                    Ok(None) => add_conflict(
                        &mut delta.conflicts,
                        KNOWLEDGE_CANON_CONFLICT,
                        "target missing for update",
                        Some(item.item_id.clone()),
                        Some(tr.to_string()),
                    ),
                    Ok(Some(obj)) => {
                        if let Some(expected) = item.target_revision {
                            if obj.revision != expected {
                                add_conflict(
                                    &mut delta.conflicts,
                                    KNOWLEDGE_REVISION_CONFLICT,
                                    format!(
                                        "revision mismatch: expected {expected}, found {}",
                                        obj.revision
                                    ),
                                    Some(item.item_id.clone()),
                                    Some(tr.to_string()),
                                );
                            }
                        }

                        // Semantic contradiction: foreshadow status must not regress.
                        if item.kind == "foreshadow" {
                            if let (Some(prev), Some(next)) = (
                                obj.fields.get("status_label").and_then(|v| v.as_str()),
                                item.fields.get("status_label").and_then(|v| v.as_str()),
                            ) {
                                if foreshadow_status_regresses(prev, next) {
                                    add_conflict(
                                        &mut delta.conflicts,
                                        KNOWLEDGE_CANON_CONFLICT,
                                        format!(
                                            "foreshadow status regressed: prev={prev} next={next}"
                                        ),
                                        Some(item.item_id.clone()),
                                        Some(tr.to_string()),
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => add_conflict(
                        &mut delta.conflicts,
                        KNOWLEDGE_CANON_CONFLICT,
                        format!("target unreadable: {e}"),
                        Some(item.item_id.clone()),
                        Some(tr.to_string()),
                    ),
                }
            }
        }
    }

    // Auto-accept only when review=pass and there are no global conflicts.
    let can_auto_accept = review
        .map(|r| r.overall_status == review_types::ReviewOverallStatus::Pass)
        .unwrap_or(false);

    let has_global_conflict = delta.conflicts.iter().any(|c| c.item_id.is_none());
    let mut accepted = Vec::new();
    if can_auto_accept && !has_global_conflict {
        for item in &bundle.proposal_items {
            if item.accept_policy != KnowledgeAcceptPolicy::AutoIfPass {
                continue;
            }
            let conflicted = delta
                .conflicts
                .iter()
                .any(|c| c.item_id.as_deref() == Some(item.item_id.as_str()));
            if conflicted {
                continue;
            }
            accepted.push(item.item_id.clone());
        }
    }

    if !accepted.is_empty() {
        delta.accepted_item_ids = Some(accepted);
    }

    if delta.conflicts.is_empty() {
        let all_auto_accepted = delta
            .accepted_item_ids
            .as_ref()
            .map(|ids| ids.len() == bundle.proposal_items.len() && !ids.is_empty())
            .unwrap_or(false);
        if all_auto_accepted {
            delta.status = KnowledgeDeltaStatus::Accepted;
        }
    }

    Ok(delta)
}

