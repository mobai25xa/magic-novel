use std::collections::HashSet;

use serde_json::json;

use crate::models::{AppError, ErrorCode};

pub(super) fn tool_is_allowed(allowed_tools: Option<&HashSet<String>>, tool_name: &str) -> bool {
    let Some(allowed) = allowed_tools else {
        return true;
    };
    allowed.contains(&tool_name.trim().to_ascii_lowercase())
}

pub(super) fn tool_not_allowed_result(
    tool_name: &str,
    call_id: &str,
    allowed_tools: Option<&HashSet<String>>,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};

    let allowed = allowed_tools
        .map(|set| {
            let mut tools = set.iter().cloned().collect::<Vec<_>>();
            tools.sort();
            tools
        })
        .unwrap_or_default();

    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_TOOL_NOT_ALLOWED".to_string(),
            message: format!("tool '{}' is not allowed in this turn", tool_name),
            retryable: false,
            fault_domain: FaultDomain::Policy,
            details: Some(json!({
                "tool": tool_name,
                "allowed_tools": allowed,
            })),
        }),
        meta: ToolMeta {
            tool: tool_name.to_string(),
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

pub(super) fn tool_cancelled_result(
    tool_name: &str,
    call_id: &str,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};

    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_CANCELLED".to_string(),
            message: "cancelled".to_string(),
            retryable: false,
            fault_domain: FaultDomain::Policy,
            details: Some(json!({ "tool": tool_name })),
        }),
        meta: ToolMeta {
            tool: tool_name.to_string(),
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

pub(super) fn cancelled_error(call_id: Option<&str>) -> AppError {
    AppError {
        code: ErrorCode::Internal,
        message: "cancelled".to_string(),
        details: Some(json!({
            "code": "E_CANCELLED",
            "call_id": call_id,
        })),
        recoverable: Some(true),
    }
}
