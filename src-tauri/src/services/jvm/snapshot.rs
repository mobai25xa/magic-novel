use crate::agent_tools::contracts::{ChapterSnapshot, SnapshotBlock};

pub fn build_snapshot_id(path: &str, revision: i64, json_hash: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    format!("snap:{}:{}:{}", normalized, revision.max(0), json_hash)
}

pub fn snapshot_id_matches(path: &str, revision: i64, json_hash: &str, snapshot_id: &str) -> bool {
    build_snapshot_id(path, revision, json_hash) == snapshot_id
}

pub fn extract_doc_blocks(doc: &serde_json::Value) -> Vec<serde_json::Value> {
    doc.get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn build_doc_with_blocks(blocks: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": blocks,
    })
}

pub fn build_chapter_snapshot(
    path: &str,
    revision: i64,
    json_hash: &str,
    doc: &serde_json::Value,
) -> ChapterSnapshot {
    let blocks = extract_doc_blocks(doc)
        .into_iter()
        .enumerate()
        .filter_map(|(order, node)| {
            node_to_snapshot_block(&node, order as u32).map(|mut block| {
                block.block_id = block_id_for_node(&node, order as u32);
                block
            })
        })
        .collect::<Vec<_>>();

    ChapterSnapshot {
        snapshot_id: build_snapshot_id(path, revision, json_hash),
        block_count: blocks.len() as u32,
        blocks,
    }
}

pub fn block_id_for_node(node: &serde_json::Value, order: u32) -> String {
    node.get("attrs")
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            let node_type = node
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("__idx_{}_{}", order, node_type)
        })
}

pub fn node_to_markdown(node: &serde_json::Value) -> Option<String> {
    let node_type = node.get("type")?.as_str()?;
    let markdown = match node_type {
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_i64())
                .unwrap_or(1)
                .clamp(1, 6) as usize;
            let text = extract_inline_text(node.get("content"));
            format!("{} {}", "#".repeat(level), text).trim().to_string()
        }
        "paragraph" => extract_inline_text(node.get("content")).trim().to_string(),
        "blockquote" => {
            let inner = extract_inline_text(node.get("content"));
            inner
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "horizontalRule" => "---".to_string(),
        _ => return None,
    };

    Some(markdown)
}

fn node_to_snapshot_block(node: &serde_json::Value, order: u32) -> Option<SnapshotBlock> {
    let markdown = node_to_markdown(node)?;
    let block_type = match node.get("type").and_then(|v| v.as_str()) {
        Some("horizontalRule") => "horizontal_rule".to_string(),
        Some(other) => other.to_string(),
        None => return None,
    };

    Some(SnapshotBlock {
        block_id: String::new(),
        block_type,
        order,
        markdown,
    })
}

fn extract_inline_text(content: Option<&serde_json::Value>) -> String {
    fn walk(node: &serde_json::Value, out: &mut String) {
        if node.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(text) = node.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
            }
            return;
        }

        if node.get("type").and_then(|v| v.as_str()) == Some("hardBreak") {
            out.push('\n');
            return;
        }

        if let Some(children) = node.get("content").and_then(|v| v.as_array()) {
            for child in children {
                walk(child, out);
            }
        }
    }

    let mut out = String::new();
    if let Some(value) = content {
        if let Some(children) = value.as_array() {
            for child in children {
                walk(child, &mut out);
            }
        } else {
            walk(value, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{build_chapter_snapshot, build_snapshot_id, snapshot_id_matches};

    #[test]
    fn snapshot_id_is_stable() {
        let id = build_snapshot_id("vol_1/ch_1.json", 7, "sha256:abc");
        assert_eq!(id, "snap:vol_1/ch_1.json:7:sha256:abc");
        assert!(snapshot_id_matches("vol_1/ch_1.json", 7, "sha256:abc", &id));
        assert!(!snapshot_id_matches(
            "vol_1/ch_1.json",
            8,
            "sha256:abc",
            &id
        ));
    }

    #[test]
    fn builds_snapshot_blocks_without_block_hints() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {
                    "type": "heading",
                    "attrs": {"id": "h1", "level": 1},
                    "content": [{"type": "text", "text": "标题"}]
                },
                {
                    "type": "paragraph",
                    "attrs": {"id": "p1"},
                    "content": [{"type": "text", "text": "正文"}]
                }
            ]
        });

        let snapshot = build_chapter_snapshot("vol_1/ch_1.json", 3, "sha256:x", &doc);
        assert_eq!(snapshot.block_count, 2);
        assert_eq!(snapshot.blocks[0].block_id, "h1");
        assert_eq!(snapshot.blocks[0].markdown, "# 标题");
        assert!(!snapshot.blocks[0].markdown.contains("@block:id"));
        assert_eq!(snapshot.blocks[1].block_id, "p1");
        assert_eq!(snapshot.blocks[1].markdown, "正文");
    }
}
