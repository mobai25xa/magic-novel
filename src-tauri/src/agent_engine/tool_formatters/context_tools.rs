pub(super) fn format_workspace_map_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let scope = args
        .get("scope")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("book");
    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(2);
    let target_ref = args
        .get("target_ref")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());

    let mut lines = Vec::new();

    let mut header = format!("[workspace_map scope={} depth={}]", scope, depth);
    if let Some(tref) = target_ref {
        header.push_str(&format!(" target_ref={}", tref));
    }
    lines.push(header);

    if let Some(summary) = payload.get("summary").and_then(|v| v.as_str()) {
        if !summary.trim().is_empty() {
            lines.push(format!("summary: {}", summary.trim()));
        }
    }

    if let Some(items) = payload.get("tree").and_then(|v| v.as_array()) {
        for node in items {
            let kind = node.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let ref_ = node.get("ref").and_then(|v| v.as_str()).unwrap_or("?");
            let title = node.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let status = node.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let words = node.get("word_count").and_then(|v| v.as_u64());
            let children = node.get("child_count").and_then(|v| v.as_u64());

            let mut line = format!("- {} {}", kind, ref_);
            if !title.trim().is_empty() {
                line.push_str(&format!(" \"{}\"", title.trim()));
            }
            if let Some(c) = children {
                line.push_str(&format!(" children={}", c));
            }
            if let Some(w) = words {
                line.push_str(&format!(" words={}", w));
            }
            if !status.trim().is_empty() {
                line.push_str(&format!(" status={}", status.trim()));
            }
            lines.push(line);
        }
    }

    if payload.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(cursor) = payload.get("next_cursor").and_then(|v| v.as_str()) {
            if !cursor.trim().is_empty() {
                lines.push(format!(
                    "[truncated: next_cursor={} — call workspace_map(cursor=\"{}\") to continue]",
                    cursor.trim(),
                    cursor.trim()
                ));
            }
        }
    }

    lines.join("\n")
}

pub(super) fn format_context_read_result(
    data: Option<&serde_json::Value>,
    _args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let ref_ = payload.get("ref").and_then(|v| v.as_str()).unwrap_or("?");
    let kind = payload.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
    let truncated = payload
        .get("truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut header = format!("[context_read ref={} kind={}]", ref_, kind);
    if truncated {
        header.push_str(" truncated=true");
    }

    format!("{header}\n{content}")
}

pub(super) fn format_context_search_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("?");
    let corpus = args.get("corpus").and_then(|v| v.as_str()).unwrap_or("all");
    let requested_mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("keyword");
    let effective_mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or(requested_mode);
    let hits = payload.get("hits").and_then(|v| v.as_array());
    let hit_count = hits.map(|a| a.len()).unwrap_or(0);

    let mut lines = Vec::new();
    lines.push(format!(
        "[context_search query=\"{}\" corpus={} mode={} hits={}]",
        query, corpus, effective_mode, hit_count
    ));

    if payload.get("degraded").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(reason) = payload.get("degraded_reason").and_then(|v| v.as_str()) {
            if !reason.trim().is_empty() {
                lines.push(format!("note: degraded=true reason={}", reason.trim()));
            }
        } else {
            lines.push("note: degraded=true".to_string());
        }
    }

    if let Some(hits) = hits {
        for (idx, hit) in hits.iter().enumerate() {
            let ref_ = hit.get("ref").and_then(|v| v.as_str()).unwrap_or("?");
            let score = hit.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let snippet = hit.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{:02}. {} (score={:.2})", idx + 1, ref_, score));
            if !snippet.trim().is_empty() {
                lines.push(format!("    \"{}\"", snippet.trim()));
            }
        }
    }

    lines.join("\n")
}

pub(super) fn format_knowledge_read_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let view_mode = args
        .get("view_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("compact");
    let knowledge_type = args
        .get("knowledge_type")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let item_ref = args
        .get("item_ref")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let items = payload.get("items").and_then(|v| v.as_array());
    let total = items.map(|a| a.len()).unwrap_or(0);

    let mut lines = Vec::new();
    let mut header = format!("[knowledge_read view_mode={} items={}]", view_mode, total);
    if let Some(ty) = knowledge_type {
        header.push_str(&format!(" knowledge_type={}", ty));
    }
    if let Some(q) = query {
        header.push_str(&format!(" query=\"{}\"", q));
    }
    if let Some(r) = item_ref {
        header.push_str(&format!(" item_ref={}", r));
    }
    if payload.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
        header.push_str(" truncated=true");
    }
    lines.push(header);

    if let Some(items) = items {
        for (idx, item) in items.iter().enumerate() {
            let ref_ = item.get("ref").and_then(|v| v.as_str()).unwrap_or("?");
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let summary = item.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");

            let mut line = format!("{:02}. {}", idx + 1, ref_);
            if !title.trim().is_empty() {
                line.push_str(&format!(" \"{}\"", title.trim()));
            }
            lines.push(line);

            if !summary.trim().is_empty() {
                lines.push(format!("    summary: {}", summary.trim()));
            }
            if view_mode == "full" && !snippet.trim().is_empty() {
                lines.push(format!("    snippet: {}", snippet.trim()));
            }
        }
    }

    lines.join("\n")
}
