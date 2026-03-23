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
mod writing_tools;

pub(crate) use askuser::{
    extract_askuser_questions, extract_questionnaire, is_askuser_call, validate_askuser_args,
};

use common::{build_result_data_preview, build_result_error, build_result_refs, truncate_to_chars};
use writing_tools::{
    format_draft_write_result, format_knowledge_write_result, format_structure_edit_result,
};

use context_tools::{
    format_context_read_result, format_context_search_result, format_knowledge_read_result,
    format_workspace_map_result,
};

/// Maximum characters in a single tool result sent to the LLM.
/// Chosen at 16K (vs Droid's 4K) because novel content is naturally longer than code,
/// but still prevents unbounded workspace_map/context_read output from blowing up context.
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
        "workspace_map" => format_workspace_map_result(result.data.as_ref(), &tc.args),
        "context_read" => format_context_read_result(result.data.as_ref(), &tc.args),
        "context_search" => format_context_search_result(result.data.as_ref(), &tc.args),
        "knowledge_read" => format_knowledge_read_result(result.data.as_ref(), &tc.args),
        "knowledge_write" => format_knowledge_write_result(result.data.as_ref(), &tc.args),
        "draft_write" => format_draft_write_result(result.data.as_ref(), &tc.args),
        "structure_edit" => format_structure_edit_result(result.data.as_ref(), &tc.args),
        _ => serde_json::to_string(&result.data).unwrap_or_else(|_| "null".to_string()),
    }
}

