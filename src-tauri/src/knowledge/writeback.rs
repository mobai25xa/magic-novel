use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeConflict, KnowledgeDecisionInput, KnowledgeDelta,
    KnowledgeDeltaChange, KnowledgeDeltaStatus, KnowledgeDeltaTarget, KnowledgeOp,
    KnowledgeProposalBundle, KnowledgeProposalItem, KnowledgeRollback, KnowledgeRollbackKind,
    PendingKnowledgeDecision, KNOWLEDGE_CANON_CONFLICT, KNOWLEDGE_POLICY_CONFLICT,
    KNOWLEDGE_PROPOSAL_INVALID, KNOWLEDGE_REVIEW_BLOCKED, KNOWLEDGE_REVISION_CONFLICT,
    KNOWLEDGE_SOURCE_MISSING,
};
use crate::mission::artifacts;
use crate::models::{AppError, Chapter};
use crate::review::types as review_types;

const STORED_OBJECT_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredKnowledgeObject {
    pub schema_version: i32,
    pub r#ref: String,
    pub kind: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    pub revision: i64,
    #[serde(default)]
    pub source_session_ids: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub fields: serde_json::Value,
}

fn normalize_path(input: &str) -> String {
    let mut p = input.trim().replace('\\', "/");
    while p.starts_with("./") {
        p = p.trim_start_matches("./").to_string();
    }
    while p.contains("//") {
        p = p.replace("//", "/");
    }
    p.trim_matches('/').to_string()
}

fn ensure_safe_relative_path(rel: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(rel);
    if p.is_absolute() {
        return Err(AppError::invalid_argument(format!(
            "{KNOWLEDGE_PROPOSAL_INVALID}: target_ref must be a relative path"
        )));
    }

    for c in p.components() {
        match c {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::invalid_argument(format!(
                    "{KNOWLEDGE_PROPOSAL_INVALID}: unsafe target_ref"
                )));
            }
            _ => {}
        }
    }

    Ok(p)
}

