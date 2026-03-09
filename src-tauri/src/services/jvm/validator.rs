use std::collections::HashSet;

use crate::models::jvm::{Diagnostic, DiagnosticLevel, PatchOp};

pub fn validate_patch_ops(patch_ops: &[PatchOp]) -> Vec<Diagnostic> {
    let mut diags = validate_patch_structure(patch_ops);
    diags.extend(validate_duplicate_targets(patch_ops));
    diags
}

fn validate_patch_structure(patch_ops: &[PatchOp]) -> Vec<Diagnostic> {
    let mut diags: Vec<Diagnostic> = vec![];

    for op in patch_ops {
        match op {
            PatchOp::InsertBlocks { blocks, .. } => {
                if blocks.is_empty() {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_SCHEMA_INVALID".to_string(),
                        message: "insert_blocks.blocks must not be empty".to_string(),
                        block_id: None,
                        suggestion: None,
                    });
                }
            }
            PatchOp::UpdateBlock {
                block_id, after, ..
            } => {
                if block_id.trim().is_empty() {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_SCHEMA_INVALID".to_string(),
                        message: "update_block.block_id must not be empty".to_string(),
                        block_id: None,
                        suggestion: None,
                    });
                }
                if after.get("type").and_then(|v| v.as_str()).is_none() {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_SCHEMA_INVALID".to_string(),
                        message: "update_block.after must be a TipTap node with type".to_string(),
                        block_id: Some(block_id.clone()),
                        suggestion: None,
                    });
                }
            }
            PatchOp::DeleteBlocks { block_ids } => {
                if block_ids.is_empty() {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_SCHEMA_INVALID".to_string(),
                        message: "delete_blocks.block_ids must not be empty".to_string(),
                        block_id: None,
                        suggestion: None,
                    });
                }
            }
            PatchOp::MoveBlock { block_id, .. } => {
                if block_id.trim().is_empty() {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_SCHEMA_INVALID".to_string(),
                        message: "move_block.block_id must not be empty".to_string(),
                        block_id: None,
                        suggestion: None,
                    });
                }
            }
        }
    }

    diags
}

fn validate_duplicate_targets(patch_ops: &[PatchOp]) -> Vec<Diagnostic> {
    let mut diags: Vec<Diagnostic> = vec![];
    let mut touched: HashSet<String> = HashSet::new();

    for op in patch_ops {
        match op {
            PatchOp::UpdateBlock { block_id, .. } => {
                if !touched.insert(block_id.clone()) {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Warn,
                        code: "E_JVM_VALIDATION_FAIL".to_string(),
                        message: format!("block_id touched multiple times: {block_id}"),
                        block_id: Some(block_id.clone()),
                        suggestion: Some("dedupe patch ops".to_string()),
                    });
                }
            }
            PatchOp::DeleteBlocks { block_ids } => {
                for id in block_ids {
                    if !touched.insert(id.clone()) {
                        diags.push(Diagnostic {
                            level: DiagnosticLevel::Warn,
                            code: "E_JVM_VALIDATION_FAIL".to_string(),
                            message: format!("block_id touched multiple times: {id}"),
                            block_id: Some(id.clone()),
                            suggestion: Some("dedupe patch ops".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    diags
}

pub fn validate_markdown_blocks(blocks_count: usize) -> Vec<Diagnostic> {
    let mut diags = vec![];
    if blocks_count == 0 {
        diags.push(Diagnostic {
            level: DiagnosticLevel::Warn,
            code: "E_JVM_VALIDATION_FAIL".to_string(),
            message: "markdown produced no blocks".to_string(),
            block_id: None,
            suggestion: Some("ensure markdown contains paragraphs/headings".to_string()),
        });
    }
    diags
}

/// Validate a TipTap document's structural integrity before committing.
/// Returns diagnostics for: missing doc type, non-array content,
/// blocks without IDs, duplicate IDs, and invalid block types.
pub fn validate_doc_integrity(doc: &serde_json::Value) -> Vec<Diagnostic> {
    let mut diags = vec![];

    if doc.get("type").and_then(|v| v.as_str()) != Some("doc") {
        diags.push(Diagnostic {
            level: DiagnosticLevel::Error,
            code: "E_JVM_DOC_INVALID".to_string(),
            message: "document root must have type: \"doc\"".to_string(),
            block_id: None,
            suggestion: None,
        });
        return diags;
    }

    let Some(content) = doc.get("content").and_then(|v| v.as_array()) else {
        diags.push(Diagnostic {
            level: DiagnosticLevel::Error,
            code: "E_JVM_DOC_INVALID".to_string(),
            message: "document must have a content array".to_string(),
            block_id: None,
            suggestion: None,
        });
        return diags;
    };

    let allowed_types = ["heading", "paragraph", "blockquote", "horizontalRule"];
    let mut seen_ids: HashSet<String> = HashSet::new();

    for (idx, node) in content.iter().enumerate() {
        let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if !allowed_types.contains(&node_type) {
            diags.push(Diagnostic {
                level: DiagnosticLevel::Warn,
                code: "E_JVM_DOC_UNKNOWN_BLOCK".to_string(),
                message: format!("block[{idx}] has unknown type: \"{node_type}\""),
                block_id: None,
                suggestion: None,
            });
        }

        if matches!(node_type, "heading" | "paragraph" | "blockquote") {
            let id = node
                .get("attrs")
                .and_then(|a| a.get("id"))
                .and_then(|v| v.as_str());

            match id {
                None | Some("") => {
                    diags.push(Diagnostic {
                        level: DiagnosticLevel::Error,
                        code: "E_JVM_DOC_MISSING_ID".to_string(),
                        message: format!("block[{idx}] (type={node_type}) is missing attrs.id"),
                        block_id: None,
                        suggestion: Some("all content blocks must have a unique id".to_string()),
                    });
                }
                Some(id_str) => {
                    if !seen_ids.insert(id_str.to_string()) {
                        diags.push(Diagnostic {
                            level: DiagnosticLevel::Error,
                            code: "E_JVM_DOC_DUPLICATE_ID".to_string(),
                            message: format!("block[{idx}] has duplicate id: \"{id_str}\""),
                            block_id: Some(id_str.to_string()),
                            suggestion: Some(
                                "each block must have a globally unique id".to_string(),
                            ),
                        });
                    }
                }
            }
        }
    }

    diags
}
