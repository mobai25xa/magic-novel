//! Tool Runtime - Execution engine

use std::time::Instant;

use serde_json::Value;

mod helpers;
mod input;
mod refs;

mod draft_write;
mod inspiration;
mod structure_edit;

#[cfg(test)]
mod contract_regressions;
#[cfg(test)]
mod tests;

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
use crate::review::engine as review_engine;
use crate::review::types::ReviewRunInput;
use helpers::{emit_from_result, map_app_error};
use input::{classify_serde_error, take_project_path};

fn fault_domain_for_code(code: &str) -> FaultDomain {
    match code {
        "E_TOOL_SCHEMA_INVALID"
        | "E_TOOL_UNKNOWN_FIELD"
        | "E_REF_INVALID"
        | "E_REF_KIND_UNSUPPORTED"
        | "E_PAYLOAD_TOO_LARGE" => FaultDomain::Validation,
        "E_REF_NOT_FOUND" | "E_IO" => FaultDomain::Io,
        "E_CONFLICT" => FaultDomain::Vc,
        "E_NOT_IMPLEMENTED" | "E_INTERNAL" => FaultDomain::Tool,
        _ => {
            if code.starts_with("E_IO") {
                FaultDomain::Io
            } else {
                FaultDomain::Tool
            }
        }
    }
}

fn tool_ok(
    tool: &str,
    call_id: String,
    started: Instant,
    data: Value,
    read_set: Option<Vec<String>>,
    write_set: Option<Vec<String>>,
) -> ToolResult<Value> {
    ToolResult {
        ok: true,
        data: Some(data),
        error: None,
        meta: ToolMeta {
            tool: tool.to_string(),
            call_id,
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set,
            write_set,
        },
    }
}

fn tool_err(
    tool: &str,
    call_id: String,
    started: Instant,
    code: &str,
    message: &str,
    retryable: bool,
    fault_domain: FaultDomain,
    read_set: Option<Vec<String>>,
    write_set: Option<Vec<String>>,
) -> ToolResult<Value> {
    ToolResult {
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
            read_set,
            write_set,
        },
    }
}

