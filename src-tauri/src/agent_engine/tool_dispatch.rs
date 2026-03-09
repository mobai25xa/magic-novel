//! Tool execution dispatch and input parsing for the agent loop.

use serde_json::json;

use crate::agent_tools::runtime::{
    execute_create, execute_delete, execute_edit, execute_grep, execute_ls, execute_move,
    execute_read,
};

use super::types::ToolCallInfo;

mod context_tools;
mod parse;

use context_tools::{
    execute_character_sheet_tool, execute_outline_tool, execute_search_knowledge_tool,
};
use parse::{
    parse_create_input, parse_delete_input, parse_edit_input, parse_grep_input, parse_ls_input,
    parse_move_input, parse_read_input,
};

const SKILL_FIELDS: &[&str] = &["skill"];

pub fn dispatch_supports_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "read"
            | "edit"
            | "create"
            | "delete"
            | "move"
            | "ls"
            | "grep"
            | "askuser"
            | "skill"
            | "todowrite"
            | "outline"
            | "character_sheet"
            | "search_knowledge"
    )
}

/// Execute a single tool call by dispatching to the appropriate runtime function.
pub fn execute_tool_call(
    tc: &ToolCallInfo,
    project_path: &str,
    call_id: &str,
    active_chapter_path: Option<&str>,
    _active_skill: Option<&str>,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    match tc.tool_name.as_str() {
        "read" => {
            let input = parse_read_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_read(i, call_id.to_string()),
                Err(e) => tool_parse_error("read", call_id, &e),
            }
        }
        "edit" => {
            let input = parse_edit_input(&tc.args, project_path, active_chapter_path);
            match input {
                Ok(i) => execute_edit(i, call_id.to_string()),
                Err(e) => tool_parse_error("edit", call_id, &e),
            }
        }
        "create" => {
            let input = parse_create_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_create(i, call_id.to_string()),
                Err(e) => tool_parse_error("create", call_id, &e),
            }
        }
        "delete" => {
            let input = parse_delete_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_delete(i, call_id.to_string()),
                Err(e) => tool_parse_error("delete", call_id, &e),
            }
        }
        "move" => {
            let input = parse_move_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_move(i, call_id.to_string()),
                Err(e) => tool_parse_error("move", call_id, &e),
            }
        }
        "ls" => {
            let input = parse_ls_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_ls(i, call_id.to_string()),
                Err(e) => tool_parse_error("ls", call_id, &e),
            }
        }
        "grep" => {
            let input = parse_grep_input(&tc.args, project_path);
            match input {
                Ok(i) => execute_grep(i, call_id.to_string()),
                Err(e) => tool_parse_error("grep", call_id, &e),
            }
        }
        "askuser" => {
            if let Err(error) = super::tool_formatters::validate_askuser_args(&tc.args) {
                return tool_parse_error("askuser", call_id, &error);
            }
            tool_parse_error(
                "askuser",
                call_id,
                "askuser is interactive-only and must be suspended before runtime execution",
            )
        }
        "skill" => {
            if let Err(error) = validate_skill_args(&tc.args) {
                return tool_parse_error("skill", call_id, &error);
            }

            let skill_name = tc
                .args
                .get("skill")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if skill_name.is_empty() {
                return tool_parse_error("skill", call_id, "skill must be a non-empty string");
            }

            let skill = match super::skills::get_skill_by_name(skill_name) {
                Some(s) => s,
                None => {
                    return tool_parse_error(
                        "skill",
                        call_id,
                        &format!("skill not found: {skill_name}"),
                    )
                }
            };

            if !skill.enabled {
                return tool_parse_error(
                    "skill",
                    call_id,
                    &format!("skill disabled: {skill_name}"),
                );
            }

            use crate::agent_tools::contracts::{ToolMeta, ToolResult};
            ToolResult {
                ok: true,
                data: Some(json!({
                    "ok": true,
                    "summary": format!("skill enabled: {}", skill.display_name),
                    "skill_name": skill.name,
                })),
                error: None,
                meta: ToolMeta {
                    tool: "skill".to_string(),
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
        "todowrite" => match super::todowrite::parse_todo_input(&tc.args, call_id) {
            Ok(state) => {
                use crate::agent_tools::contracts::{ToolMeta, ToolResult};
                ToolResult {
                    ok: true,
                    data: Some(json!({
                        "updated": true,
                        "item_count": state.items.len(),
                        "todo_state": state,
                    })),
                    error: None,
                    meta: ToolMeta {
                        tool: "todowrite".to_string(),
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
            Err(e) => tool_parse_error("todowrite", call_id, &e),
        },
        "outline" => execute_outline_tool(tc, project_path, call_id),
        "character_sheet" => execute_character_sheet_tool(tc, project_path, call_id),
        "search_knowledge" => execute_search_knowledge_tool(tc, project_path, call_id),
        other => {
            tracing::warn!(target: "agent_engine", tool = other, "unknown tool");
            tool_parse_error(other, call_id, &format!("unknown tool: {other}"))
        }
    }
}

fn validate_skill_args(args: &serde_json::Value) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !SKILL_FIELDS.contains(&key.as_str()) {
            return Err(format!("skill args: unknown field '{key}'"));
        }
    }

    Ok(())
}

pub(super) fn tool_parse_error(
    tool: &str,
    call_id: &str,
    msg: &str,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_TOOL_SCHEMA_INVALID".to_string(),
            message: msg.to_string(),
            retryable: false,
            fault_domain: FaultDomain::Validation,
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn skill_parser_allowlist_matches_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("skill", &context)
            .expect("skill schema should exist");
        let schema_fields: BTreeSet<String> = schema
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("schema properties")
            .keys()
            .cloned()
            .collect();
        let parser_fields: BTreeSet<String> = super::SKILL_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect();

        assert_eq!(schema_fields, parser_fields);
    }
}
