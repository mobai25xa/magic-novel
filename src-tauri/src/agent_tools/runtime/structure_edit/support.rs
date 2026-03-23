use std::time::Instant;

use serde_json::Value;

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
use crate::models::{AppError, ErrorCode};

use super::{StructureEditArgs, StructureNodeType, StructureOp, TOOL_NAME};

pub(super) fn validate_args(args: &StructureEditArgs) -> Result<(), String> {
    match args.op {
        StructureOp::Create => {
            let title = args.title.as_deref().unwrap_or("").trim();
            if title.is_empty() {
                return Err("create requires title".to_string());
            }
            if args.node_type == StructureNodeType::Chapter && args.parent_ref.is_none() {
                return Err("create(chapter) requires parent_ref (volume ref)".to_string());
            }
            if let Some(pos) = args.position {
                if pos < 0 {
                    return Err("position must be >= 0".to_string());
                }
            }
            Ok(())
        }
        StructureOp::Move => {
            if args.target_ref.as_deref().unwrap_or("").trim().is_empty() {
                return Err("move requires target_ref".to_string());
            }
            if args.parent_ref.as_deref().unwrap_or("").trim().is_empty() {
                return Err("move requires parent_ref".to_string());
            }
            let Some(pos) = args.position else {
                return Err("move requires position".to_string());
            };
            if pos < 0 {
                return Err("position must be >= 0".to_string());
            }
            Ok(())
        }
        StructureOp::Rename => {
            if args.target_ref.as_deref().unwrap_or("").trim().is_empty() {
                return Err("rename requires target_ref".to_string());
            }
            let title = args.title.as_deref().unwrap_or("").trim();
            if title.is_empty() {
                return Err("rename requires title".to_string());
            }
            Ok(())
        }
        StructureOp::Archive | StructureOp::Restore => {
            if args.target_ref.as_deref().unwrap_or("").trim().is_empty() {
                return Err("archive/restore requires target_ref".to_string());
            }
            Ok(())
        }
    }
}

pub(super) fn new_tx_id() -> String {
    format!("tx_struct_{}", uuid::Uuid::new_v4())
}

pub(super) fn map_app_error(err: AppError) -> ToolError {
    let (code, fault_domain) = match err.code {
        ErrorCode::NotFound => ("E_REF_NOT_FOUND", FaultDomain::Io),
        ErrorCode::InvalidArgument => ("E_TOOL_SCHEMA_INVALID", FaultDomain::Validation),
        ErrorCode::Conflict => ("E_CONFLICT", FaultDomain::Vc),
        ErrorCode::IoError => ("E_IO", FaultDomain::Io),
        _ => ("E_INTERNAL", FaultDomain::Tool),
    };

    ToolError {
        code: code.to_string(),
        message: err.message,
        retryable: err.recoverable.unwrap_or(false),
        fault_domain,
        details: err.details,
    }
}

pub(super) fn chapter_filename(chapter_path: &str) -> &str {
    chapter_path.split('/').last().unwrap_or("")
}

pub(super) fn chapter_volume_path(chapter_path: &str) -> Option<String> {
    let parts: Vec<&str> = chapter_path.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    Some(parts[..parts.len() - 1].join("/"))
}

pub(super) fn tool_err(
    call_id: &str,
    started: Instant,
    code: &str,
    message: &str,
    retryable: bool,
    fault_domain: FaultDomain,
    read_set: Option<Vec<String>>,
    write_set: Option<Vec<String>>,
    tx_id: Option<String>,
    details: Option<Value>,
) -> ToolResult<Value> {
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            fault_domain,
            details,
        }),
        meta: ToolMeta {
            tool: TOOL_NAME.to_string(),
            call_id: call_id.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id,
            read_set,
            write_set,
        },
    }
}