pub fn execute_workspace_map(
    project_path: &str,
    mut input: Value,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                "workspace_map",
                call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: crate::agent_tools::tools::workspace_map::WorkspaceMapArgs =
        match serde_json::from_value(input) {
            Ok(v) => v,
            Err(err) => {
                let (code, msg) = classify_serde_error(&err);
                let result = tool_err(
                    "workspace_map",
                    call_id,
                    started,
                    code,
                    &msg,
                    false,
                    FaultDomain::Validation,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        };

    let result =
        match crate::agent_tools::tools::workspace_map::run_workspace_map(&project_path, args) {
            Ok(run) => tool_ok(
                "workspace_map",
                call_id,
                started,
                serde_json::to_value(run.output).expect("workspace_map output should serialize"),
                run.read_set,
                None,
            ),
            Err(err) => tool_err(
                "workspace_map",
                call_id,
                started,
                err.code,
                &err.message,
                false,
                fault_domain_for_code(err.code),
                None,
                None,
            ),
        };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_context_read(
    project_path: &str,
    mut input: Value,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                "context_read",
                call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: crate::agent_tools::tools::context_read::ContextReadArgs =
        match serde_json::from_value(input) {
            Ok(v) => v,
            Err(err) => {
                let (code, msg) = classify_serde_error(&err);
                let result = tool_err(
                    "context_read",
                    call_id,
                    started,
                    code,
                    &msg,
                    false,
                    FaultDomain::Validation,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        };

    let result =
        match crate::agent_tools::tools::context_read::run_context_read(&project_path, args) {
            Ok(run) => tool_ok(
                "context_read",
                call_id,
                started,
                serde_json::to_value(run.output).expect("context_read output should serialize"),
                run.read_set,
                None,
            ),
            Err(err) => tool_err(
                "context_read",
                call_id,
                started,
                err.code,
                &err.message,
                false,
                fault_domain_for_code(err.code),
                None,
                None,
            ),
        };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_context_search(
    project_path: &str,
    mut input: Value,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                "context_search",
                call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: crate::agent_tools::tools::context_search::ContextSearchArgs =
        match serde_json::from_value(input) {
            Ok(v) => v,
            Err(err) => {
                let (code, msg) = classify_serde_error(&err);
                let result = tool_err(
                    "context_search",
                    call_id,
                    started,
                    code,
                    &msg,
                    false,
                    FaultDomain::Validation,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        };

    let result =
        match crate::agent_tools::tools::context_search::run_context_search(&project_path, args) {
            Ok(run) => tool_ok(
                "context_search",
                call_id,
                started,
                serde_json::to_value(run.output).expect("context_search output should serialize"),
                run.read_set,
                None,
            ),
            Err(err) => tool_err(
                "context_search",
                call_id,
                started,
                err.code,
                &err.message,
                false,
                fault_domain_for_code(err.code),
                None,
                None,
            ),
        };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_knowledge_read(
    project_path: &str,
    mut input: Value,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                "knowledge_read",
                call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: crate::agent_tools::tools::knowledge_read::KnowledgeReadArgs =
        match serde_json::from_value(input) {
            Ok(v) => v,
            Err(err) => {
                let (code, msg) = classify_serde_error(&err);
                let result = tool_err(
                    "knowledge_read",
                    call_id,
                    started,
                    code,
                    &msg,
                    false,
                    FaultDomain::Validation,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        };

    let result =
        match crate::agent_tools::tools::knowledge_read::run_knowledge_read(&project_path, args) {
            Ok(run) => tool_ok(
                "knowledge_read",
                call_id,
                started,
                serde_json::to_value(run.output).expect("knowledge_read output should serialize"),
                run.read_set,
                None,
            ),
            Err(err) => tool_err(
                "knowledge_read",
                call_id,
                started,
                err.code,
                &err.message,
                false,
                fault_domain_for_code(err.code),
                None,
                None,
            ),
        };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_knowledge_write(
    project_path: &str,
    mut input: Value,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                "knowledge_write",
                call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    if let Err(err) = crate::agent_tools::tools::knowledge_write::validate_knowledge_write_input_shape(&input)
    {
        let result = tool_err(
            "knowledge_write",
            call_id,
            started,
            err.code,
            &err.message,
            false,
            FaultDomain::Validation,
            None,
            None,
        );
        emit_from_result(&result, "execute");
        return result;
    }

    let args: crate::agent_tools::tools::knowledge_write::KnowledgeWriteArgs =
        match serde_json::from_value(input) {
            Ok(v) => v,
            Err(err) => {
                let (code, msg) = classify_serde_error(&err);
                let result = tool_err(
                    "knowledge_write",
                    call_id,
                    started,
                    code,
                    &msg,
                    false,
                    FaultDomain::Validation,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        };

    let result = match crate::agent_tools::tools::knowledge_write::run_knowledge_write(
        &project_path,
        &call_id,
        args,
    ) {
        Ok(run) => tool_ok(
            "knowledge_write",
            call_id,
            started,
            serde_json::to_value(run.output).expect("knowledge_write output should serialize"),
            run.read_set,
            run.write_set,
        ),
        Err(err) => tool_err(
            "knowledge_write",
            call_id,
            started,
            err.code,
            &err.message,
            false,
            fault_domain_for_code(err.code),
            None,
            None,
        ),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_draft_write(project_path: &str, input: Value, call_id: String) -> ToolResult<Value> {
    draft_write::execute(project_path, input, call_id)
}

pub fn execute_inspiration_consensus_patch(input: Value, call_id: String) -> ToolResult<Value> {
    inspiration::execute_consensus_patch(input, call_id)
}

pub fn execute_inspiration_open_questions_patch(
    input: Value,
    call_id: String,
) -> ToolResult<Value> {
    inspiration::execute_open_questions_patch(input, call_id)
}

pub fn execute_structure_edit(
    project_path: &str,
    input: Value,
    call_id: String,
) -> ToolResult<Value> {
    structure_edit::execute(project_path, input, call_id)
}

pub fn execute_review_check(
    project_path: &str,
    input: ReviewRunInput,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();
    let read_set = Some(input.target_refs.clone());

    let result = match review_engine::run_review(std::path::Path::new(project_path), input) {
        Ok(report) => ToolResult {
            ok: true,
            data: Some(serde_json::to_value(report).expect("review report should serialize")),
            error: None,
            meta: ToolMeta {
                tool: "review_check".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set,
                write_set: None,
            },
        },
        Err(err) => map_app_error("review_check", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}
