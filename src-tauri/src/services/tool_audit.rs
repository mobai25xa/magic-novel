use serde::Serialize;

use crate::agent_engine::tool_errors::sanitize_error_message;

#[derive(Debug, Clone, Serialize)]
pub struct ToolAuditRecord {
    pub tool: String,
    pub call_id: String,
    pub ok: bool,
    pub duration_ms: u64,
    pub revision_before: Option<u64>,
    pub revision_after: Option<u64>,
    pub tx_id: Option<String>,
    pub fault_domain: Option<String>,
    pub error_code: Option<String>,
    /// Error message (sanitized before logging to strip paths and secrets).
    pub error_message: Option<String>,
    pub stage: String,
}

pub fn emit_tool_audit(record: &ToolAuditRecord) {
    let sanitized_msg = record.error_message.as_deref().map(sanitize_error_message);

    tracing::info!(
        target: "tool_audit",
        tool = %record.tool,
        call_id = %record.call_id,
        ok = record.ok,
        duration_ms = record.duration_ms,
        revision_before = ?record.revision_before,
        revision_after = ?record.revision_after,
        tx_id = ?record.tx_id,
        fault_domain = ?record.fault_domain,
        error_code = ?record.error_code,
        error_message = ?sanitized_msg,
        stage = %record.stage,
        "tool_runtime_result"
    );
}
