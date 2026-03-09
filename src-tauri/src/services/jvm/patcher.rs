use std::collections::HashMap;

use crate::models::jvm::{Diagnostic, PatchOp};
use crate::models::DiagnosticLevel;

use super::parser::{to_tiptap_block, MdBlock};

pub fn generate_patch_ops(
    base_doc: &serde_json::Value,
    md_blocks: &[MdBlock],
) -> (Vec<PatchOp>, Vec<Diagnostic>, Vec<String>) {
    let base_blocks = extract_base_blocks(base_doc);
    let base_ids = collect_base_ids(&base_blocks);
    let id_to_base_index = build_id_to_base_index(&base_ids);

    let diagnostics = validate_alignment(md_blocks, &id_to_base_index);
    if diagnostics
        .iter()
        .any(|d| d.level == DiagnosticLevel::Error)
    {
        return (
            vec![],
            diagnostics,
            vec!["align failed: ambiguous mapping".to_string()],
        );
    }

    let (desired_ids, desired_blocks) = build_desired_blocks(md_blocks);
    let mut ops = build_delete_ops(&base_ids, &desired_ids);
    ops.extend(build_update_insert_ops(
        &base_blocks,
        &base_ids,
        &id_to_base_index,
        &desired_ids,
        &desired_blocks,
    ));

    let mut all_diagnostics = diagnostics;
    if let Some(warn) = build_order_change_warning(&base_ids, &desired_ids) {
        all_diagnostics.push(warn);
    }

    let diff_summary = summarize(&ops);
    (ops, all_diagnostics, diff_summary)
}

fn collect_base_ids(base_blocks: &[serde_json::Value]) -> Vec<Option<String>> {
    base_blocks
        .iter()
        .map(|block| {
            block
                .get("attrs")
                .and_then(|attrs| attrs.get("id"))
                .and_then(|id| id.as_str())
                .map(|id| id.to_string())
        })
        .collect()
}

fn build_id_to_base_index(base_ids: &[Option<String>]) -> HashMap<String, usize> {
    let mut id_to_base_index: HashMap<String, usize> = HashMap::new();
    for (idx, id_opt) in base_ids.iter().enumerate() {
        if let Some(id) = id_opt {
            id_to_base_index.insert(id.clone(), idx);
        }
    }
    id_to_base_index
}

fn validate_alignment(
    md_blocks: &[MdBlock],
    id_to_base_index: &HashMap<String, usize>,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];
    let mut used_base: HashMap<usize, usize> = HashMap::new();

    for (idx, block) in md_blocks.iter().enumerate() {
        if let Some(id) = &block.block_id {
            if let Some(base_idx) = id_to_base_index.get(id).copied() {
                if used_base.contains_key(&base_idx) {
                    diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_ALIGN_AMBIGUOUS".to_string(),
                        message: format!(
                            "multiple markdown blocks refer to same base block id: {id}"
                        ),
                        block_id: Some(id.clone()),
                        suggestion: Some("ensure block hints are unique".to_string()),
                    });
                } else {
                    used_base.insert(base_idx, idx);
                }
            }
        }
    }

    diagnostics
}

fn build_desired_blocks(md_blocks: &[MdBlock]) -> (Vec<Option<String>>, Vec<serde_json::Value>) {
    let mut desired_ids: Vec<Option<String>> = vec![];
    let mut desired_blocks: Vec<serde_json::Value> = vec![];

    for block in md_blocks {
        let tiptap = to_tiptap_block(block);
        let id = tiptap
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        desired_ids.push(id);
        desired_blocks.push(tiptap);
    }

    (desired_ids, desired_blocks)
}

fn build_delete_ops(base_ids: &[Option<String>], desired_ids: &[Option<String>]) -> Vec<PatchOp> {
    let desired_id_set: std::collections::HashSet<String> =
        desired_ids.iter().filter_map(|id| id.clone()).collect();

    let delete_ids: Vec<String> = base_ids
        .iter()
        .filter_map(|id| id.clone())
        .filter(|id| !desired_id_set.contains(id))
        .collect();

    if delete_ids.is_empty() {
        vec![]
    } else {
        vec![PatchOp::DeleteBlocks {
            block_ids: delete_ids,
        }]
    }
}

