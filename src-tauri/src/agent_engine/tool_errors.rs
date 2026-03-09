//! Error constructors and resource-key helpers for the tool scheduler.

use regex::Regex;
use serde_json::json;
use std::sync::LazyLock;

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
use crate::agent_tools::registry::get_manifest;
use crate::models::AppError;

use super::types::ToolCallInfo;

// ── Sanitization patterns ──

static RE_WIN_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[A-Z]:\\[^\s"']+"#).unwrap());

static RE_UNIX_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"/(?:home|Users|tmp|var|etc)/[^\s"']+"#).unwrap());

static RE_API_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:sk|key|token|api[_\-]?key)[_\-]?[A-Za-z0-9]{8,}").unwrap()
});

/// Sanitize an error message before sending it to the LLM or writing to audit logs.
/// Replaces absolute file paths and API key patterns with placeholders.
pub(crate) fn sanitize_error_message(msg: &str) -> String {
    let s = RE_WIN_PATH.replace_all(msg, "[path]");
    let s = RE_UNIX_PATH.replace_all(&s, "[path]");
    let s = RE_API_KEY.replace_all(&s, "[redacted]");
    s.into_owned()
}

pub(crate) fn tool_join_error(
    tool: &str,
    call_id: &str,
    msg: &str,
) -> ToolResult<serde_json::Value> {
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_TOOL_TASK_JOIN_FAILED".to_string(),
            message: msg.to_string(),
            retryable: true,
            fault_domain: FaultDomain::Tool,
            details: None,
        }),
        meta: ToolMeta {
            tool: tool.to_string(),
            call_id: call_id.to_string(),
            duration_ms: 0,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

pub(crate) fn tool_lock_error(
    tool: &str,
    call_id: &str,
    app_error: &AppError,
) -> ToolResult<serde_json::Value> {
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: app_error
                .details
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("E_TOOL_RESOURCE_LOCK_FAILED")
                .to_string(),
            message: app_error.message.clone(),
            retryable: app_error.recoverable.unwrap_or(true),
            fault_domain: FaultDomain::Policy,
            details: Some(json!({
                "app_error": app_error.details,
                "call_id": call_id,
                "tool": tool,
            })),
        }),
        meta: ToolMeta {
            tool: tool.to_string(),
            call_id: call_id.to_string(),
            duration_ms: 0,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

pub(crate) fn tool_timeout_error(
    tool: &str,
    call_id: &str,
    timeout: std::time::Duration,
) -> ToolResult<serde_json::Value> {
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_TOOL_TIMEOUT".to_string(),
            message: format!(
                "tool '{}' exceeded timeout of {}ms",
                tool,
                timeout.as_millis()
            ),
            retryable: true,
            fault_domain: FaultDomain::Tool,
            details: Some(json!({
                "timeout_ms": timeout.as_millis() as u64,
                "tool": tool,
                "call_id": call_id,
            })),
        }),
        meta: ToolMeta {
            tool: tool.to_string(),
            call_id: call_id.to_string(),
            duration_ms: timeout.as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

pub(crate) fn write_resource_key(tc: &ToolCallInfo, project_path: &str) -> Option<String> {
    match tc.tool_name.as_str() {
        "edit" => {
            let path = tc.args.get("path").and_then(|v| v.as_str())?.trim();
            if path.is_empty() {
                None
            } else {
                Some(format!(
                    "{}::{}",
                    project_path,
                    normalize_resource_segment(path)
                ))
            }
        }
        "create" => {
            let volume = tc
                .args
                .get("volume_path")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .trim();
            let title = tc
                .args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if title.is_empty() {
                None
            } else {
                Some(format!(
                    "{}::{}::{}",
                    project_path,
                    normalize_resource_segment(volume),
                    normalize_resource_segment(title)
                ))
            }
        }
        _ => None,
    }
}

fn normalize_resource_segment(segment: &str) -> String {
    segment
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// Look up the timeout for a tool from its manifest, falling back to the default.
pub(crate) fn get_tool_timeout(tool_name: &str) -> std::time::Duration {
    use crate::agent_tools::definition::DEFAULT_TOOL_TIMEOUT_MS;
    let ms = get_manifest(tool_name)
        .map(|m| m.timeout_ms)
        .unwrap_or(DEFAULT_TOOL_TIMEOUT_MS);
    std::time::Duration::from_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_write_resource_key_for_edit() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "edit".to_string(),
            args: json!({ "path": "vol_1/ch_1.json" }),
        };
        let key = write_resource_key(&tc, "D:/p").expect("key");
        assert_eq!(key, "D:/p::vol_1/ch_1.json");
    }

    #[test]
    fn test_write_resource_key_for_create() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "create".to_string(),
            args: json!({ "volume_path": "vol_1", "title": "Chapter A" }),
        };
        let key = write_resource_key(&tc, "D:/p").expect("key");
        assert_eq!(key, "D:/p::vol_1::Chapter A");
    }

    #[test]
    fn test_write_resource_key_normalizes_path() {
        let tc = ToolCallInfo {
            llm_call_id: "c2".to_string(),
            tool_name: "edit".to_string(),
            args: json!({ "path": "\\vol_1\\ch_1.json" }),
        };
        let key = write_resource_key(&tc, "D:/p").expect("key");
        assert_eq!(key, "D:/p::vol_1/ch_1.json");
    }

    #[test]
    fn test_get_tool_timeout_returns_manifest_value() {
        let dur = get_tool_timeout("read");
        assert_eq!(dur, std::time::Duration::from_millis(30_000));

        let dur = get_tool_timeout("edit");
        assert_eq!(dur, std::time::Duration::from_millis(60_000));

        let dur = get_tool_timeout("grep");
        assert_eq!(dur, std::time::Duration::from_millis(60_000));

        let dur = get_tool_timeout("skill");
        assert_eq!(dur, std::time::Duration::from_millis(10_000));
    }

    #[test]
    fn test_get_tool_timeout_unknown_tool_uses_default() {
        use crate::agent_tools::definition::DEFAULT_TOOL_TIMEOUT_MS;
        let dur = get_tool_timeout("nonexistent_tool");
        assert_eq!(
            dur,
            std::time::Duration::from_millis(DEFAULT_TOOL_TIMEOUT_MS)
        );
    }

    #[test]
    fn test_tool_timeout_error_format() {
        let timeout = std::time::Duration::from_millis(30_000);
        let result = tool_timeout_error("read", "call_1", timeout);
        assert!(!result.ok);
        let err = result.error.as_ref().unwrap();
        assert_eq!(err.code, "E_TOOL_TIMEOUT");
        assert!(err.message.contains("30000ms"));
        assert!(err.retryable);
    }

    #[test]
    fn test_sanitize_strips_windows_path() {
        let msg = "file not found at D:\\Users\\admin\\project\\manuscripts\\ch1.json";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("D:\\Users"));
        assert!(sanitized.contains("[path]"));
    }

    #[test]
    fn test_sanitize_strips_unix_path() {
        let msg = "cannot read /home/user/novels/project/manuscripts/ch1.json";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("/home/user"));
        assert!(sanitized.contains("[path]"));
    }

    #[test]
    fn test_sanitize_strips_api_key() {
        let msg = "auth failed with sk-abc123defghijklmnop";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("sk-abc123"));
        assert!(sanitized.contains("[redacted]"));
    }

    #[test]
    fn test_sanitize_preserves_normal_message() {
        let msg = "revision mismatch: expected 3, got 5";
        let sanitized = sanitize_error_message(msg);
        assert_eq!(sanitized, msg);
    }

    #[test]
    fn test_sanitize_strips_api_key_token_pattern() {
        let msg = "failed with token_abcdefghij12345";
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("[redacted]"));
    }
}
