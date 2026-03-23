use std::path::PathBuf;

use crate::models::jvm::{Actor, CommitResult, PatchOp};
use crate::models::{AppError, Chapter};
use crate::services::{
    count_text, read_json, versioning_port::VcActor, VcCommitInput, VcCommitPort, VersioningService,
};

use super::parser::{app_err_jvm, ensure_doc_block_ids};
use super::validator::validate_doc_integrity;

#[cfg(test)]
pub fn apply_patch_ops_to_doc(
    base_doc: &serde_json::Value,
    patch_ops: &[PatchOp],
) -> Result<serde_json::Value, AppError> {
    // Minimal patch application: only handles insert/update/delete in the doc.content array.
    // Move is implemented as best-effort reorder when possible.
    let mut doc = base_doc.clone();
    if doc.get("type").and_then(|v| v.as_str()) != Some("doc") {
        doc["type"] = serde_json::Value::String("doc".to_string());
    }
    if !doc.get("content").and_then(|v| v.as_array()).is_some() {
        doc["content"] = serde_json::Value::Array(vec![]);
    }

    for op in patch_ops {
        match op {
            PatchOp::DeleteBlocks { block_ids } => {
                delete_blocks(&mut doc, block_ids);
            }
            PatchOp::UpdateBlock {
                block_id, after, ..
            } => {
                update_block(&mut doc, block_id, after.clone());
            }
            PatchOp::InsertBlocks {
                after_block_id,
                blocks,
            } => {
                insert_blocks(&mut doc, after_block_id.clone(), blocks.clone());
            }
            PatchOp::MoveBlock {
                block_id,
                after_block_id,
            } => {
                move_block(&mut doc, block_id, after_block_id.clone());
            }
        }
    }

    // Defense: ensure every block has an ID after patching.
    ensure_doc_block_ids(&mut doc);

    Ok(doc)
}

#[cfg(test)]
fn delete_blocks(doc: &mut serde_json::Value, block_ids: &[String]) {
    let Some(arr) = doc.get_mut("content").and_then(|v| v.as_array_mut()) else {
        return;
    };

    arr.retain(|node| {
        let id = node
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        match id {
            Some(id) => !block_ids.contains(&id.to_string()),
            None => true,
        }
    });
}

#[cfg(test)]
fn update_block(doc: &mut serde_json::Value, block_id: &str, after: serde_json::Value) {
    let Some(arr) = doc.get_mut("content").and_then(|v| v.as_array_mut()) else {
        return;
    };

    for node in arr.iter_mut() {
        let id = node
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        if id == Some(block_id) {
            // Ensure id is preserved even if caller forgot.
            let mut after = after;
            if !after.get("attrs").and_then(|a| a.get("id")).is_some() {
                if after.get("attrs").is_none() {
                    after["attrs"] = serde_json::json!({});
                }
                after["attrs"]["id"] = serde_json::Value::String(block_id.to_string());
            }
            *node = after;
            break;
        }
    }
}

#[cfg(test)]
fn insert_blocks(
    doc: &mut serde_json::Value,
    after_block_id: Option<String>,
    blocks: Vec<serde_json::Value>,
) {
    let Some(arr) = doc.get_mut("content").and_then(|v| v.as_array_mut()) else {
        return;
    };

    let insert_idx = resolve_insert_index(arr, after_block_id.as_deref());

    for (offset, block) in blocks.into_iter().enumerate() {
        arr.insert(insert_idx + offset, block);
    }
}

#[cfg(test)]
fn resolve_insert_index(arr: &[serde_json::Value], after_block_id: Option<&str>) -> usize {
    match after_block_id {
        None => 0,
        Some(after_id) => arr
            .iter()
            .enumerate()
            .find(|(_, node)| {
                node.get("attrs")
                    .and_then(|a| a.get("id"))
                    .and_then(|v| v.as_str())
                    == Some(after_id)
            })
            .map(|(idx, _)| idx + 1)
            .unwrap_or(arr.len()),
    }
}