fn build_update_insert_ops(
    base_blocks: &[serde_json::Value],
    base_ids: &[Option<String>],
    id_to_base_index: &HashMap<String, usize>,
    desired_ids: &[Option<String>],
    desired_blocks: &[serde_json::Value],
) -> Vec<PatchOp> {
    let base_id_set: std::collections::HashSet<String> =
        base_ids.iter().filter_map(|id| id.clone()).collect();

    let mut ops: Vec<PatchOp> = vec![];
    let mut last_existing_id: Option<String> = None;
    let mut pending_insert_after: Option<String> = None;
    let mut pending_insert_blocks: Vec<serde_json::Value> = vec![];

    for (idx, block) in desired_blocks.iter().enumerate() {
        let id_opt = desired_ids[idx].clone();
        if let Some(id) = id_opt.clone() {
            if base_id_set.contains(&id) {
                flush_pending_inserts(
                    &mut ops,
                    &mut pending_insert_after,
                    &mut pending_insert_blocks,
                );

                if let Some(base_idx) = id_to_base_index.get(&id).copied() {
                    let base_block = &base_blocks[base_idx];
                    if !json_blocks_equivalent(base_block, block) {
                        ops.push(PatchOp::UpdateBlock {
                            block_id: id.clone(),
                            before: None,
                            after: block.clone(),
                        });
                    }
                }
                last_existing_id = Some(id);
                continue;
            }
        }

        if pending_insert_blocks.is_empty() {
            pending_insert_after = last_existing_id.clone();
        }
        pending_insert_blocks.push(block.clone());
    }

    flush_pending_inserts(
        &mut ops,
        &mut pending_insert_after,
        &mut pending_insert_blocks,
    );
    ops
}

fn flush_pending_inserts(
    ops: &mut Vec<PatchOp>,
    pending_insert_after: &mut Option<String>,
    pending_insert_blocks: &mut Vec<serde_json::Value>,
) {
    if pending_insert_blocks.is_empty() {
        return;
    }

    ops.push(PatchOp::InsertBlocks {
        after_block_id: pending_insert_after.clone(),
        blocks: std::mem::take(pending_insert_blocks),
    });
    *pending_insert_after = None;
}

fn build_order_change_warning(
    base_ids: &[Option<String>],
    desired_ids: &[Option<String>],
) -> Option<Diagnostic> {
    let base_seq: Vec<String> = base_ids.iter().filter_map(|id| id.clone()).collect();
    let desired_seq: Vec<String> = desired_ids.iter().filter_map(|id| id.clone()).collect();

    if base_seq.is_empty() || desired_seq.is_empty() || base_seq == desired_seq {
        return None;
    }

    Some(Diagnostic {
        level: DiagnosticLevel::Warn,
        code: "E_JVM_VALIDATION_FAIL".to_string(),
        message: "block order changed; move ops not emitted (will apply as delete/insert)"
            .to_string(),
        block_id: None,
        suggestion: Some("consider patch-preferred mode with explicit move_block ops".to_string()),
    })
}

fn extract_base_blocks(doc: &serde_json::Value) -> Vec<serde_json::Value> {
    doc.get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn json_blocks_equivalent(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    // Compare type + text content + heading level; ignore attrs except id/level.
    let ta = a.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let tb = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if ta != tb {
        return false;
    }

    if ta == "heading" {
        let la = a
            .get("attrs")
            .and_then(|x| x.get("level"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1);
        let lb = b
            .get("attrs")
            .and_then(|x| x.get("level"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1);
        if la != lb {
            return false;
        }
    }

    let text_a = extract_text(a);
    let text_b = extract_text(b);
    text_a == text_b
}

fn extract_text(node: &serde_json::Value) -> String {
    let mut out = String::new();
    walk(node, &mut out);
    out
}

fn walk(node: &serde_json::Value, out: &mut String) {
    if node.get("type").and_then(|v| v.as_str()) == Some("text") {
        if let Some(s) = node.get("text").and_then(|v| v.as_str()) {
            out.push_str(s);
        }
        return;
    }
    if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
        for c in arr {
            walk(c, out);
        }
    }
}

fn summarize(ops: &[PatchOp]) -> Vec<String> {
    let mut inserts = 0;
    let mut updates = 0;
    let mut deletes = 0;
    let mut moves = 0;

    for op in ops {
        match op {
            PatchOp::InsertBlocks { blocks, .. } => inserts += blocks.len(),
            PatchOp::UpdateBlock { .. } => updates += 1,
            PatchOp::DeleteBlocks { block_ids } => deletes += block_ids.len(),
            PatchOp::MoveBlock { .. } => moves += 1,
        }
    }

    let mut out = vec![];
    if inserts > 0 {
        out.push(format!("insert_blocks: {inserts}"));
    }
    if updates > 0 {
        out.push(format!("update_block: {updates}"));
    }
    if deletes > 0 {
        out.push(format!("delete_blocks: {deletes}"));
    }
    if moves > 0 {
        out.push(format!("move_block: {moves}"));
    }
    if out.is_empty() {
        out.push("no changes".to_string());
    }
    out
}
