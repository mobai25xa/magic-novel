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
        "workspace_map" => {
            let tree_count = data
                .get("tree")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "tree_count": tree_count,
                "summary": data.get("summary").cloned(),
                "truncated": data.get("truncated").cloned(),
                "next_cursor": data.get("next_cursor").cloned(),
            })
        }
        "context_read" => json!({
            "ref": data.get("ref").cloned(),
            "kind": data.get("kind").cloned(),
            "content_chars": data
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().count())
                .unwrap_or(0),
            "truncated": data.get("truncated").cloned(),
        }),
        "context_search" => {
            let hits_count = data
                .get("hits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "mode": data.get("mode").cloned(),
                "hits_count": hits_count,
                "degraded": data.get("degraded").cloned(),
                "degraded_reason": data.get("degraded_reason").cloned(),
            })
        }
        "knowledge_read" => {
            let items_count = data
                .get("items")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "items_count": items_count,
                "truncated": data.get("truncated").cloned(),
            })
        }
        "knowledge_write" => {
            let conflicts_count = data
                .get("conflicts")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "delta_id": data.get("delta_id").cloned(),
                "status": data.get("status").cloned(),
                "conflicts_count": conflicts_count,
                "next_action": data.get("next_action").cloned(),
            })
        }
        "draft_write" => {
            let diff_summary_count = data
                .get("diff_summary")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            let snippet_after_chars = data
                .get("snippet_after")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().count())
                .unwrap_or(0);
            json!({
                "accepted": data.get("accepted").cloned(),
                "mode": data.get("mode").cloned(),
                "diff_summary_count": diff_summary_count,
                "tx_id": data.get("tx_id").cloned(),
                "snippet_after_chars": snippet_after_chars,
            })
        }
        "structure_edit" => {
            let impact_summary_count = data
                .get("impact_summary")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "accepted": data.get("accepted").cloned(),
                "mode": data.get("mode").cloned(),
                "impact_summary_count": impact_summary_count,
                "refs": data.get("refs").cloned(),
                "tx_id": data.get("tx_id").cloned(),
            })
        }
        "review_check" => {
            let issues_count = data
                .get("issues")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            json!({
                "overall_status": data.get("overall_status").cloned(),
                "recommended_action": data.get("recommended_action").cloned(),
                "issues_count": issues_count,
            })
        }
        "skill" => json!({
            "ok": data.get("ok").cloned(),
            "summary": data.get("summary").cloned(),
            "skill_name": data.get("skill_name").cloned(),
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
    let mut refs = serde_json::Map::new();

    if let Some(read_set) = result.meta.read_set.as_ref() {
        refs.insert("read_set".to_string(), json!(read_set));
    }
    if let Some(write_set) = result.meta.write_set.as_ref() {
        refs.insert("write_set".to_string(), json!(write_set));
    }

    if let Some(data) = result.data.as_ref() {
        for key in ["ref", "delta_id", "status", "mode"] {
            if let Some(value) = data.get(key).cloned() {
                refs.insert(key.to_string(), value);
            }
        }

        if tool_name == "structure_edit" {
            if let Some(value) = data.get("refs").cloned() {
                refs.insert("refs".to_string(), value);
            }
        }
    }

    if refs.is_empty() {
        Value::Null
    } else {
        Value::Object(refs)
    }
}
