//! Tool result formatters, trace builders, and askuser extraction helpers.
//!
//! Extracted from tool_scheduler.rs to keep each module focused on a single responsibility.

use serde_json::{json, Value};

use crate::agent_tools::contracts::ToolResult;

use super::messages::AgentMessage;
use super::tool_errors::sanitize_error_message;
use super::types::ToolCallInfo;

mod askuser;
mod common;
mod context_tools;
mod discovery_tools;
mod writing_tools;

pub(crate) use askuser::{
    extract_askuser_questions, extract_questionnaire, is_askuser_call, validate_askuser_args,
};

use common::{build_result_data_preview, build_result_error, build_result_refs, truncate_to_chars};
use context_tools::{
    format_character_sheet_result, format_outline_result, format_search_knowledge_result,
};
use discovery_tools::{format_grep_result, format_ls_result};
use writing_tools::{
    format_create_result, format_delete_result, format_edit_result, format_move_result,
    format_read_result,
};

/// Maximum characters in a single tool result sent to the LLM.
/// Chosen at 16K (vs Droid's 4K) because novel content is naturally longer than code,
/// but still prevents unbounded grep/outline/character_sheet output from blowing up context.
const MAX_TOOL_OUTPUT_CHARS: usize = 16_000;

pub(crate) fn build_tool_message(
    tc: &ToolCallInfo,
    result: &ToolResult<serde_json::Value>,
) -> AgentMessage {
    let mut content = if result.ok {
        format_success_content(tc, result)
    } else {
        format_error_content(tc, result)
    };

    let char_count = content.chars().count();
    if char_count > MAX_TOOL_OUTPUT_CHARS {
        content = truncate_to_chars(&content, MAX_TOOL_OUTPUT_CHARS);
        content.push_str(&format!(
            "\n\n[output truncated: {} of {} chars shown. Use more specific queries or pagination to reduce output.]",
            MAX_TOOL_OUTPUT_CHARS, char_count
        ));
    }

    AgentMessage::tool_result(
        tc.llm_call_id.clone(),
        Some(tc.tool_name.clone()),
        content,
        !result.ok,
    )
}

pub(crate) fn build_tool_trace(tool_name: &str, result: &ToolResult<serde_json::Value>) -> Value {
    let mut trace = json!({
        "schema_version": 2,
        "stage": "result",
        "meta": {
            "tool": result.meta.tool.as_str(),
            "call_id": result.meta.call_id.as_str(),
            "duration_ms": result.meta.duration_ms,
            "revision_before": result.meta.revision_before,
            "revision_after": result.meta.revision_after,
            "tx_id": result.meta.tx_id.as_deref(),
        },
        "result": {
            "ok": result.ok,
            "preview": build_result_data_preview(tool_name, result),
            "error": build_result_error(result),
        },
        "refs": build_result_refs(tool_name, result),
    });

    if let Some(root) = trace.as_object_mut() {
        if matches!(root.get("refs"), Some(Value::Null)) {
            root.remove("refs");
        }
    }

    trace
}

fn format_success_content(tc: &ToolCallInfo, result: &ToolResult<serde_json::Value>) -> String {
    match tc.tool_name.as_str() {
        "read" => format_read_result(result.data.as_ref()),
        "edit" => format_edit_result(result.data.as_ref()),
        "create" => format_create_result(result.data.as_ref()),
        "delete" => format_delete_result(result.data.as_ref()),
        "move" => format_move_result(result.data.as_ref()),
        "ls" => format_ls_result(result.data.as_ref(), &tc.args),
        "grep" => format_grep_result(result.data.as_ref(), &tc.args),
        "outline" => format_outline_result(result.data.as_ref(), &tc.args),
        "character_sheet" => format_character_sheet_result(result.data.as_ref(), &tc.args),
        "search_knowledge" => format_search_knowledge_result(result.data.as_ref(), &tc.args),
        _ => serde_json::to_string(&result.data).unwrap_or_else(|_| "null".to_string()),
    }
}

