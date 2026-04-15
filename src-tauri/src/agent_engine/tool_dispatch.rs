//! Tool execution dispatch and input parsing for the agent loop.

use serde_json::json;

use crate::agent_tools::runtime::{
    execute_context_read, execute_context_search, execute_draft_write,
    execute_inspiration_consensus_patch, execute_inspiration_open_questions_patch,
    execute_knowledge_read, execute_knowledge_write, execute_review_check, execute_structure_edit,
    execute_workspace_map,
};
use crate::application::command_usecases::inspiration::{
    ApplyConsensusPatchInput, ApplyOpenQuestionsPatchInput,
};

use super::types::ToolCallInfo;

use crate::review::types::ReviewRunInput;

const SKILL_FIELDS: &[&str] = &["skill"];
const REVIEW_CHECK_FIELDS: &[&str] = &[
    "scope_ref",
    "target_refs",
    "review_types",
    "branch_id",
    "task_card_ref",
    "context_pack_ref",
    "effective_rules_fingerprint",
    "severity_threshold",
];
const INSPIRATION_CONSENSUS_PATCH_FIELDS: &[&str] = &[
    "state",
    "field_id",
    "operation",
    "text_value",
    "items",
    "source_turn_id",
];
const INSPIRATION_OPEN_QUESTIONS_PATCH_FIELDS: &[&str] = &[
    "questions",
    "operation",
    "question_id",
    "question",
    "importance",
];

pub fn dispatch_supports_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "workspace_map"
            | "context_read"
            | "context_search"
            | "knowledge_read"
            | "knowledge_write"
            | "draft_write"
            | "structure_edit"
            | "review_check"
            | "inspiration_consensus_patch"
            | "inspiration_open_questions_patch"
            | "askuser"
            | "skill"
            | "todowrite"
    )
}

