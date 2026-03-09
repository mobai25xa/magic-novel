use serde_json::json;

pub(super) fn format_ls_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let cwd = payload.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
    let items = payload.get("items").and_then(|v| v.as_array());
    let total = items.map(|a| a.len()).unwrap_or(0);

    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let requested_limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
    let limit = requested_limit.clamp(1, 200);

    let mut lines = Vec::new();

    let start = offset.min(total);
    let end = start.saturating_add(limit).min(total);
    lines.push(format!(
        "[ls path={} total={} showing={}-{}]",
        cwd,
        total,
        start + 1,
        end
    ));

    if let Some(items) = items {
        for item in items.iter().skip(start).take(limit) {
            let kind = item.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let children = item
                .get("child_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if kind == "folder" {
                lines.push(format!("  {}/ ({})  [{}]", name, path, children));
            } else {
                lines.push(format!("  {} ({})", name, path));
            }
        }

        if end < total {
            lines.push(format!(
                "\n[truncated: {} more items. Use ls(path=\"{}\", offset={}, limit={}) to continue]",
                total - end,
                cwd,
                end,
                limit
            ));
        }
    }

    lines.join("\n")
}

pub(super) fn format_grep_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("?");
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("keyword");
    let hits = payload.get("hits").and_then(|v| v.as_array());
    let hit_count = hits.map(|a| a.len()).unwrap_or(0);

    let mut lines = Vec::new();
    lines.push(format!(
        "[grep query=\"{}\" mode={} hits={}]",
        query, mode, hit_count
    ));

    if let Some(hits) = hits {
        for (i, hit) in hits.iter().enumerate() {
            let path = hit.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let score = hit.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let snippet = hit.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("\n{}. {} (score={:.2})", i + 1, path, score));
            lines.push(format!("   \"{}\"", snippet));
        }
    }

    if let Some(notice) = payload.get("semantic_notice") {
        if notice.get("semantic_retrieval_available") == Some(&json!(false)) {
            if let Some(msg) = notice.get("message").and_then(|v| v.as_str()) {
                lines.push(format!("\nNote: {}", msg));
            }
        }
    }

    lines.join("\n")
}