fn format_error_content(_tc: &ToolCallInfo, result: &ToolResult<serde_json::Value>) -> String {
    let err = match &result.error {
        Some(e) => e,
        None => return "unknown error".to_string(),
    };

    let recovery = match err.code.as_str() {
        "E_VC_CONFLICT_REVISION" => {
            " Recovery: Re-read the chapter to get the latest revision, then retry."
        }
        "E_TOOL_NOT_FOUND" | "E_TOOL_PATH_NOT_FOUND" => {
            " Recovery: Use ls() to check available paths."
        }
        "E_TOOL_AMBIGUOUS_MATCH" => {
            " Recovery: Read the latest snapshot and retry with precise edit ops on block ids."
        }
        "E_TOOL_NO_MATCH" => {
            " Recovery: The target blocks were not found. Re-read the chapter snapshot and retry."
        }
        "E_TOOL_SCHEMA_INVALID" => {
            " Recovery: Check the parameter types and required fields in the tool schema."
        }
        "E_TOOL_TIMEOUT" => {
            " Recovery: The tool call took too long. Retry with a simpler query or smaller scope."
        }
        _ => "",
    };

    format!(
        "[error code={}] {}{}",
        err.code,
        sanitize_error_message(&err.message),
        recovery
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_questionnaire() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "askuser".to_string(),
            args: json!({"questionnaire": "1. [question] A\n[topic] T\n[option] X\n[option] Y"}),
        };
        assert!(extract_questionnaire(&tc).is_some());
    }

    #[test]
    fn test_extract_askuser_questions_structured() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "askuser".to_string(),
            args: json!({
                "questions": [
                    {
                        "question": "What writing style?",
                        "topic": "style",
                        "options": ["Formal", "Casual"]
                    }
                ]
            }),
        };
        let result = extract_askuser_questions(&tc);
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_extract_askuser_questions_rejects_single_option() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "askuser".to_string(),
            args: json!({
                "questions": [{
                    "question": "Q?",
                    "topic": "t",
                    "options": ["Only one"]
                }]
            }),
        };
        assert!(extract_askuser_questions(&tc).is_none());
    }

    #[test]
    fn test_extract_askuser_questions_rejects_too_many() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "askuser".to_string(),
            args: json!({
                "questions": [
                    { "question": "Q1", "topic": "t1", "options": ["A", "B"] },
                    { "question": "Q2", "topic": "t2", "options": ["A", "B"] },
                    { "question": "Q3", "topic": "t3", "options": ["A", "B"] },
                    { "question": "Q4", "topic": "t4", "options": ["A", "B"] },
                    { "question": "Q5", "topic": "t5", "options": ["A", "B"] }
                ]
            }),
        };
        assert!(extract_askuser_questions(&tc).is_none());
    }

    #[test]
    fn test_is_askuser_call_requires_canonical_name() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: ["ask", "user"].join("_"),
            args: json!({}),
        };
        assert!(!is_askuser_call(&tc));
    }

    #[test]
    fn test_build_tool_message_read_starts_with_revision_hint() {
        let tc = ToolCallInfo {
            llm_call_id: "call_read_1".to_string(),
            tool_name: "read".to_string(),
            args: json!({
                "path": "vol_1/ch_1.json",
                "view": "markdown"
            }),
        };

        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "path": "vol_1/ch_1.json",
                "kind": "domain_object",
                "revision": 3,
                "hash": "abc",
                "content": "# Chapter\nBody"
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "read".to_string(),
                call_id: "tool_1".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };

        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(content.starts_with(
            "[revision=3] Use this revision number as base_revision when calling edit."
        ));
        assert!(content.contains("[path=vol_1/ch_1.json hash=abc]"));
        assert!(content.contains("# Chapter\nBody"));
    }

    #[test]
    fn test_build_tool_message_read_truncated_includes_pagination_hint() {
        let tc = ToolCallInfo {
            llm_call_id: "call_read_2".to_string(),
            tool_name: "read".to_string(),
            args: json!({ "path": "vol_1/ch_1.json", "view": "markdown" }),
        };

        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "path": "vol_1/ch_1.json",
                "kind": "domain_object",
                "revision": 5,
                "hash": "def",
                "content": "First page...",
                "truncated": {
                    "total_chars": 50000,
                    "returned_offset": 0,
                    "returned_chars": 10000,
                    "next_offset": 10000
                }
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "read".to_string(),
                call_id: "tool_2".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };

        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(content.contains("[truncated: returned 10000 of 50000 chars. Continue with another read call from offset 10000 if needed.]"));
    }

    #[test]
    fn test_build_tool_trace_todowrite_contains_todo_state() {
        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "todo_state": {
                    "items": [
                        {"status": "in_progress", "text": "Implement trace"},
                        {"status": "pending", "text": "Add tests"}
                    ],
                    "last_updated_at": 123,
                    "source_call_id": "tool_abc"
                }
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "todowrite".to_string(),
                call_id: "tool_abc".to_string(),
                duration_ms: 8,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };

        let trace = build_tool_trace("todowrite", &result);
        assert_eq!(
            trace.get("schema_version").and_then(|v| v.as_i64()),
            Some(2)
        );
        assert_eq!(trace.get("stage").and_then(|v| v.as_str()), Some("result"));
        assert_eq!(
            trace
                .get("meta")
                .and_then(|v| v.get("duration_ms"))
                .and_then(|v| v.as_u64()),
            Some(8)
        );

        let todo_state = trace
            .get("result")
            .and_then(|v| v.get("preview"))
            .and_then(|v| v.get("todo_state"))
            .expect("todo_state should exist");

        let items = todo_state
            .get("items")
            .and_then(|v| v.as_array())
            .expect("todo items array should exist");
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get("status").and_then(|v| v.as_str()),
            Some("in_progress")
        );
    }

    #[test]
    fn test_build_tool_trace_read_uses_compact_preview_without_content_body() {
        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "path": "vol_1/ch_1.json",
                "kind": "domain_object",
                "revision": 9,
                "hash": "h123",
                "content": "# chapter\nvery long body"
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "read".to_string(),
                call_id: "tool_read".to_string(),
                duration_ms: 3,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };

        let trace = build_tool_trace("read", &result);
        let preview = trace
            .get("result")
            .and_then(|v| v.get("preview"))
            .expect("result.preview should exist");

        assert_eq!(
            preview.get("path").and_then(|v| v.as_str()),
            Some("vol_1/ch_1.json")
        );
        assert_eq!(preview.get("revision").and_then(|v| v.as_i64()), Some(9));
        assert!(
            preview.get("content").is_none(),
            "trace should not include full content body"
        );
        assert_eq!(
            preview.get("content_chars").and_then(|v| v.as_u64()),
            Some(24)
        );
    }

    #[test]
    fn test_format_edit_result_preview() {
        let data = json!({
            "mode": "preview",
            "accepted": true,
            "path": "vol_1/ch_1.json",
            "revision_before": 3,
            "revision_after": 3,
            "diagnostics": [],
            "diff_summary": [{"operation": "replace", "description": "paragraph 2"}],
            "hash_after": "xyz"
        });
        let result = format_edit_result(Some(&data));
        assert!(result.starts_with("[preview path=vol_1/ch_1.json revision=3→3]"));
        assert!(result.contains("replace paragraph 2"));
        assert!(result.contains(
            "Preview only — call edit with dry_run=false if you want to commit this change."
        ));
    }

    #[test]
    fn test_format_edit_result_commit() {
        let data = json!({
            "mode": "commit",
            "accepted": true,
            "path": "vol_1/ch_1.json",
            "revision_before": 3,
            "revision_after": 4,
            "diagnostics": [],
            "diff_summary": [],
            "tx_id": "tx_abc",
            "hash_after": "xyz"
        });
        let result = format_edit_result(Some(&data));
        assert!(result.starts_with("[committed path=vol_1/ch_1.json revision=3→4 tx=tx_abc]"));
        assert!(result.contains("Edit applied successfully."));
    }

    #[test]
    fn test_format_create_result() {
        let data = json!({
            "created_kind": "file",
            "path": "vol_1/ch_013.json",
            "id": "abc",
            "revision_after": 1,
            "created_at": 1234567890
        });
        let result = format_create_result(Some(&data));
        assert!(result.starts_with("[created path=vol_1/ch_013.json revision=1]"));
        assert!(result.contains("file created successfully."));
    }

    #[test]
    fn test_format_delete_result_preview() {
        let data = json!({
            "mode": "preview",
            "kind": "volume",
            "path": "vol_1",
            "impact": { "chapter_count": 3 }
        });
        let result = format_delete_result(Some(&data));
        assert!(result.starts_with("[delete preview kind=volume path=vol_1"));
        assert!(result.contains("chapter_count:3"));
    }

    #[test]
    fn test_format_move_result_commit() {
        let data = json!({
            "mode": "commit",
            "chapter_path": "vol_1/ch_1.json",
            "target_volume_path": "vol_2",
            "target_index": 1,
            "new_chapter_path": "vol_2/ch_1.json"
        });
        let result = format_move_result(Some(&data));
        assert!(result.starts_with("[move committed chapter=vol_1/ch_1.json"));
        assert!(result.contains("new_path=vol_2/ch_1.json"));
    }

    #[test]
    fn test_format_ls_result_with_items() {
        let data = json!({
            "cwd": ".",
            "items": [
                {"kind": "folder", "name": "Vol 1", "path": "vol_1", "child_count": 5, "revision": 0},
                {"kind": "folder", "name": "Vol 2", "path": "vol_2", "child_count": 3, "revision": 0}
            ]
        });
        let args = json!({"offset": 0, "limit": 30});
        let result = format_ls_result(Some(&data), &args);
        assert!(result.starts_with("[ls path=. total=2 showing=1-2]"));
        assert!(result.contains("Vol 1/ (vol_1)"));
    }

    #[test]
    fn test_format_ls_result_truncated_with_pagination_hint() {
        let mut items = Vec::new();
        for i in 0..35 {
            items.push(json!({
                "kind": "folder",
                "name": format!("Vol {}", i + 1),
                "path": format!("vol_{}", i + 1),
                "child_count": i,
                "revision": 0
            }));
        }

        let data = json!({ "cwd": ".", "items": items });
        let args = json!({"offset": 0, "limit": 30});
        let result = format_ls_result(Some(&data), &args);

        assert!(result.starts_with("[ls path=. total=35 showing=1-30]"));
        assert!(result.contains("[truncated: 5 more items."));
        assert!(result.contains("offset=30, limit=30"));
    }

    #[test]
    fn test_format_grep_result() {
        let data = json!({
            "hits": [
                {"path": "vol_1/ch_003.json", "score": 0.95, "snippet": "the dragon emerged"}
            ]
        });
        let args = json!({"query": "dragon", "mode": "keyword"});
        let result = format_grep_result(Some(&data), &args);
        assert!(result.starts_with("[grep query=\"dragon\" mode=keyword hits=1]"));
        assert!(result.contains("vol_1/ch_003.json (score=0.95)"));
        assert!(result.contains("\"the dragon emerged\""));
    }

    #[test]
    fn test_format_outline_result() {
        let data = json!({"outline": "# Outline\n- A\n- B"});
        let args = json!({"volume_path": "vol_1"});
        let result = format_outline_result(Some(&data), &args);
        assert!(result.starts_with("[outline scope=vol_1]"));
        assert!(result.contains("# Outline"));
    }

    #[test]
    fn test_format_character_sheet_result() {
        let data = json!({"result": "Name: Li Wei"});
        let args = json!({"name": "Li Wei"});
        let result = format_character_sheet_result(Some(&data), &args);
        assert!(result.starts_with("[character_sheet name=\"Li Wei\"]"));
        assert!(result.contains("Name: Li Wei"));
    }

    #[test]
    fn test_format_search_knowledge_result() {
        let data = json!({"result": "Found in world.md"});
        let args = json!({"query": "dragon", "top_k": 3});
        let result = format_search_knowledge_result(Some(&data), &args);
        assert!(result.starts_with("[search_knowledge query=\"dragon\" top_k=3]"));
        assert!(result.contains("Found in world.md"));
    }

    #[test]
    fn test_format_error_with_recovery_conflict() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "edit".to_string(),
            args: json!({}),
        };
        let result = ToolResult {
            ok: false,
            data: None,
            error: Some(crate::agent_tools::contracts::ToolError {
                code: "E_VC_CONFLICT_REVISION".to_string(),
                message: "revision mismatch".to_string(),
                retryable: true,
                fault_domain: crate::agent_tools::contracts::FaultDomain::Vc,
                details: None,
            }),
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "edit".to_string(),
                call_id: "tool_1".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };
        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(content.contains("[error code=E_VC_CONFLICT_REVISION]"));
        assert!(content.contains("Recovery:"));
        assert!(content.contains("Re-read the chapter"));
    }

    #[test]
    fn test_format_error_timeout_has_recovery_hint() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "grep".to_string(),
            args: json!({}),
        };
        let timeout = std::time::Duration::from_millis(60_000);
        let result = super::super::tool_errors::tool_timeout_error("grep", "call_1", timeout);
        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(content.contains("[error code=E_TOOL_TIMEOUT]"));
        assert!(content.contains("Recovery:"));
    }

    #[test]
    fn test_build_tool_message_truncates_oversized_output() {
        // Create content larger than MAX_TOOL_OUTPUT_CHARS (16,000)
        let big_text = "a".repeat(20_000);
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "outline".to_string(),
            args: json!({"volume_path": "vol_1"}),
        };
        let result = ToolResult {
            ok: true,
            data: Some(json!({"outline": big_text})),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "outline".to_string(),
                call_id: "tool_1".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };
        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(content.contains("[output truncated: 16000 of"));
        assert!(content.contains("20")); // contains the original char count
    }

    #[test]
    fn test_build_tool_message_preserves_normal_output() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "outline".to_string(),
            args: json!({"volume_path": "vol_1"}),
        };
        let result = ToolResult {
            ok: true,
            data: Some(json!({"outline": "short outline"})),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "outline".to_string(),
                call_id: "tool_1".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };
        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(!content.contains("[output truncated"));
        assert!(content.contains("short outline"));
    }

    #[test]
    fn test_truncate_to_chars_cjk_safe() {
        let text = "你好世界测试数据";
        let truncated = truncate_to_chars(text, 4);
        assert_eq!(truncated, "你好世界");
    }

    #[test]
    fn test_truncate_to_chars_no_op_when_short() {
        let text = "hello";
        let truncated = truncate_to_chars(text, 100);
        assert_eq!(truncated, "hello");
    }

    #[test]
    fn test_format_error_content_sanitizes_paths() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "read".to_string(),
            args: json!({}),
        };
        let result = ToolResult {
            ok: false,
            data: None,
            error: Some(crate::agent_tools::contracts::ToolError {
                code: "E_TOOL_NOT_FOUND".to_string(),
                message: "file not found at D:\\Users\\admin\\project\\manuscripts\\ch1.json"
                    .to_string(),
                retryable: false,
                fault_domain: crate::agent_tools::contracts::FaultDomain::Io,
                details: None,
            }),
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "read".to_string(),
                call_id: "tool_1".to_string(),
                duration_ms: 1,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };
        let msg = build_tool_message(&tc, &result);
        let content = match &msg.blocks[0] {
            crate::agent_engine::messages::ContentBlock::ToolResult { content, .. } => content,
            _ => panic!("expected tool_result"),
        };
        assert!(
            !content.contains("D:\\Users\\admin"),
            "absolute path should be sanitized"
        );
        assert!(content.contains("[path]"));
    }
}
