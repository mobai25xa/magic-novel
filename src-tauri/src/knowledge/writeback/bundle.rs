mod closeout;

use std::collections::HashSet;
use std::path::Path;

use crate::knowledge::types::{KnowledgeDelta, KnowledgeOp, KnowledgeProposalBundle};
use crate::models::AppError;

use super::path::{ensure_safe_relative_path, normalize_path};
use super::storage::{read_stored_object, stored_object_path};

pub(super) fn generate_proposal_bundle_after_closeout(
    project_path: &Path,
    mission_id: &str,
    scope_ref: String,
    write_paths: Vec<String>,
    source_session_id: String,
    source_review_id: Option<String>,
) -> Result<KnowledgeProposalBundle, AppError> {
    closeout::generate_proposal_bundle_after_closeout(
        project_path,
        mission_id,
        scope_ref,
        write_paths,
        source_session_id,
        source_review_id,
    )
}

pub fn proposal_kinds(bundle: &KnowledgeProposalBundle) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for item in &bundle.proposal_items {
        let kind = item.kind.trim();
        if kind.is_empty() {
            continue;
        }
        if seen.insert(kind.to_string()) {
            out.push(kind.to_string());
        }
    }
    out
}

pub fn accepted_target_refs(bundle: &KnowledgeProposalBundle, delta: &KnowledgeDelta) -> Vec<String> {
    let accepted = delta
        .accepted_item_ids
        .as_ref()
        .cloned()
        .unwrap_or_default();
    if accepted.is_empty() {
        return Vec::new();
    }

    let accepted_set: HashSet<String> = accepted.into_iter().collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for item in &bundle.proposal_items {
        if !accepted_set.contains(&item.item_id) {
            continue;
        }
        let Some(target_ref) = item.target_ref.as_ref().map(|s| normalize_path(s)) else {
            continue;
        };
        if target_ref.is_empty() {
            continue;
        }
        if seen.insert(target_ref.clone()) {
            out.push(target_ref);
        }
    }
    out
}

pub fn repropose_bundle_refresh_target_revisions(
    project_path: &Path,
    bundle: &KnowledgeProposalBundle,
) -> Result<KnowledgeProposalBundle, AppError> {
    let mut out = bundle.clone();
    out.bundle_id = format!("kbundle_{}", uuid::Uuid::new_v4());
    out.generated_at = chrono::Utc::now().timestamp_millis();

    for item in &mut out.proposal_items {
        let Some(tr) = item
            .target_ref
            .as_ref()
            .map(|s| normalize_path(s))
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        item.target_ref = Some(tr.clone());

        if !matches!(item.op, KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore) {
            continue;
        }
        if ensure_safe_relative_path(&tr).is_err() {
            continue;
        }
        let p = stored_object_path(project_path, &tr);
        if let Ok(Some(obj)) = read_stored_object(&p) {
            item.target_revision = Some(obj.revision);
        }
    }

    Ok(out)
}