fn format_error_content(_tc: &ToolCallInfo, result: &ToolResult<serde_json::Value>) -> String {
    let err = match &result.error {
        Some(e) => e,
        None => return "unknown error".to_string(),
    };

    let recovery = match err.code.as_str() {
        "E_CONFLICT" | "E_VC_CONFLICT_REVISION" => " Recovery: Run context_read to fetch the latest target state, then retry with the same idempotency_key when applicable.",
        "E_TOOL_NOT_FOUND" => {
            " Recovery: Use only the exposed tool set for this turn, and verify tool names in the registry."
        }
        "E_TOOL_SCHEMA_INVALID" => {
            " Recovery: Check the parameter types and required fields in the tool schema."
        }
        "E_TOOL_NOT_ALLOWED" => {
            " Recovery: Use only the tools exposed for this turn (see allowed_tools in error details)."
        }
        "E_TOOL_TIMEOUT" => {
            " Recovery: The tool call took too long. Retry with a simpler query or smaller scope."
        }
        "E_REF_NOT_FOUND" => " Recovery: Use workspace_map to locate the correct ref, then retry.",
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
    fn test_build_tool_message_context_read_includes_content() {
        let tc = ToolCallInfo {
            llm_call_id: "call_ctx_1".to_string(),
            tool_name: "context_read".to_string(),
            args: json!({ "target_ref": "chapter:manuscripts/vol_1/ch_1.json" }),
        };

        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "ref": "chapter:manuscripts/vol_1/ch_1.json",
                "kind": "chapter",
                "content": "Chapter: T\\n\\nBody"
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "context_read".to_string(),
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
            content.contains("[context_read ref=chapter:manuscripts/vol_1/ch_1.json kind=chapter]")
        );
        assert!(content.contains("Chapter: T"));
        assert!(content.contains("Body"));
    }

    #[test]
    fn test_build_tool_message_workspace_map_truncated_includes_cursor_hint() {
        let tc = ToolCallInfo {
            llm_call_id: "call_map_1".to_string(),
            tool_name: "workspace_map".to_string(),
            args: json!({ "scope": "book", "depth": 2 }),
        };

        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "tree": [
                    { "ref": "volume:manuscripts/vol_1", "kind": "volume", "title": "Vol 1" },
                    { "ref": "chapter:manuscripts/vol_1/ch_1.json", "kind": "chapter", "title": "Ch 1" }
                ],
                "summary": "book: volumes=1, chapters=1, truncated=true",
                "truncated": true,
                "next_cursor": "2"
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "workspace_map".to_string(),
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
        assert!(content.contains("summary: book: volumes=1"));
        assert!(content.contains("next_cursor=2"));
        assert!(content.contains("workspace_map(cursor=\"2\")"));
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
    fn test_build_tool_trace_context_read_uses_compact_preview_without_content_body() {
        let content = "Chapter: T\\n\\nBody";
        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "ref": "chapter:manuscripts/vol_1/ch_1.json",
                "kind": "chapter",
                "content": content
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "context_read".to_string(),
                call_id: "tool_ctx".to_string(),
                duration_ms: 3,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        };

        let trace = build_tool_trace("context_read", &result);
        let preview = trace
            .get("result")
            .and_then(|v| v.get("preview"))
            .expect("result.preview should exist");

        assert_eq!(
            preview.get("ref").and_then(|v| v.as_str()),
            Some("chapter:manuscripts/vol_1/ch_1.json")
        );
        assert_eq!(
            preview.get("kind").and_then(|v| v.as_str()),
            Some("chapter")
        );
        assert!(
            preview.get("content").is_none(),
            "trace should not include full content body"
        );
        assert_eq!(
            preview.get("content_chars").and_then(|v| v.as_u64()),
            Some(content.chars().count() as u64)
        );
    }

    #[test]
    fn test_format_draft_write_result_preview() {
        let data = json!({
            "accepted": true,
            "mode": "preview",
            "diff_summary": ["write_mode: draft", "length_target: 1200"],
            "snippet_after": "Chapter: T\\n\\nBody…"
        });
        let args = json!({
            "target_ref": "chapter:manuscripts/vol_1/ch_1.json",
            "write_mode": "draft"
        });
        let result = format_draft_write_result(Some(&data), &args);
        assert!(result.contains("[draft_write mode=preview accepted=true write_mode=draft target_ref=chapter:manuscripts/vol_1/ch_1.json]"));
        assert!(result.contains("diff_summary:"));
        assert!(result.contains("- write_mode: draft"));
        assert!(result.contains("snippet_after:"));
    }

    #[test]
    fn test_format_structure_edit_result_commit() {
        let data = json!({
            "accepted": true,
            "mode": "commit",
            "impact_summary": ["created chapter:manuscripts/vol_1/ch_new.json"],
            "refs": { "after": "chapter:manuscripts/vol_1/ch_new.json" },
            "tx_id": "tx_1"
        });
        let args = json!({
            "op": "create",
            "node_type": "chapter"
        });
        let result = format_structure_edit_result(Some(&data), &args);
        assert!(result
            .contains("[structure_edit mode=commit accepted=true op=create node_type=chapter]"));
        assert!(result.contains("tx=tx_1"));
        assert!(result.contains("impact_summary:"));
        assert!(result.contains("created chapter:manuscripts/vol_1/ch_new.json"));
    }

    #[test]
    fn test_format_context_search_result_includes_hits() {
        let data = json!({
            "hits": [
                { "ref": "chapter:manuscripts/vol_1/ch_003.json", "score": 0.95, "snippet": "the dragon emerged" }
            ],
            "mode": "keyword"
        });
        let args = json!({"query": "dragon", "corpus": "draft", "mode": "keyword"});
        let result = format_context_search_result(Some(&data), &args);
        assert!(result
            .starts_with("[context_search query=\"dragon\" corpus=draft mode=keyword hits=1]"));
        assert!(result.contains("chapter:manuscripts/vol_1/ch_003.json"));
        assert!(result.contains("\"the dragon emerged\""));
    }

    #[test]
    fn test_format_knowledge_read_result_full_view_includes_snippet() {
        let data = json!({
            "items": [
                {
                    "ref": "knowledge:.magic_novel/terms/foo.md",
                    "title": "Foo",
                    "summary": "foo summary",
                    "snippet": "foo snippet"
                }
            ],
            "truncated": true
        });
        let args = json!({
            "view_mode": "full",
            "knowledge_type": "term",
            "query": "foo"
        });
        let result = format_knowledge_read_result(Some(&data), &args);
        assert!(result.contains("[knowledge_read view_mode=full items=1]"));
        assert!(result.contains("knowledge_type=term"));
        assert!(result.contains("query=\"foo\""));
        assert!(result.contains("knowledge:.magic_novel/terms/foo.md"));
        assert!(result.contains("snippet: foo snippet"));
    }

    #[test]
    fn test_format_knowledge_write_result_summary() {
        let data = json!({
            "delta_id": "kdelta_1",
            "status": "proposed"
        });
        let args = json!({
            "changes": [
                { "target_ref": "knowledge:.magic_novel/terms/foo.json", "kind": "add", "fields": {} }
            ]
        });
        let result = format_knowledge_write_result(Some(&data), &args);
        assert!(result.contains("[knowledge_write status=proposed delta_id=kdelta_1 changes=1]"));
    }

    #[test]
    fn test_format_error_with_recovery_conflict() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "draft_write".to_string(),
            args: json!({}),
        };
        let result = ToolResult {
            ok: false,
            data: None,
            error: Some(crate::agent_tools::contracts::ToolError {
                code: "E_CONFLICT".to_string(),
                message: "revision mismatch".to_string(),
                retryable: true,
                fault_domain: crate::agent_tools::contracts::FaultDomain::Vc,
                details: None,
            }),
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "draft_write".to_string(),
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
        assert!(content.contains("[error code=E_CONFLICT]"));
        assert!(content.contains("Recovery:"));
        assert!(content.contains("context_read"));
    }

    #[test]
    fn test_format_error_timeout_has_recovery_hint() {
        let tc = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "context_search".to_string(),
            args: json!({}),
        };
        let timeout = std::time::Duration::from_millis(60_000);
        let result =
            super::super::tool_errors::tool_timeout_error("context_search", "call_1", timeout);
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
            tool_name: "context_read".to_string(),
            args: json!({"target_ref": "chapter:manuscripts/vol_1/ch_1.json"}),
        };
        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "ref": "chapter:manuscripts/vol_1/ch_1.json",
                "kind": "chapter",
                "content": big_text
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "context_read".to_string(),
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
            tool_name: "context_read".to_string(),
            args: json!({"target_ref": "chapter:manuscripts/vol_1/ch_1.json"}),
        };
        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "ref": "chapter:manuscripts/vol_1/ch_1.json",
                "kind": "chapter",
                "content": "short content"
            })),
            error: None,
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "context_read".to_string(),
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
        assert!(content.contains("short content"));
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
            tool_name: "context_read".to_string(),
            args: json!({}),
        };
        let result = ToolResult {
            ok: false,
            data: None,
            error: Some(crate::agent_tools::contracts::ToolError {
                code: "E_REF_NOT_FOUND".to_string(),
                message: "file not found at D:\\Users\\admin\\project\\manuscripts\\ch1.json"
                    .to_string(),
                retryable: false,
                fault_domain: crate::agent_tools::contracts::FaultDomain::Io,
                details: None,
            }),
            meta: crate::agent_tools::contracts::ToolMeta {
                tool: "context_read".to_string(),
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