#[cfg(test)]
fn move_block(doc: &mut serde_json::Value, block_id: &str, after_block_id: Option<String>) {
    let Some(arr) = doc.get_mut("content").and_then(|v| v.as_array_mut()) else {
        return;
    };

    let mut from_idx: Option<usize> = None;
    for (idx, node) in arr.iter().enumerate() {
        let id = node
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        if id == Some(block_id) {
            from_idx = Some(idx);
            break;
        }
    }
    let Some(from_idx) = from_idx else {
        return;
    };

    let node = arr.remove(from_idx);

    let mut insert_idx = arr.len();
    if let Some(after_id) = after_block_id {
        for (idx, n) in arr.iter().enumerate() {
            let id = n
                .get("attrs")
                .and_then(|a| a.get("id"))
                .and_then(|v| v.as_str());
            if id == Some(after_id.as_str()) {
                insert_idx = idx + 1;
                break;
            }
        }
    }

    arr.insert(insert_idx, node);
}

/// Commit a fully-built TipTap document, bypassing the patch-apply pipeline.
/// `patch_ops_for_audit` is stored in the version-control log for diffing/auditing
/// but is NOT used to derive the document — `new_doc` is written as-is.
pub fn commit_full_document(
    project_path: &str,
    chapter_path: &str,
    expected_revision: i64,
    call_id: &str,
    actor: Actor,
    mut new_doc: serde_json::Value,
    patch_ops_for_audit: Vec<PatchOp>,
) -> Result<CommitResult, AppError> {
    validate_commit_input(call_id)?;

    let full_path = PathBuf::from(project_path)
        .join("manuscripts")
        .join(chapter_path);

    if !full_path.exists() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "chapter_path not found".to_string(),
            false,
            None,
        ));
    }

    let chapter: Chapter = read_json(&full_path)?;

    let vc = VersioningService::new();
    let entity_id = format!("chapter:{chapter_path}");
    let head = vc.get_current_head(project_path, &entity_id)?;
    ensure_revision_match(head.revision, expected_revision)?;

    // Defense: ensure every block has an ID before committing.
    ensure_doc_block_ids(&mut new_doc);

    let integrity_diags = validate_doc_integrity(&new_doc);
    if integrity_diags
        .iter()
        .any(|d| d.level == crate::models::DiagnosticLevel::Error)
    {
        let messages: Vec<String> = integrity_diags
            .iter()
            .filter(|d| d.level == crate::models::DiagnosticLevel::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(app_err_jvm(
            "E_JVM_DOC_INTEGRITY_FAIL",
            format!("document failed integrity check: {}", messages.join("; ")),
            false,
            None,
        ));
    }

    let after_json = build_after_chapter_json(chapter, new_doc)?;

    let commit_out = vc.commit_with_occ(VcCommitInput {
        project_path: project_path.to_string(),
        entity_id,
        expected_revision,
        call_id: call_id.to_string(),
        actor: map_actor(actor),
        before_hash: head.json_hash.clone(),
        after_json,
        patch_ops: patch_ops_for_audit
            .iter()
            .map(|op| serde_json::to_value(op).unwrap_or(serde_json::Value::Null))
            .collect(),
    })?;

    Ok(CommitResult {
        ok: commit_out.ok,
        revision_before: commit_out.revision_before,
        revision_after: commit_out.revision_after,
        json_hash_after: commit_out.after_hash,
        tx_id: commit_out.tx_id,
    })
}

fn validate_commit_input(call_id: &str) -> Result<(), AppError> {
    if call_id.trim().is_empty() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "call_id is required".to_string(),
            false,
            None,
        ));
    }
    Ok(())
}

fn ensure_revision_match(current_revision: i64, expected_revision: i64) -> Result<(), AppError> {
    if current_revision == expected_revision {
        return Ok(());
    }

    Err(app_err_jvm(
        "E_JVM_CONFLICT_REVISION",
        format!(
            "expected_revision {} does not match current {}",
            expected_revision, current_revision
        ),
        true,
        Some(serde_json::json!({
            "expected_revision": expected_revision,
            "current_revision": current_revision,
        })),
    ))
}

fn build_after_chapter_json(
    mut chapter: Chapter,
    after_doc: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    chapter.content = after_doc;
    chapter.counts = count_text(&chapter.content);
    chapter.updated_at = chrono::Utc::now().timestamp_millis();
    serde_json::to_value(chapter)
        .map_err(|e| app_err_jvm("E_JVM_IO_FAIL", e.to_string(), true, None))
}

fn map_actor(actor: Actor) -> VcActor {
    match actor {
        Actor::Agent => VcActor::Agent,
        Actor::User => VcActor::User,
        Actor::System => VcActor::System,
    }
}