fn slugify_locator(locator: &str) -> String {
    normalize_path(locator)
        .trim_end_matches(".json")
        .trim_end_matches(".md")
        .replace(&['/', ' ', ':'][..], "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}

fn infer_chapter_locator_from_write_paths(write_paths: &[String]) -> Option<String> {
    for p in write_paths {
        let p = normalize_path(p);
        if p.starts_with("manuscripts/") && p.ends_with(".json") && !p.ends_with("/volume.json") {
            return Some(p.trim_start_matches("manuscripts/").to_string());
        }
    }
    None
}

fn knowledge_root_read(project_path: &Path) -> PathBuf {
    crate::services::knowledge_paths::resolve_knowledge_root_for_read(project_path)
}

fn knowledge_root_write(project_path: &Path) -> Result<PathBuf, AppError> {
    crate::services::knowledge_paths::resolve_knowledge_root_for_write(project_path)
}

fn stored_object_path(project_path: &Path, target_ref: &str) -> PathBuf {
    knowledge_root_read(project_path).join(target_ref)
}

fn read_stored_object(path: &Path) -> Result<Option<StoredKnowledgeObject>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let obj: StoredKnowledgeObject = serde_json::from_str(&raw).map_err(|e| {
        AppError::invalid_argument(format!(
            "KNOWLEDGE_PROPOSAL_INVALID: stored object parse error at {}: {e}",
            path.to_string_lossy()
        ))
    })?;
    Ok(Some(obj))
}

fn merge_unique(mut base: Vec<String>, extra: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = base.iter().cloned().collect();
    for s in extra {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        if seen.insert(s.to_string()) {
            base.push(s.to_string());
        }
    }
    base
}

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

fn kind_allows_auto_if_pass(kind: &str) -> bool {
    matches!(kind, "chapter_summary" | "recent_fact" | "foreshadow")
}

fn validate_auto_policy_fields(kind: &str, fields: &serde_json::Value) -> bool {
    if kind != "foreshadow" {
        return true;
    }

    let Some(obj) = fields.as_object() else {
        return false;
    };

    // M4 P1: only allow lightweight foreshadow status progression to be auto-accepted.
    for k in obj.keys() {
        if !matches!(k.as_str(), "status_label" | "current_notes" | "seed_ref") {
            return false;
        }
    }

    obj.contains_key("status_label")
}

pub fn generate_proposal_bundle_after_closeout(
    project_path: &Path,
    mission_id: &str,
    scope_ref: String,
    write_paths: Vec<String>,
    source_session_id: String,
    source_review_id: Option<String>,
) -> Result<KnowledgeProposalBundle, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let bundle_id = format!("kbundle_{}", uuid::Uuid::new_v4());

    let layer1_cc = artifacts::read_layer1_chapter_card(project_path, mission_id)
        .ok()
        .flatten();
    let locator = infer_chapter_locator_from_write_paths(&write_paths)
        .or_else(|| layer1_cc.as_ref().and_then(|cc| cc.scope_locator.clone()))
        .map(|s| normalize_path(&s));

    let chapter_locator = locator.clone().unwrap_or_else(|| format!("unknown/{bundle_id}.json"));
    let chapter_slug = slugify_locator(&chapter_locator);

    // ── Chapter summary proposal ───────────────────────────────
    let chapter_target_ref = format!("chapter_summaries/{chapter_locator}");
    let chapter_obj_path = stored_object_path(project_path, &chapter_target_ref);
    let existing = read_stored_object(&chapter_obj_path)?;

    let (chapter_op, chapter_target_revision) = match existing {
        Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
        None => (KnowledgeOp::Create, None),
    };

    let chapter_summary_fields = load_chapter_summary_fields(project_path, &chapter_locator)
        .unwrap_or_else(|| {
            json!({
                "chapter_locator": chapter_locator,
                "summary": "",
                "key_events": [],
                "state_changes": []
            })
        });

    let mut proposal_items: Vec<KnowledgeProposalItem> = vec![KnowledgeProposalItem {
        item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
        kind: "chapter_summary".to_string(),
        op: chapter_op,
        target_ref: Some(chapter_target_ref),
        target_revision: chapter_target_revision,
        fields: chapter_summary_fields,
        evidence_refs: write_paths
            .iter()
            .map(|p| normalize_path(p))
            .filter(|p| !p.is_empty())
            .take(8)
            .collect(),
        source_refs: vec![format!("scope_ref:{scope_ref}")],
        change_reason: "chapter closeout: update chapter summary".to_string(),
        accept_policy: KnowledgeAcceptPolicy::AutoIfPass,
    }];

    // ── Recent facts (from Layer1) ─────────────────────────────
    if let Ok(Some(rf)) = artifacts::read_layer1_recent_facts(project_path, mission_id) {
        for (idx, fact) in rf.facts.iter().enumerate() {
            let summary = fact.summary.trim();
            if summary.is_empty() {
                continue;
            }
            let target_ref = format!("recent_facts/{chapter_slug}/fact_{}.json", idx + 1);
            let obj_path = stored_object_path(project_path, &target_ref);
            let existing = read_stored_object(&obj_path)?;
            let (op, target_revision) = match existing {
                Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
                None => (KnowledgeOp::Create, None),
            };

            let confidence = match fact.confidence {
                crate::mission::layer1_types::FactConfidence::Accepted => "accepted",
                crate::mission::layer1_types::FactConfidence::Proposed => "proposed",
            };

            let mut evidence = Vec::new();
            if !fact.source_ref.trim().is_empty() {
                evidence.push(fact.source_ref.trim().to_string());
            }
            if evidence.is_empty() {
                evidence.extend(
                    write_paths
                        .iter()
                        .map(|p| normalize_path(p))
                        .filter(|p| !p.is_empty())
                        .take(4),
                );
            }
            if evidence.is_empty() {
                evidence.push(format!("scope_ref:{scope_ref}"));
            }

            proposal_items.push(KnowledgeProposalItem {
                item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
                kind: "recent_fact".to_string(),
                op,
                target_ref: Some(target_ref),
                target_revision,
                fields: json!({
                    "scope_ref": rf.scope_ref,
                    "summary": summary,
                    "source_ref": fact.source_ref,
                    "fact_scope": "chapter",
                    "confidence": confidence,
                    "subject_refs": []
                }),
                evidence_refs: evidence.clone(),
                source_refs: vec![format!("scope_ref:{scope_ref}")],
                change_reason: "chapter closeout: recent fact".to_string(),
                accept_policy: KnowledgeAcceptPolicy::AutoIfPass,
            });
        }
    }

    // Ensure every item has at least one source_ref.
    for it in &mut proposal_items {
        if it.source_refs.is_empty() {
            it.source_refs.push(format!("scope_ref:{scope_ref}"));
        }
        if !it.fields.is_object() {
            it.fields = json!({});
        }
    }

    Ok(KnowledgeProposalBundle {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id,
        scope_ref,
        branch_id: None,
        source_session_id,
        source_review_id,
        generated_at: now,
        proposal_items,
    })
}

