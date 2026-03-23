use serde_json::Value;

/// Resolve the effective project_path for tool execution.
///
/// - Agent engine path: `project_path` is injected via runtime context and is non-empty.
/// - Legacy Tauri tool-runtime path: `project_path` is empty and input includes `project_path`.
pub(super) fn take_project_path(project_path: &str, input: &mut Value) -> Result<String, String> {
    if !project_path.trim().is_empty() {
        return Ok(project_path.to_string());
    }

    let Value::Object(map) = input else {
        return Err("input must be an object".to_string());
    };

    match map.remove("project_path") {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s),
        Some(_) => Err("project_path must be a non-empty string".to_string()),
        None => Err("project_path is required in this transport layer".to_string()),
    }
}

pub(super) fn classify_serde_error(err: &serde_json::Error) -> (&'static str, String) {
    let msg = err.to_string();
    if msg.contains("unknown field") {
        ("E_TOOL_UNKNOWN_FIELD", msg)
    } else {
        ("E_TOOL_SCHEMA_INVALID", msg)
    }
}
