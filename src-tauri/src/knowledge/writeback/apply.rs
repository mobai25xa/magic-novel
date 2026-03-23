use std::path::Path;

use crate::knowledge::types::{
    KnowledgeDecisionActor, KnowledgeDelta, KnowledgeDeltaStatus, KnowledgeOp, KnowledgeProposalBundle,
    KnowledgeRollback, KnowledgeRollbackKind, KNOWLEDGE_CANON_CONFLICT, KNOWLEDGE_PROPOSAL_INVALID,
    KNOWLEDGE_REVISION_CONFLICT,
};
use crate::models::AppError;

use super::branch::validate_bundle_branch_active;
use super::path::{ensure_safe_relative_path, normalize_path};
use super::rollback::{rollback, rollback_dir, RollbackEntry, RollbackManifest};
use super::roots::knowledge_root_write;
use super::storage::{history_object_ref, read_stored_object, StoredKnowledgeObject, STORED_OBJECT_SCHEMA_VERSION};
use super::util::merge_unique;

pub(super) fn apply_accepted(
    project_path: &Path,
    mission_id: &str,
    bundle: &KnowledgeProposalBundle,
    delta: &KnowledgeDelta,
    actor: KnowledgeDecisionActor,
) -> Result<KnowledgeDelta, AppError> {
    if delta.status != KnowledgeDeltaStatus::Accepted {
        return Err(AppError::invalid_argument(
            "knowledge delta is not accepted; cannot apply",
        ));
    }
    if !delta.conflicts.is_empty() {
        return Err(AppError::invalid_argument(
            "knowledge delta has conflicts; cannot apply",
        ));
    }

    validate_bundle_branch_active(project_path, bundle)?;

    let accepted = delta.accepted_item_ids.clone().unwrap_or_default();
    if accepted.is_empty() {
        return Err(AppError::invalid_argument("no accepted_item_ids; nothing to apply"));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let accepted_by = match actor {
        KnowledgeDecisionActor::User => "user",
        KnowledgeDecisionActor::Orchestrator => "orchestrator",
    };
    let root = knowledge_root_write(project_path)?;

    let rollback_token = format!("rbk_{}", delta.knowledge_delta_id);
    let rb_dir = rollback_dir(project_path, mission_id, &rollback_token);
    std::fs::create_dir_all(&rb_dir)?;

    #[derive(Debug, Clone)]
    struct PlanEntry {
        item_idx: usize,
        target_ref: String,
        full_path: std::path::PathBuf,
        existed: bool,
        prev: Option<StoredKnowledgeObject>,
        history_ref: Option<String>,
    }

    // ── Preflight: validate all accepted items before any writes ─
    let mut plan: Vec<PlanEntry> = Vec::new();
    for item_id in &accepted {
        let item_idx = bundle
            .proposal_items
            .iter()
            .position(|it| it.item_id == *item_id)
            .ok_or_else(|| AppError::invalid_argument("accepted_item_id not found in bundle"))?;
        let item = &bundle.proposal_items[item_idx];

        let target_ref = item
            .target_ref
            .as_ref()
            .map(|s| normalize_path(s))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppError::invalid_argument(
                    "KNOWLEDGE_PROPOSAL_INVALID: accepted item missing target_ref",
                )
            })?;
        let full_path = root.join(ensure_safe_relative_path(&target_ref)?);
        let existed = full_path.exists();
        let mut prev: Option<StoredKnowledgeObject> = None;
        let mut history_ref: Option<String> = None;

        match item.op {
            KnowledgeOp::Create => {
                if existed {
                    return Err(AppError::invalid_argument(format!(
                        "{KNOWLEDGE_CANON_CONFLICT}: target exists for create"
                    )));
                }
            }
            KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore => {
                if !existed {
                    return Err(AppError::invalid_argument(format!(
                        "{KNOWLEDGE_CANON_CONFLICT}: target missing for update"
                    )));
                }
                let current = read_stored_object(&full_path)?.ok_or_else(|| {
                    AppError::invalid_argument(format!(
                        "{KNOWLEDGE_PROPOSAL_INVALID}: cannot read target object"
                    ))
                })?;
                if let Some(expected) = item.target_revision {
                    if current.revision != expected {
                        return Err(AppError::invalid_argument(format!(
                            "{KNOWLEDGE_REVISION_CONFLICT}: expected {expected}, found {}",
                            current.revision
                        )));
                    }
                }
                history_ref = Some(history_object_ref(&target_ref, current.revision));
                prev = Some(current);
            }
        }

        plan.push(PlanEntry {
            item_idx,
            target_ref,
            full_path,
            existed,
            prev,
            history_ref,
        });
    }

    // ── Stage backups and persist manifest BEFORE applying writes ─
    let mut manifest = RollbackManifest {
        schema_version: 1,
        token: rollback_token.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        created_at: now,
        entries: Vec::new(),
    };

    for (idx, p) in plan.iter().enumerate() {
        let backup_file = if p.existed {
            let raw = std::fs::read_to_string(&p.full_path)?;
            let name = format!("entry_{idx}.bak.json");
            std::fs::write(rb_dir.join(&name), raw)?;
            Some(name)
        } else {
            None
        };

        manifest.entries.push(RollbackEntry {
            rel_path: p.target_ref.clone(),
            existed: p.existed,
            backup_file,
        });

        if let Some(history_ref) = p.history_ref.as_ref() {
            manifest.entries.push(RollbackEntry {
                rel_path: history_ref.clone(),
                existed: false,
                backup_file: None,
            });
        }
    }

    crate::utils::atomic_write::atomic_write_json(&rb_dir.join("manifest.json"), &manifest)?;

    // ── Apply writes ─
    let apply_result: Result<(), AppError> = (|| {
        for p in &plan {
            let item = &bundle.proposal_items[p.item_idx];

            if let Some(parent) = p.full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Re-check revision just before write (best-effort OCC).
            if matches!(
                item.op,
                KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore
            ) {
                let current = read_stored_object(&p.full_path)?.ok_or_else(|| {
                    AppError::invalid_argument(format!(
                        "{KNOWLEDGE_CANON_CONFLICT}: target missing for update"
                    ))
                })?;
                if let Some(expected) = item.target_revision {
                    if current.revision != expected {
                        return Err(AppError::invalid_argument(format!(
                            "{KNOWLEDGE_REVISION_CONFLICT}: expected {expected}, found {}",
                            current.revision
                        )));
                    }
                }
            }

            let (
                created_at,
                next_revision,
                mut source_session_ids,
                mut source_refs,
                existing_source_review_id,
                previous_archived_at,
            ) = match p.prev.clone() {
                Some(obj) => (
                    obj.created_at,
                    obj.revision.saturating_add(1),
                    obj.source_session_ids,
                    obj.source_refs,
                    obj.source_review_id,
                    obj.archived_at,
                ),
                None => (
                    now,
                    1,
                    vec![bundle.source_session_id.clone()],
                    Vec::new(),
                    None,
                    None,
                ),
            };

            if let (Some(prev), Some(history_ref)) = (p.prev.as_ref(), p.history_ref.as_ref()) {
                let history_path = root.join(ensure_safe_relative_path(history_ref)?);
                if let Some(parent) = history_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut superseded = prev.clone();
                superseded.status = "superseded".to_string();
                superseded.superseded_by = Some(format!("{}:{}", item.kind, p.target_ref));
                superseded.superseded_at = Some(now);
                superseded.updated_at = now;

                crate::utils::atomic_write::atomic_write_json(&history_path, &superseded)?;
            }

            let source_review_id = bundle
                .source_review_id
                .clone()
                .or(existing_source_review_id);

            source_session_ids = merge_unique(source_session_ids, &[bundle.source_session_id.clone()]);
            source_refs = merge_unique(source_refs, &item.source_refs);

            let status = match item.op {
                KnowledgeOp::Archive => "archived",
                _ => "accepted",
            };
            let archived_at = match item.op {
                KnowledgeOp::Archive => Some(now),
                KnowledgeOp::Restore => None,
                KnowledgeOp::Update => None,
                KnowledgeOp::Create => previous_archived_at,
            };

            let stored = StoredKnowledgeObject {
                schema_version: STORED_OBJECT_SCHEMA_VERSION,
                r#ref: format!("{}:{}", item.kind, p.target_ref),
                kind: item.kind.clone(),
                status: status.to_string(),
                branch_id: bundle.branch_id.clone(),
                revision: next_revision,
                source_session_ids,
                source_refs,
                source_review_id,
                accepted_by: Some(accepted_by.to_string()),
                accepted_at: Some(now),
                archived_at,
                superseded_by: None,
                superseded_at: None,
                created_at,
                updated_at: now,
                fields: item.fields.clone(),
            };

            crate::utils::atomic_write::atomic_write_json(&p.full_path, &stored)?;
        }
        Ok(())
    })();

    if let Err(e) = apply_result {
        // Best-effort rollback to avoid partial pollution.
        let rb = rollback(project_path, mission_id, &rollback_token);
        let rb_summary = rb
            .map(|(restored, deleted)| format!("restored={restored} deleted={deleted}"))
            .unwrap_or_else(|re| format!("rollback_failed: {re}"));
        return Err(AppError::internal(format!(
            "apply failed; rolled back ({rb_summary}); token={rollback_token}; error={e}"
        )));
    }

    let mut out = delta.clone();
    out.status = KnowledgeDeltaStatus::Applied;
    out.applied_at = Some(now);
    out.rollback = Some(KnowledgeRollback {
        kind: KnowledgeRollbackKind::Hard,
        token: Some(rollback_token),
    });
    Ok(out)
}

