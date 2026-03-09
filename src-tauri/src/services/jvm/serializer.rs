use crate::models::jvm::ExportResult;
use crate::models::{AppError, Chapter};

pub fn export_chapter_to_markdown(
    chapter: &Chapter,
    revision: i64,
    json_hash: String,
    include_block_hints: bool,
) -> Result<ExportResult, AppError> {
    let markdown = serialize_doc_to_markdown(&chapter.content, include_block_hints);

    Ok(ExportResult {
        chapter_id: chapter.id.clone(),
        revision,
        json_hash,
        markdown,
    })
}

fn serialize_doc_to_markdown(doc: &serde_json::Value, include_block_hints: bool) -> String {
    let mut out: Vec<String> = vec![];

    let blocks = doc
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for node in blocks {
        if let Some(block_md) = serialize_block(&node, include_block_hints) {
            if !block_md.trim().is_empty() {
                out.push(block_md);
            }
        }
    }

    out.join("\n\n")
}

fn serialize_block(node: &serde_json::Value, include_block_hints: bool) -> Option<String> {
    let node_type = node.get("type")?.as_str()?.to_string();
    let id = node
        .get("attrs")
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut md = match node_type.as_str() {
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_i64())
                .unwrap_or(1)
                .clamp(1, 6);

            let prefix = "#".repeat(level as usize);
            let text = extract_inline_text(node.get("content"));
            format!("{prefix} {text}").trim().to_string()
        }
        "paragraph" => extract_inline_text(node.get("content")).trim().to_string(),
        "blockquote" => {
            // For simplicity, we serialize the inner text lines with '> '.
            let inner = extract_inline_text(node.get("content"));
            inner
                .lines()
                .map(|l| format!("> {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "horizontalRule" => "---".to_string(),
        _ => {
            // Unknown block types are ignored to keep markdown whitelist stable.
            return None;
        }
    };

    if include_block_hints {
        if let Some(id) = id {
            let hint = format!("<!-- @block:id={id} type={node_type} -->");
            if md.is_empty() {
                md = hint;
            } else {
                md = format!("{hint}\n{md}");
            }
        }
    }

    Some(md)
}

fn extract_inline_text(content: Option<&serde_json::Value>) -> String {
    fn walk(node: &serde_json::Value, out: &mut String) {
        if let Some(t) = node.get("type").and_then(|v| v.as_str()) {
            if t == "text" {
                if let Some(s) = node.get("text").and_then(|v| v.as_str()) {
                    out.push_str(s);
                }
                return;
            }
            if t == "hardBreak" {
                out.push('\n');
                return;
            }
        }

        if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
            for child in arr {
                walk(child, out);
            }
        }
    }

    let mut out = String::new();
    if let Some(v) = content {
        if let Some(arr) = v.as_array() {
            for child in arr {
                walk(child, &mut out);
            }
        } else {
            walk(v, &mut out);
        }
    }
    out
}
