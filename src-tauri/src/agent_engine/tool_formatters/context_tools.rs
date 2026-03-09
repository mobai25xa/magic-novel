pub(super) fn format_outline_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let scope = args
        .get("volume_path")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("full");

    let text = payload
        .get("outline")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    format!("[outline scope={}]\n{}", scope, text)
}

pub(super) fn format_character_sheet_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("*");

    let text = payload.get("result").and_then(|v| v.as_str()).unwrap_or("");

    format!("[character_sheet name=\"{}\"]\n{}", name, text)
}

pub(super) fn format_search_knowledge_result(
    data: Option<&serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let Some(payload) = data else {
        return "null".to_string();
    };

    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("?");
    let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5);

    let text = payload.get("result").and_then(|v| v.as_str()).unwrap_or("");

    format!(
        "[search_knowledge query=\"{}\" top_k={}]\n{}",
        query, top_k, text
    )
}
