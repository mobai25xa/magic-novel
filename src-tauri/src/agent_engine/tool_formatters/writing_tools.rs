pub(super) fn format_draft_write_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let target_ref = args
        .get("target_ref")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("?");
    let write_mode = args
        .get("write_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let mode = payload.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
    let accepted = payload
        .get("accepted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let tx_id = payload.get("tx_id").and_then(|v| v.as_str()).unwrap_or("");

    let mut header = format!(
        "[draft_write mode={} accepted={} write_mode={} target_ref={}]",
        mode, accepted, write_mode, target_ref
    );
    if !tx_id.trim().is_empty() {
        header.push_str(&format!(" tx={}", tx_id.trim()));
    }

    let mut lines = vec![header];

    if let Some(diffs) = payload.get("diff_summary").and_then(|v| v.as_array()) {
        if !diffs.is_empty() {
            lines.push("diff_summary:".to_string());
            for item in diffs {
                if let Some(text) = item.as_str() {
                    lines.push(format!("- {}", text.trim()));
                } else {
                    lines.push(format!("- {}", item));
                }
            }
        }
    }

    if let Some(snippet) = payload.get("snippet_after").and_then(|v| v.as_str()) {
        if !snippet.trim().is_empty() {
            lines.push("snippet_after:".to_string());
            lines.push(snippet.trim().to_string());
        }
    }

    lines.join("\n")
}

pub(super) fn format_structure_edit_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("?");
    let node_type = args
        .get("node_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let mode = payload.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
    let accepted = payload
        .get("accepted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let tx_id = payload.get("tx_id").and_then(|v| v.as_str()).unwrap_or("");

    let mut header = format!(
        "[structure_edit mode={} accepted={} op={} node_type={}]",
        mode, accepted, op, node_type
    );
    if !tx_id.trim().is_empty() {
        header.push_str(&format!(" tx={}", tx_id.trim()));
    }

    let mut lines = vec![header];

    if let Some(refs) = payload.get("refs") {
        lines.push(format!("refs: {}", refs));
    }

    if let Some(items) = payload.get("impact_summary").and_then(|v| v.as_array()) {
        if !items.is_empty() {
            lines.push("impact_summary:".to_string());
            for item in items {
                if let Some(text) = item.as_str() {
                    lines.push(format!("- {}", text.trim()));
                } else {
                    lines.push(format!("- {}", item));
                }
            }
        }
    }

    lines.join("\n")
}

pub(super) fn format_knowledge_write_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let changes_count = args
        .get("changes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    let delta_id = payload
        .get("delta_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let next_action = payload
        .get("next_action")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut header = format!(
        "[knowledge_write status={} delta_id={} changes={}]",
        status, delta_id, changes_count
    );
    if !next_action.trim().is_empty() {
        header.push_str(&format!(" next_action={}", next_action.trim()));
    }

    let mut lines = vec![header];

    if let Some(conflicts) = payload.get("conflicts").and_then(|v| v.as_array()) {
        if !conflicts.is_empty() {
            lines.push(format!("conflicts: {}", conflicts.len()));
        }
    }

    lines.join("\n")
}
