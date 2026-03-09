use crate::models::{AssetNode, AssetTree};

pub fn extract_tiptap_text(value: &serde_json::Value) -> String {
    let mut out = String::new();
    extract_tiptap_text_into(value, &mut out);
    out
}

fn extract_tiptap_text_into(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::String(s) => out.push_str(s),
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_tiptap_text_into(v, out);
            }
        }
        serde_json::Value::Object(obj) => {
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
                return;
            }

            if let Some(content) = obj.get("content") {
                extract_tiptap_text_into(content, out);
            }

            if should_insert_newline(obj.get("type").and_then(|v| v.as_str())) {
                out.push('\n');
            }
        }
        _ => {}
    }
}

fn should_insert_newline(node_type: Option<&str>) -> bool {
    matches!(
        node_type,
        Some("paragraph")
            | Some("heading")
            | Some("blockquote")
            | Some("listItem")
            | Some("codeBlock")
    )
}

pub fn extract_asset_text(asset: &AssetTree) -> String {
    let mut out = String::new();
    out.push_str(&asset.title);
    out.push('\n');
    extract_asset_node_text(&asset.root, &mut out);
    out
}

fn extract_asset_node_text(node: &AssetNode, out: &mut String) {
    if !node.title.trim().is_empty() {
        out.push_str(&node.title);
        out.push('\n');
    }

    if !node.content.trim().is_empty() {
        out.push_str(&node.content);
        out.push('\n');
    }

    for child in &node.children {
        extract_asset_node_text(child, out);
    }
}