/// Execute a single tool call by dispatching to the appropriate runtime function.
pub fn execute_tool_call(
    tc: &ToolCallInfo,
    project_path: &str,
    call_id: &str,
    _active_chapter_path: Option<&str>,
    _active_skill: Option<&str>,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    match tc.tool_name.as_str() {
        "workspace_map" => {
            execute_workspace_map(project_path, tc.args.clone(), call_id.to_string())
        }
        "context_read" => execute_context_read(project_path, tc.args.clone(), call_id.to_string()),
        "context_search" => {
            execute_context_search(project_path, tc.args.clone(), call_id.to_string())
        }
        "knowledge_read" => {
            execute_knowledge_read(project_path, tc.args.clone(), call_id.to_string())
        }
        "knowledge_write" => {
            execute_knowledge_write(project_path, tc.args.clone(), call_id.to_string())
        }
        "draft_write" => execute_draft_write(project_path, tc.args.clone(), call_id.to_string()),
        "structure_edit" => {
            execute_structure_edit(project_path, tc.args.clone(), call_id.to_string())
        }
        "review_check" => {
            let input = parse_review_check_input(&tc.args);
            match input {
                Ok(i) => execute_review_check(project_path, i, call_id.to_string()),
                Err(e) => tool_parse_error("review_check", call_id, &e),
            }
        }
        "inspiration_consensus_patch" => {
            let input = parse_inspiration_consensus_patch_input(&tc.args);
            match input {
                Ok(i) => execute_inspiration_consensus_patch(
                    serde_json::to_value(i).expect("consensus patch input should serialize"),
                    call_id.to_string(),
                ),
                Err(e) => tool_parse_error("inspiration_consensus_patch", call_id, &e),
            }
        }
        "inspiration_open_questions_patch" => {
            let input = parse_inspiration_open_questions_patch_input(&tc.args);
            match input {
                Ok(i) => execute_inspiration_open_questions_patch(
                    serde_json::to_value(i).expect("open questions patch input should serialize"),
                    call_id.to_string(),
                ),
                Err(e) => tool_parse_error("inspiration_open_questions_patch", call_id, &e),
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
        other => {
            tracing::warn!(target: "agent_engine", tool = other, "unknown tool");
            tool_parse_error(other, call_id, &format!("unknown tool: {other}"))
        }
    }
}

fn parse_review_check_input(args: &serde_json::Value) -> Result<ReviewRunInput, String> {
    reject_unknown_fields(args, REVIEW_CHECK_FIELDS, "review_check")?;

    serde_json::from_value::<ReviewRunInput>(args.clone())
        .map_err(|error| format!("review_check args: {error}"))
}

fn parse_inspiration_consensus_patch_input(
    args: &serde_json::Value,
) -> Result<ApplyConsensusPatchInput, String> {
    reject_unknown_fields(
        args,
        INSPIRATION_CONSENSUS_PATCH_FIELDS,
        "inspiration_consensus_patch",
    )?;

    serde_json::from_value::<ApplyConsensusPatchInput>(args.clone())
        .map_err(|error| format!("inspiration_consensus_patch args: {error}"))
}

fn parse_inspiration_open_questions_patch_input(
    args: &serde_json::Value,
) -> Result<ApplyOpenQuestionsPatchInput, String> {
    reject_unknown_fields(
        args,
        INSPIRATION_OPEN_QUESTIONS_PATCH_FIELDS,
        "inspiration_open_questions_patch",
    )?;

    serde_json::from_value::<ApplyOpenQuestionsPatchInput>(args.clone())
        .map_err(|error| format!("inspiration_open_questions_patch args: {error}"))
}

fn reject_unknown_fields(
    args: &serde_json::Value,
    allowed_fields: &[&str],
    tool_name: &str,
) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !allowed_fields.contains(&key.as_str()) {
            return Err(format!("{tool_name} args: unknown field '{key}'"));
        }
    }

    Ok(())
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

    use serde_json::json;

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

    #[test]
    fn parse_review_check_accepts_full_payload() {
        let args = json!({
            "scope_ref": "chapter:manuscripts/vol_1/ch_1.json",
            "target_refs": ["manuscripts/vol_1/ch_1.json"],
            "review_types": ["word_count", "continuity"],
            "branch_id": "branch/main",
            "task_card_ref": "task:123",
            "context_pack_ref": "ctx:abc",
            "effective_rules_fingerprint": "rules:v1",
            "severity_threshold": "warn"
        });

        let input = super::parse_review_check_input(&args).expect("review_check parsed");
        assert_eq!(input.scope_ref, "chapter:manuscripts/vol_1/ch_1.json");
        assert_eq!(input.target_refs, vec!["manuscripts/vol_1/ch_1.json"]);
        assert_eq!(input.review_types.len(), 2);
        assert_eq!(input.severity_threshold.as_deref(), Some("warn"));
    }

    #[test]
    fn parse_review_check_rejects_unknown_fields() {
        let args = json!({
            "scope_ref": "chapter:manuscripts/vol_1/ch_1.json",
            "target_refs": ["manuscripts/vol_1/ch_1.json"],
            "unexpected": true
        });

        let err = super::parse_review_check_input(&args).expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn review_check_parser_allowlist_matches_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("review_check", &context)
            .expect("review_check schema should exist");
        let schema_fields: BTreeSet<String> = schema
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("schema properties")
            .keys()
            .cloned()
            .collect();
        let parser_fields: BTreeSet<String> = super::REVIEW_CHECK_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect();

        assert_eq!(schema_fields, parser_fields);
    }

    #[test]
    fn execute_tool_call_rejects_invalid_askuser_questions_at_parse_boundary() {
        let tc = crate::agent_engine::types::ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "askuser".to_string(),
            args: json!({
                "questions": [{
                    "question": "Pick one",
                    "topic": "style",
                    "options": ["Only one"]
                }]
            }),
        };

        let result = super::execute_tool_call(&tc, "D:/p", "tool_askuser_invalid", None, None);
        assert!(!result.ok);
        assert_eq!(
            result.error.as_ref().map(|error| error.code.as_str()),
            Some("E_TOOL_SCHEMA_INVALID")
        );
        assert_eq!(
            result.error.as_ref().map(|error| error.message.as_str()),
            Some("askuser questions[0].options must contain between 2 and 4 items")
        );
    }
}
