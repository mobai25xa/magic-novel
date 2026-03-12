//! Tool Runtime - Execution engine

use std::time::Instant;

use serde_json::Value;

mod helpers;

use crate::agent_tools::contracts::{
    CreateInput, DeleteInput, EditInput, FaultDomain, GrepInput, LsInput, MoveInput, ReadInput,
    ToolMeta, ToolResult,
};
use crate::agent_tools::registry::{get_manifest, visible_tools_for_model};
use crate::agent_tools::tools::{
    create_tool, delete_tool, edit_tool, grep_tool, ls_tool, move_tool, read_tool,
};
use crate::review::engine as review_engine;
use crate::review::types::ReviewRunInput;
use helpers::{
    edit_entity_ref, emit_from_result, extract_revision, extract_tx_id, map_app_error,
    read_entity_ref, tool_error,
};

fn disabled_tool_message(tool: &str) -> String {
    let available = visible_tools_for_model();
    if available.is_empty() {
        return format!("tool {tool} is not registered");
    }

    format!(
        "tool {tool} is not registered (available: {})",
        available.join(", ")
    )
}

pub fn execute_create(input: CreateInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("create").is_none() {
        return tool_error(
            "create",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("create"),
            false,
            FaultDomain::Policy,
        );
    }

    let result = match create_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v),
            error: None,
            meta: ToolMeta {
                tool: "create".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
        Err(err) => map_app_error("create", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_read(input: ReadInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("read").is_none() {
        return tool_error(
            "read",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("read"),
            false,
            FaultDomain::Policy,
        );
    }

    let read_set = Some(vec![read_entity_ref(&input)]);

    let result = match read_tool::run(input, &call_id) {
        Ok(v) => {
            let revision = extract_revision(&v, "revision");
            ToolResult {
                ok: true,
                data: Some(v),
                error: None,
                meta: ToolMeta {
                    tool: "read".to_string(),
                    call_id,
                    duration_ms: started.elapsed().as_millis() as u64,
                    revision_before: revision,
                    revision_after: revision,
                    tx_id: None,
                    read_set,
                    write_set: None,
                },
            }
        }
        Err(err) => map_app_error("read", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_edit(input: EditInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("edit").is_none() {
        return tool_error(
            "edit",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("edit"),
            false,
            FaultDomain::Policy,
        );
    }

    let rw_set = Some(vec![edit_entity_ref(&input)]);

    let result = match edit_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v.clone()),
            error: None,
            meta: ToolMeta {
                tool: "edit".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: extract_revision(&v, "revision_before"),
                revision_after: extract_revision(&v, "revision_after"),
                tx_id: extract_tx_id(&v),
                read_set: rw_set.clone(),
                write_set: rw_set,
            },
        },
        Err(err) => map_app_error("edit", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_ls(input: LsInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("ls").is_none() {
        return tool_error(
            "ls",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("ls"),
            false,
            FaultDomain::Policy,
        );
    }

    let result = match ls_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v),
            error: None,
            meta: ToolMeta {
                tool: "ls".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
        Err(err) => map_app_error("ls", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_delete(input: DeleteInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("delete").is_none() {
        return tool_error(
            "delete",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("delete"),
            false,
            FaultDomain::Policy,
        );
    }

    let result = match delete_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v),
            error: None,
            meta: ToolMeta {
                tool: "delete".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
        Err(err) => map_app_error("delete", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_move(input: MoveInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("move").is_none() {
        return tool_error(
            "move",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("move"),
            false,
            FaultDomain::Policy,
        );
    }

    let result = match move_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v),
            error: None,
            meta: ToolMeta {
                tool: "move".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
        Err(err) => map_app_error("move", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_grep(input: GrepInput, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("grep").is_none() {
        return tool_error(
            "grep",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("grep"),
            false,
            FaultDomain::Policy,
        );
    }

    let result = match grep_tool::run(input, &call_id) {
        Ok(v) => ToolResult {
            ok: true,
            data: Some(v),
            error: None,
            meta: ToolMeta {
                tool: "grep".to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
        Err(err) => map_app_error("grep", call_id, started, err),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_review_check(
    project_path: &str,
    input: ReviewRunInput,
    call_id: String,
) -> ToolResult<Value> {
    let started = Instant::now();
    if get_manifest("review_check").is_none() {
        return tool_error(
            "review_check",
            call_id,
            started,
            "E_TOOL_DISABLED",
            &disabled_tool_message("review_check"),
            false,
            FaultDomain::Policy,
        );
    }

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
