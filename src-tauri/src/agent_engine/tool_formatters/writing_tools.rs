fn format_impact(impact: Option<&serde_json::Value>) -> String {
    let Some(impact) = impact else {
        return String::new();
    };

    let chapter_count = impact.get("chapter_count").and_then(|v| v.as_u64());
    let chapter_id = impact.get("chapter_id").and_then(|v| v.as_str());

    match (chapter_count, chapter_id) {
        (Some(count), _) => format!(" impact=chapter_count:{}", count),
        (_, Some(id)) => format!(" impact=chapter_id:{}", id),
        _ => String::new(),
    }
}

pub(super) fn format_read_result(data: Option<&serde_json::Value>) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("?");
    let revision = payload
        .get("revision")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let hash = payload.get("hash").and_then(|v| v.as_str()).unwrap_or("?");

    if payload.get("content_json").is_some() && payload.get("content").is_none() {
        return serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    }

    let body = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut result = format!(
        "[revision={}] Use this revision number as base_revision when calling edit.\n[path={} hash={}]\n\n{}",
        revision, path, hash, body
    );

    if let Some(trunc) = payload.get("truncated") {
        let total = trunc
            .get("total_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let returned = trunc
            .get("returned_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let next_offset = trunc
            .get("next_offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        result.push_str(&format!(
            "\n\n[truncated: returned {} of {} chars. Continue with another read call from offset {} if needed.]",
            returned, total, next_offset
        ));
    }

    result
}

pub(super) fn format_edit_result(data: Option<&serde_json::Value>) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let mode = payload.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("?");
    let rev_before = payload
        .get("revision_before")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let rev_after = payload
        .get("revision_after")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let tx_id = payload.get("tx_id").and_then(|v| v.as_str()).unwrap_or("");

    let mut lines = Vec::new();

    if mode == "preview" {
        lines.push(format!(
            "[preview path={} revision={}→{}]",
            path, rev_before, rev_after
        ));

        if let Some(diffs) = payload.get("diff_summary").and_then(|v| v.as_array()) {
            if !diffs.is_empty() {
                lines.push("Changes:".to_string());
                for d in diffs {
                    let op = d.get("operation").and_then(|v| v.as_str()).unwrap_or("?");
                    let desc = d.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    lines.push(format!("  - {} {}", op, desc));
                }
            }
        }

        if let Some(diags) = payload.get("diagnostics").and_then(|v| v.as_array()) {
            let errors: Vec<_> = diags
                .iter()
                .filter(|d| d.get("level").and_then(|v| v.as_str()) == Some("error"))
                .collect();
            if errors.is_empty() {
                lines.push("No issues found. Preview only — call edit with dry_run=false if you want to commit this change.".to_string());
            } else {
                for e in errors {
                    let msg = e.get("message").and_then(|v| v.as_str()).unwrap_or("?");
                    lines.push(format!("  ERROR: {}", msg));
                }
            }
        } else {
            lines.push("No issues found. Preview only — call edit with dry_run=false if you want to commit this change.".to_string());
        }
    } else {
        let tx_part = if tx_id.is_empty() {
            String::new()
        } else {
            format!(" tx={}", tx_id)
        };
        lines.push(format!(
            "[committed path={} revision={}→{}{}]",
            path, rev_before, rev_after, tx_part
        ));
        lines.push("Edit applied successfully.".to_string());
    }

    lines.join("\n")
}

pub(super) fn format_create_result(data: Option<&serde_json::Value>) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("?");
    let revision = payload
        .get("revision_after")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let kind = payload
        .get("created_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("file");

    format!(
        "[created path={} revision={}]\n{} created successfully.",
        path, revision, kind
    )
}

pub(super) fn format_delete_result(data: Option<&serde_json::Value>) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("preview");
    let kind = payload.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("?");
    let impact = format_impact(payload.get("impact"));

    if mode == "preview" {
        format!("[delete preview kind={} path={}{}]", kind, path, impact)
    } else {
        format!(
            "[delete committed kind={} path={}{}]\nMoved to recycle bin.",
            kind, path, impact
        )
    }
}

pub(super) fn format_move_result(data: Option<&serde_json::Value>) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("preview");
    let chapter_path = payload
        .get("chapter_path")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let target_volume_path = payload
        .get("target_volume_path")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let target_index = payload
        .get("target_index")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let new_path = payload
        .get("new_chapter_path")
        .and_then(|v| v.as_str())
        .unwrap_or(chapter_path);

    if mode == "preview" {
        format!(
            "[move preview chapter={} -> volume={} index={} next_path={}]",
            chapter_path, target_volume_path, target_index, new_path
        )
    } else {
        format!(
            "[move committed chapter={} -> volume={} index={} new_path={}]",
            chapter_path, target_volume_path, target_index, new_path
        )
    }
}