fn load_chapter_summary_fields(project_path: &Path, chapter_locator: &str) -> Option<serde_json::Value> {
    let p = project_path.join("manuscripts").join(chapter_locator);
    let ch: Chapter = crate::services::read_json(&p).ok()?;
    let summary = ch.summary.unwrap_or_default();
    Some(json!({
        "chapter_locator": chapter_locator,
        "chapter_id": ch.id,
        "chapter_title": ch.title,
        "summary": summary,
        "key_events": [],
        "state_changes": []
    }))
}

pub fn gate_bundle(
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
        if report.review_id == *src_review_id
            && report.overall_status == review_types::ReviewOverallStatus::Block
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

        // Revision conflict check for updates.
        if item.op == KnowledgeOp::Update {
            let p = stored_object_path(project_path, tr);
            match read_stored_object(&p)? {
                None => add_conflict(
                    &mut delta.conflicts,
                    KNOWLEDGE_CANON_CONFLICT,
                    "target missing for update",
                    Some(item.item_id.clone()),
                    Some(tr.to_string()),
                ),
                Some(obj) => {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RollbackManifest {
    pub schema_version: i32,
    pub token: String,
    pub delta_id: String,
    pub created_at: i64,
    pub entries: Vec<RollbackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RollbackEntry {
    pub rel_path: String,
    pub existed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_file: Option<String>,
}

fn rollback_dir(project_path: &Path, mission_id: &str, token: &str) -> PathBuf {
    artifacts::knowledge_dir(project_path, mission_id)
        .join("rollback")
        .join(token)
}

pub fn apply_accepted(
    project_path: &Path,
    mission_id: &str,
    bundle: &KnowledgeProposalBundle,
    delta: &KnowledgeDelta,
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

    let accepted = delta.accepted_item_ids.clone().unwrap_or_default();
    if accepted.is_empty() {
        return Err(AppError::invalid_argument(
            "no accepted_item_ids; nothing to apply",
        ));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let root = knowledge_root_write(project_path)?;

    let rollback_token = format!("rbk_{}", delta.knowledge_delta_id);
    let rb_dir = rollback_dir(project_path, mission_id, &rollback_token);
    std::fs::create_dir_all(&rb_dir)?;

    #[derive(Debug, Clone)]
    struct PlanEntry {
        item_idx: usize,
        target_ref: String,
        full_path: PathBuf,
        existed: bool,
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
            }
        }

        plan.push(PlanEntry {
            item_idx,
            target_ref,
            full_path,
            existed,
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
            if matches!(item.op, KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore) {
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

            let prev = read_stored_object(&p.full_path)?;
            let (created_at, next_revision, mut source_session_ids, mut source_refs) = match prev {
                Some(obj) => (
                    obj.created_at,
                    obj.revision.saturating_add(1),
                    obj.source_session_ids,
                    obj.source_refs,
                ),
                None => (
                    now,
                    1,
                    vec![bundle.source_session_id.clone()],
                    Vec::new(),
                ),
            };

            source_session_ids =
                merge_unique(source_session_ids, &[bundle.source_session_id.clone()]);
            source_refs = merge_unique(source_refs, &item.source_refs);

            let status = match item.op {
                KnowledgeOp::Archive => "archived",
                _ => "accepted",
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

pub fn rollback(
    project_path: &Path,
    mission_id: &str,
    token: &str,
) -> Result<(usize, usize), AppError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::invalid_argument("rollback token is required"));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let root = knowledge_root_write(project_path)?;
    let rb_dir = rollback_dir(project_path, mission_id, token);
    let manifest_path = rb_dir.join("manifest.json");

    if !manifest_path.exists() {
        return Err(AppError::not_found("rollback manifest not found"));
    }

    let raw = std::fs::read_to_string(&manifest_path)?;
    let manifest: RollbackManifest = serde_json::from_str(&raw)?;

    let mut restored = 0_usize;
    let mut deleted = 0_usize;
    for entry in &manifest.entries {
        let rel = ensure_safe_relative_path(&entry.rel_path)?;
        let full = root.join(rel);
        if entry.existed {
            let Some(bf) = entry.backup_file.as_ref() else {
                return Err(AppError::invalid_argument(
                    "rollback manifest missing backup_file",
                ));
            };
            let prev = std::fs::read_to_string(rb_dir.join(bf))?;
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::utils::atomic_write::atomic_write(&full, &prev)?;
            restored += 1;
        } else {
            if full.exists() {
                std::fs::remove_file(&full)?;
            }
            deleted += 1;
        }
    }

    // Touch a marker for audit.
    let _ = std::fs::write(rb_dir.join("rolled_back_at.txt"), format!("{now}"));

    Ok((restored, deleted))
}

pub fn build_pending_decision(bundle: &KnowledgeProposalBundle, delta: &KnowledgeDelta) -> PendingKnowledgeDecision {
    PendingKnowledgeDecision {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: bundle.bundle_id.clone(),
        delta_id: delta.knowledge_delta_id.clone(),
        scope_ref: delta.scope_ref.clone(),
        conflicts: delta.conflicts.clone(),
        created_at: chrono::Utc::now().timestamp_millis(),
    }
}

pub fn apply_decision_to_delta(
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

    let bundle_ids: HashSet<String> = bundle.proposal_items.iter().map(|i| i.item_id.clone()).collect();
    for id in accepted_set.iter().chain(rejected_set.iter()) {
        if !bundle_ids.contains(id) {
            return Err(AppError::invalid_argument(
                "decision references unknown item_id",
            ));
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

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::types::{
        KnowledgeAcceptPolicy, KnowledgeDecisionInput, KnowledgeDeltaStatus,
        KnowledgeOp, KnowledgeProposalBundle, KnowledgeProposalItem,
        KNOWLEDGE_REVISION_CONFLICT,
    };

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
            branch_id: None,
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

    #[test]
    fn serde_roundtrip_bundle_and_delta() {
        let item = mk_item(
            "chapter_summary",
            KnowledgeOp::Create,
            "chapter_summaries/vol1/ch1.json",
            None,
            KnowledgeAcceptPolicy::AutoIfPass,
            json!({"summary": "x"}),
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

    #[test]
    fn gate_auto_accepts_only_on_review_pass() {
        let i1 = mk_item(
            "chapter_summary",
            KnowledgeOp::Create,
            "chapter_summaries/vol1/ch1.json",
            None,
            KnowledgeAcceptPolicy::AutoIfPass,
            json!({"summary": "x"}),
        );
        let i2 = mk_item(
            "recent_fact",
            KnowledgeOp::Create,
            "recent_facts/vol1_ch1/f1.json",
            None,
            KnowledgeAcceptPolicy::AutoIfPass,
            json!({"summary": "y"}),
        );
        let bundle = mk_bundle(vec![i1.clone(), i2.clone()]);

        let pass = mk_review(review_types::ReviewOverallStatus::Pass);
        let delta = gate_bundle(&temp_project_dir(), &bundle, Some(&pass)).unwrap();
        assert_eq!(delta.status, KnowledgeDeltaStatus::Accepted);
        assert_eq!(
            delta.accepted_item_ids.unwrap_or_default().len(),
            bundle.proposal_items.len()
        );

        let warn = mk_review(review_types::ReviewOverallStatus::Warn);
        let delta = gate_bundle(&temp_project_dir(), &bundle, Some(&warn)).unwrap();
        assert_eq!(delta.status, KnowledgeDeltaStatus::Proposed);
        assert!(delta.accepted_item_ids.is_none());
    }

    #[test]
    fn gate_detects_revision_conflict() {
        let project = temp_project_dir();
        let root = crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project).unwrap();
        let target_ref = "chapter_summaries/vol1/ch1.json";
        let full = root.join(target_ref);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        crate::utils::atomic_write::atomic_write_json(
            &full,
            &StoredKnowledgeObject {
                schema_version: STORED_OBJECT_SCHEMA_VERSION,
                r#ref: "chapter_summary:vol1/ch1".to_string(),
                kind: "chapter_summary".to_string(),
                status: "accepted".to_string(),
                branch_id: None,
                revision: 2,
                source_session_ids: vec!["s".to_string()],
                source_refs: vec!["r".to_string()],
                created_at: 1,
                updated_at: 2,
                fields: json!({"summary": "old"}),
            },
        )
        .unwrap();

        let item = mk_item(
            "chapter_summary",
            KnowledgeOp::Update,
            target_ref,
            Some(1),
            KnowledgeAcceptPolicy::Manual,
            json!({"summary": "new"}),
        );
        let bundle = mk_bundle(vec![item.clone()]);
        let delta = gate_bundle(&project, &bundle, None).unwrap();
        assert!(delta
            .conflicts
            .iter()
            .any(|c| c.conflict_type == KNOWLEDGE_REVISION_CONFLICT));
    }

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
            json!({"summary": "hello"}),
        );
        let bundle = mk_bundle(vec![item.clone()]);
        let delta = KnowledgeDelta {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            knowledge_delta_id: "kdelta_apply_create".to_string(),
            status: KnowledgeDeltaStatus::Accepted,
            scope_ref: bundle.scope_ref.clone(),
            branch_id: None,
            source_session_id: bundle.source_session_id.clone(),
            source_review_id: bundle.source_review_id.clone(),
            generated_at: bundle.generated_at,
            targets: Vec::new(),
            changes: Vec::new(),
            evidence_refs: Vec::new(),
            conflicts: Vec::new(),
            accepted_item_ids: Some(vec![item.item_id.clone()]),
            rejected_item_ids: None,
            applied_at: None,
            rollback: None,
        };

        let applied = apply_accepted(&project, mission_id, &bundle, &delta).unwrap();
        let token = applied.rollback.as_ref().and_then(|r| r.token.clone()).unwrap();

        let root = crate::services::knowledge_paths::resolve_knowledge_root_for_read(&project);
        assert!(root.join("terms/foo.json").exists());

        let (_restored, _deleted) = rollback(&project, mission_id, &token).unwrap();
        assert!(!root.join("terms/foo.json").exists());
    }

    #[test]
    fn apply_update_then_rollback_restores_previous_content() {
        let project = temp_project_dir();
        let mission_id = "mis_test_apply_update";

        let root = crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project).unwrap();
        let target_ref = "terms/foo.json";
        let full = root.join(target_ref);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        crate::utils::atomic_write::atomic_write_json(
            &full,
            &StoredKnowledgeObject {
                schema_version: STORED_OBJECT_SCHEMA_VERSION,
                r#ref: "term:foo".to_string(),
                kind: "term".to_string(),
                status: "accepted".to_string(),
                branch_id: None,
                revision: 5,
                source_session_ids: vec!["s".to_string()],
                source_refs: vec!["r".to_string()],
                created_at: 1,
                updated_at: 2,
                fields: json!({"a": 1}),
            },
        )
        .unwrap();

        let item = mk_item(
            "term",
            KnowledgeOp::Update,
            target_ref,
            Some(5),
            KnowledgeAcceptPolicy::Manual,
            json!({"a": 2}),
        );
        let bundle = mk_bundle(vec![item.clone()]);
        let delta = KnowledgeDelta {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            knowledge_delta_id: "kdelta_apply_update".to_string(),
            status: KnowledgeDeltaStatus::Accepted,
            scope_ref: bundle.scope_ref.clone(),
            branch_id: None,
            source_session_id: bundle.source_session_id.clone(),
            source_review_id: bundle.source_review_id.clone(),
            generated_at: bundle.generated_at,
            targets: Vec::new(),
            changes: Vec::new(),
            evidence_refs: Vec::new(),
            conflicts: Vec::new(),
            accepted_item_ids: Some(vec![item.item_id.clone()]),
            rejected_item_ids: None,
            applied_at: None,
            rollback: None,
        };

        let applied = apply_accepted(&project, mission_id, &bundle, &delta).unwrap();
        let token = applied.rollback.as_ref().and_then(|r| r.token.clone()).unwrap();

        let raw = std::fs::read_to_string(&full).unwrap();
        let obj: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
        assert_eq!(obj.revision, 6);
        assert_eq!(obj.fields["a"], json!(2));

        rollback(&project, mission_id, &token).unwrap();
        let raw = std::fs::read_to_string(&full).unwrap();
        let obj: StoredKnowledgeObject = serde_json::from_str(&raw).unwrap();
        assert_eq!(obj.revision, 5);
        assert_eq!(obj.fields["a"], json!(1));
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
            json!({"a": 1}),
        );
        let i2 = mk_item(
            "term",
            KnowledgeOp::Update,
            "terms/missing.json",
            Some(1),
            KnowledgeAcceptPolicy::Manual,
            json!({"a": 2}),
        );
        let bundle = mk_bundle(vec![i1.clone(), i2.clone()]);
        let delta = KnowledgeDelta {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            knowledge_delta_id: "kdelta_preflight".to_string(),
            status: KnowledgeDeltaStatus::Accepted,
            scope_ref: bundle.scope_ref.clone(),
            branch_id: None,
            source_session_id: bundle.source_session_id.clone(),
            source_review_id: bundle.source_review_id.clone(),
            generated_at: bundle.generated_at,
            targets: Vec::new(),
            changes: Vec::new(),
            evidence_refs: Vec::new(),
            conflicts: Vec::new(),
            accepted_item_ids: Some(vec![i1.item_id.clone(), i2.item_id.clone()]),
            rejected_item_ids: None,
            applied_at: None,
            rollback: None,
        };

        let err = apply_accepted(&project, mission_id, &bundle, &delta).unwrap_err();
        assert!(err.message.contains("KNOWLEDGE_CANON_CONFLICT"));

        let root = crate::services::knowledge_paths::resolve_knowledge_root_for_read(&project);
        assert!(!root.join("terms/one.json").exists());
    }

    #[test]
    fn decide_rejects_accepting_conflicted_item() {
        let item = mk_item(
            "chapter_summary",
            KnowledgeOp::Create,
            "chapter_summaries/vol1/ch1.json",
            None,
            KnowledgeAcceptPolicy::Manual,
            json!({"summary": "x"}),
        );
        let bundle = mk_bundle(vec![item.clone()]);
        let mut delta = KnowledgeDelta {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            knowledge_delta_id: "kdelta_decide".to_string(),
            status: KnowledgeDeltaStatus::Proposed,
            scope_ref: bundle.scope_ref.clone(),
            branch_id: None,
            source_session_id: bundle.source_session_id.clone(),
            source_review_id: bundle.source_review_id.clone(),
            generated_at: bundle.generated_at,
            targets: Vec::new(),
            changes: Vec::new(),
            evidence_refs: Vec::new(),
            conflicts: vec![KnowledgeConflict {
                conflict_type: "KNOWLEDGE_CANON_CONFLICT".to_string(),
                message: "x".to_string(),
                item_id: Some(item.item_id.clone()),
                target_ref: Some("chapter_summaries/vol1/ch1.json".to_string()),
            }],
            accepted_item_ids: None,
            rejected_item_ids: None,
            applied_at: None,
            rollback: None,
        };

        let decision = KnowledgeDecisionInput {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            bundle_id: bundle.bundle_id.clone(),
            delta_id: delta.knowledge_delta_id.clone(),
            accepted_item_ids: vec![item.item_id.clone()],
            rejected_item_ids: Vec::new(),
        };

        let res = apply_decision_to_delta(&bundle, delta.clone(), &decision);
        assert!(res.is_err());

        // Rejecting should be allowed and clears item conflict.
        decision_reject(&bundle, &mut delta, &item.item_id);
    }

    fn decision_reject(bundle: &KnowledgeProposalBundle, delta: &mut KnowledgeDelta, item_id: &str) {
        let decision = KnowledgeDecisionInput {
            schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
            bundle_id: bundle.bundle_id.clone(),
            delta_id: delta.knowledge_delta_id.clone(),
            accepted_item_ids: Vec::new(),
            rejected_item_ids: vec![item_id.to_string()],
        };
        let updated = apply_decision_to_delta(bundle, delta.clone(), &decision).unwrap();
        assert!(updated
            .conflicts
            .iter()
            .all(|c| c.item_id.as_deref() != Some(item_id)));
    }
}
