use serde_json::{json, Value};

use crate::agent_tools::contracts::ToolResult;

/// Truncate a string to at most `max_chars` characters, respecting char boundaries (CJK-safe).
pub(super) fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    s.char_indices()
        .nth(max_chars)
        .map(|(byte_idx, _)| s[..byte_idx].to_string())
        .unwrap_or_else(|| s.to_string())
}

pub(super) fn build_result_data_preview(
    tool_name: &str,
    result: &ToolResult<serde_json::Value>,
) -> Value {
    let Some(data) = result.data.as_ref() else {
        return Value::Null;
    };

    match tool_name {
        "todowrite" => {
            let todo_state = data.get("todo_state").cloned().unwrap_or(Value::Null);
            json!({ "todo_state": todo_state })
        }
        "read" => json!({
            "path": data.get("path").cloned(),
            "kind": data.get("kind").cloned(),
            "revision": data.get("revision").cloned(),
            "hash": data.get("hash").cloned(),
            "content_chars": data
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().count())
                .unwrap_or(0),
            "has_content_json": data.get("content_json").is_some(),
        }),
        "edit" => json!({
            "mode": data.get("mode").cloned(),
            "accepted": data.get("accepted").cloned(),
            "path": data.get("path").cloned(),
            "revision_before": data.get("revision_before").cloned(),
            "revision_after": data.get("revision_after").cloned(),
            "diagnostics_count": data
                .get("diagnostics")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0),
            "diff_summary_count": data
                .get("diff_summary")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0),
            "tx_id": data.get("tx_id").cloned(),
        }),
        "create" => json!({
            "created_kind": data.get("created_kind").cloned(),
            "path": data.get("path").cloned(),
            "id": data.get("id").cloned(),
            "revision_after": data.get("revision_after").cloned(),
            "created_at": data.get("created_at").cloned(),
        }),
        "delete" => json!({
            "mode": data.get("mode").cloned(),
            "kind": data.get("kind").cloned(),
            "path": data.get("path").cloned(),
            "impact": data.get("impact").cloned(),
        }),
        "move" => json!({
            "mode": data.get("mode").cloned(),
            "accepted": data.get("accepted").cloned(),
            "chapter_path": data.get("chapter_path").cloned(),
            "target_volume_path": data.get("target_volume_path").cloned(),
            "target_index": data.get("target_index").cloned(),
            "new_chapter_path": data.get("new_chapter_path").cloned(),
        }),
        "ls" => {
            let items_count = data
                .get("items")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "cwd": data.get("cwd").cloned(),
                "items_count": items_count,
            })
        }
        "grep" => {
            let hits_count = data
                .get("hits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "hits_count": hits_count,
                "semantic_notice": data.get("semantic_notice").cloned(),
            })
        }
        "outline" => json!({
            "outline_chars": data
                .get("outline")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().count())
                .unwrap_or(0),
        }),
        "character_sheet" | "search_knowledge" => json!({
            "result_chars": data
                .get("result")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().count())
                .unwrap_or(0),
        }),
        _ => Value::Null,
    }
}

pub(super) fn build_result_error(result: &ToolResult<serde_json::Value>) -> Value {
    match result.error.as_ref() {
        None => Value::Null,
        Some(error) => json!({
            "code": error.code,
            "message": error.message,
            "retryable": error.retryable,
            "fault_domain": error.fault_domain,
            "details": error.details,
        }),
    }
}

pub(super) fn build_result_refs(tool_name: &str, result: &ToolResult<serde_json::Value>) -> Value {
    let Some(data) = result.data.as_ref() else {
        return Value::Null;
    };

    let mut refs = serde_json::Map::new();

    for key in [
        "path",
        "chapter_path",
        "volume_path",
        "chapter_id",
        "tx_id",
        "hash",
        "hash_after",
        "json_hash",
        "json_hash_after",
        "revision",
        "revision_before",
        "revision_after",
        "target",
        "mode",
        "accepted",
        "snapshot_id",
    ] {
        if let Some(value) = data.get(key).cloned() {
            refs.insert(key.to_string(), value);
        }
    }

    if tool_name == "create" {
        if let Some(value) = data.get("created_kind").cloned() {
            refs.insert("created_kind".to_string(), value);
        }
    }

    if tool_name == "delete" {
        if let Some(value) = data.get("kind").cloned() {
            refs.insert("deleted_kind".to_string(), value);
        }
    }

    if refs.is_empty() {
        Value::Null
    } else {
        Value::Object(refs)
    }
}
