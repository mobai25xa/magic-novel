use std::collections::HashSet;

use crate::agent_tools::contracts::{DiffSummary, EditOp, SnapshotBlockInput};
use crate::models::jvm::Diagnostic;
use crate::models::{AppError, ErrorCode};

use super::parser::{ensure_doc_block_ids, parse_markdown_to_blocks, to_tiptap_block};
use super::snapshot::{block_id_for_node, build_doc_with_blocks, extract_doc_blocks};

#[derive(Debug)]
pub struct EditOpsApplyResult {
    pub doc: serde_json::Value,
    pub changed_block_ids: Vec<String>,
    pub diff_summary: Vec<DiffSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

struct EditApplyState {
    blocks: Vec<serde_json::Value>,
    changed_block_ids: Vec<String>,
    changed_seen: HashSet<String>,
    diff_summary: Vec<DiffSummary>,
}

impl EditApplyState {
    fn new(blocks: Vec<serde_json::Value>) -> Self {
        Self {
            blocks,
            changed_block_ids: Vec::new(),
            changed_seen: HashSet::new(),
            diff_summary: Vec::new(),
        }
    }
}

pub fn apply_edit_ops_to_doc(
    base_doc: &serde_json::Value,
    ops: &[EditOp],
) -> Result<EditOpsApplyResult, AppError> {
    if ops.is_empty() {
        return Err(edit_error(
            "E_EDIT_OPS_EMPTY",
            "ops must contain at least one operation",
            true,
            None,
        ));
    }

    let mut state = EditApplyState::new(extract_doc_blocks(base_doc));

    for op in ops {
        apply_single_op(&mut state, op)?;
    }

    let mut doc = build_doc_with_blocks(state.blocks);
    ensure_doc_block_ids(&mut doc);

    Ok(EditOpsApplyResult {
        doc,
        changed_block_ids: state.changed_block_ids,
        diff_summary: state.diff_summary,
        diagnostics: Vec::new(),
    })
}

fn apply_single_op(state: &mut EditApplyState, op: &EditOp) -> Result<(), AppError> {
    match op {
        EditOp::ReplaceBlock { block_id, markdown } => {
            apply_replace_block(state, block_id, markdown)
        }
        EditOp::DeleteBlock { block_id } => apply_delete_block(state, block_id),
        EditOp::InsertBefore {
            block_id,
            blocks: inserted,
        } => apply_insert_before(state, block_id, inserted),
        EditOp::InsertAfter {
            block_id,
            blocks: inserted,
        } => apply_insert_after(state, block_id, inserted),
        EditOp::AppendBlocks { blocks: appended } => apply_append_blocks(state, appended),
        EditOp::ReplaceRange {
            start_block_id,
            end_block_id,
            blocks: replacement,
        } => apply_replace_range(state, start_block_id, end_block_id, replacement),
    }
}

fn apply_replace_block(
    state: &mut EditApplyState,
    block_id: &str,
    markdown: &str,
) -> Result<(), AppError> {
    let idx = find_named_block_index(&state.blocks, block_id, "replace_block target")?;
    let mut replacement = parse_single_markdown_block(markdown)?;
    force_block_id(&mut replacement, block_id);
    state.blocks[idx] = replacement;

    push_changed_id(
        &mut state.changed_block_ids,
        &mut state.changed_seen,
        block_id,
    );
    state.diff_summary.push(DiffSummary {
        operation: "replace_block".to_string(),
        description: format!("replace {block_id}"),
    });
    Ok(())
}

fn apply_delete_block(state: &mut EditApplyState, block_id: &str) -> Result<(), AppError> {
    let idx = find_named_block_index(&state.blocks, block_id, "delete_block target")?;
    state.blocks.remove(idx);

    push_changed_id(
        &mut state.changed_block_ids,
        &mut state.changed_seen,
        block_id,
    );
    state.diff_summary.push(DiffSummary {
        operation: "delete_block".to_string(),
        description: format!("delete {block_id}"),
    });
    Ok(())
}

fn apply_insert_before(
    state: &mut EditApplyState,
    block_id: &str,
    inserted: &[SnapshotBlockInput],
) -> Result<(), AppError> {
    let idx = find_named_block_index(&state.blocks, block_id, "insert_before anchor")?;
    let mut nodes = parse_markdown_inputs(inserted)?;
    let inserted_ids = collect_explicit_block_ids(&nodes);
    for (offset, node) in nodes.drain(..).enumerate() {
        state.blocks.insert(idx + offset, node);
    }

    push_changed_id(
        &mut state.changed_block_ids,
        &mut state.changed_seen,
        block_id,
    );
    for id in inserted_ids {
        push_changed_id(&mut state.changed_block_ids, &mut state.changed_seen, &id);
    }
    state.diff_summary.push(DiffSummary {
        operation: "insert_before".to_string(),
        description: format!("insert before {block_id}"),
    });
    Ok(())
}

fn apply_insert_after(
    state: &mut EditApplyState,
    block_id: &str,
    inserted: &[SnapshotBlockInput],
) -> Result<(), AppError> {
    let idx = find_named_block_index(&state.blocks, block_id, "insert_after anchor")?;
    let mut nodes = parse_markdown_inputs(inserted)?;
    let inserted_ids = collect_explicit_block_ids(&nodes);
    for (offset, node) in nodes.drain(..).enumerate() {
        state.blocks.insert(idx + 1 + offset, node);
    }

    push_changed_id(
        &mut state.changed_block_ids,
        &mut state.changed_seen,
        block_id,
    );
    for id in inserted_ids {
        push_changed_id(&mut state.changed_block_ids, &mut state.changed_seen, &id);
    }
    state.diff_summary.push(DiffSummary {
        operation: "insert_after".to_string(),
        description: format!("insert after {block_id}"),
    });
    Ok(())
}

fn apply_append_blocks(
    state: &mut EditApplyState,
    appended: &[SnapshotBlockInput],
) -> Result<(), AppError> {
    let mut nodes = parse_markdown_inputs(appended)?;
    let inserted_ids = collect_explicit_block_ids(&nodes);
    state.blocks.extend(nodes.drain(..));

    for id in inserted_ids {
        push_changed_id(&mut state.changed_block_ids, &mut state.changed_seen, &id);
    }
    state.diff_summary.push(DiffSummary {
        operation: "append_blocks".to_string(),
        description: "append blocks".to_string(),
    });
    Ok(())
}

fn apply_replace_range(
    state: &mut EditApplyState,
    start_block_id: &str,
    end_block_id: &str,
    replacement: &[SnapshotBlockInput],
) -> Result<(), AppError> {
    let start_idx =
        find_named_block_index(&state.blocks, start_block_id, "replace_range start block")?;
    let end_idx = find_named_block_index(&state.blocks, end_block_id, "replace_range end block")?;

    if start_idx > end_idx {
        return Err(edit_error(
            "E_EDIT_RANGE_INVALID",
            format!(
                "replace_range invalid order: start {start_block_id} appears after end {end_block_id}"
            ),
            true,
            Some(serde_json::json!({
                "start_block_id": start_block_id,
                "end_block_id": end_block_id,
            })),
        ));
    }

    let removed_ids = (start_idx..=end_idx)
        .map(|idx| block_id_for_node(&state.blocks[idx], idx as u32))
        .collect::<Vec<_>>();
    let nodes = parse_markdown_inputs(replacement)?;
    let inserted_ids = collect_explicit_block_ids(&nodes);

    state.blocks.splice(start_idx..=end_idx, nodes.into_iter());

    for id in removed_ids {
        push_changed_id(&mut state.changed_block_ids, &mut state.changed_seen, &id);
    }
    for id in inserted_ids {
        push_changed_id(&mut state.changed_block_ids, &mut state.changed_seen, &id);
    }
    state.diff_summary.push(DiffSummary {
        operation: "replace_range".to_string(),
        description: format!("replace range {start_block_id}..{end_block_id}"),
    });
    Ok(())
}

fn find_named_block_index(
    blocks: &[serde_json::Value],
    block_id: &str,
    context: &str,
) -> Result<usize, AppError> {
    find_block_index(blocks, block_id).ok_or_else(|| {
        edit_error(
            "E_EDIT_BLOCK_NOT_FOUND",
            format!("{context} not found: {block_id}"),
            true,
            Some(serde_json::json!({ "block_id": block_id })),
        )
    })
}

fn push_changed_id(out: &mut Vec<String>, seen: &mut HashSet<String>, id: &str) {
    if seen.insert(id.to_string()) {
        out.push(id.to_string());
    }
}

fn find_block_index(blocks: &[serde_json::Value], block_id: &str) -> Option<usize> {
    blocks
        .iter()
        .enumerate()
        .find_map(|(idx, node)| (block_id_for_node(node, idx as u32) == block_id).then_some(idx))
}

fn parse_single_markdown_block(markdown: &str) -> Result<serde_json::Value, AppError> {
    let (md_blocks, _) = parse_markdown_to_blocks(markdown)?;
    if md_blocks.len() != 1 {
        return Err(edit_error(
            "E_EDIT_RANGE_INVALID",
            "replace_block markdown must produce exactly one block",
            true,
            Some(serde_json::json!({ "produced_blocks": md_blocks.len() })),
        ));
    }
    Ok(to_tiptap_block(&md_blocks[0]))
}

fn parse_markdown_inputs(
    blocks: &[SnapshotBlockInput],
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut out = Vec::new();
    for block in blocks {
        let (md_blocks, _) = parse_markdown_to_blocks(&block.markdown)?;
        if md_blocks.is_empty() {
            return Err(edit_error(
                "E_EDIT_RANGE_INVALID",
                "op markdown produced no blocks",
                true,
                None,
            ));
        }
        out.extend(md_blocks.iter().map(to_tiptap_block));
    }
    Ok(out)
}

fn collect_explicit_block_ids(nodes: &[serde_json::Value]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|node| {
            node.get("attrs")
                .and_then(|attrs| attrs.get("id"))
                .and_then(|id| id.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn force_block_id(node: &mut serde_json::Value, block_id: &str) {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(node_type, "heading" | "paragraph" | "blockquote") {
        return;
    }

    if node.get("attrs").is_none() {
        node["attrs"] = serde_json::json!({});
    }
    node["attrs"]["id"] = serde_json::Value::String(block_id.to_string());
}

fn edit_error(
    code: &str,
    message: impl Into<String>,
    recoverable: bool,
    details: Option<serde_json::Value>,
) -> AppError {
    AppError {
        code: ErrorCode::InvalidArgument,
        message: message.into(),
        details: Some(merge_details(code, details)),
        recoverable: Some(recoverable),
    }
}

fn merge_details(code: &str, details: Option<serde_json::Value>) -> serde_json::Value {
    let mut out = serde_json::json!({ "code": code });
    if let Some(extra) = details {
        match extra {
            serde_json::Value::Object(map) => {
                if let serde_json::Value::Object(out_map) = &mut out {
                    for (k, v) in map {
                        out_map.insert(k, v);
                    }
                }
            }
            other => out["extra"] = other,
        }
    }
    out
}

#[cfg(test)]
mod tests;
