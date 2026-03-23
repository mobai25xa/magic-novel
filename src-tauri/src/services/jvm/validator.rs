use std::collections::HashSet;

use crate::models::jvm::{Diagnostic, DiagnosticLevel};

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
