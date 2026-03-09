use std::time::Instant;

use serde_json::Value;

use crate::agent_tools::contracts::{
    EditInput, FaultDomain, ReadInput, ToolError, ToolMeta, ToolResult,
};
use crate::models::AppError;
use crate::services::tool_audit::{emit_tool_audit, ToolAuditRecord};

pub(super) fn tool_error(
    tool: &str,
    call_id: String,
    started: Instant,
    code: &str,
    message: &str,
    retryable: bool,
    fault_domain: FaultDomain,
) -> ToolResult<Value> {
    let result = ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            fault_domain,
            details: None,
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

    emit_from_result(&result, "policy");
    result
}

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

pub(super) fn extract_revision(v: &Value, field: &str) -> Option<u64> {
    v.get(field).and_then(|it| {
        it.as_u64()
            .or_else(|| it.as_i64().and_then(|n| (n >= 0).then_some(n as u64)))
    })
}

pub(super) fn extract_tx_id(v: &Value) -> Option<String> {
    v.get("tx_id")
        .and_then(|it| it.as_str())
        .map(ToString::to_string)
}

pub(super) fn read_entity_ref(input: &ReadInput) -> String {
    let kind = input
        .kind
        .as_ref()
        .map(|k| match k {
            crate::agent_tools::contracts::ReadKind::Volume => "volume",
            crate::agent_tools::contracts::ReadKind::Chapter => "chapter",
        })
        .unwrap_or("chapter");
    format!("{}:{}", kind, input.path)
}

pub(super) fn edit_entity_ref(input: &EditInput) -> String {
    let kind = match input.target.as_ref() {
        Some(crate::agent_tools::contracts::EditTarget::VolumeMeta) => "volume",
        _ => "chapter",
    };
    format!("{}:{}", kind, input.path)
}
