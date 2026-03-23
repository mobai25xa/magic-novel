use std::time::Instant;

use serde_json::Value;

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
use crate::models::AppError;
use crate::services::tool_audit::{emit_tool_audit, ToolAuditRecord};

pub(super) fn map_app_error(
    tool: &str,
    call_id: String,
    started: Instant,
    err: AppError,
) -> ToolResult<Value> {
    let code = err
        .details
        .as_ref()
        .and_then(|d| d.get("code"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("E_TOOL_{}_FAILED", tool.to_uppercase()));

    let fault_domain = if code.starts_with("E_JVM_") {
        FaultDomain::Jvm
    } else if code.starts_with("E_VC_") {
        FaultDomain::Vc
    } else if code.starts_with("E_IO_") {
        FaultDomain::Io
    } else {
        FaultDomain::Tool
    };

    let result = ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code,
            message: err.message,
            retryable: err.recoverable.unwrap_or(false),
            fault_domain,
            details: err.details,
        }),
        meta: ToolMeta {
            tool: tool.to_string(),
            call_id,
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    };

    emit_from_result(&result, "result");
    result
}

pub(super) fn emit_from_result(result: &ToolResult<Value>, stage: &str) {
    let fault_domain = result
        .error
        .as_ref()
        .map(|error| format!("{:?}", error.fault_domain).to_lowercase());
    let error_code = result.error.as_ref().map(|error| error.code.clone());

    emit_tool_audit(&ToolAuditRecord {
        tool: result.meta.tool.clone(),
        call_id: result.meta.call_id.clone(),
        ok: result.ok,
        duration_ms: result.meta.duration_ms,
        revision_before: result.meta.revision_before,
        revision_after: result.meta.revision_after,
        tx_id: result.meta.tx_id.clone(),
        fault_domain,
        error_code,
        error_message: result.error.as_ref().map(|e| e.message.clone()),
        stage: stage.to_string(),
    });
}
