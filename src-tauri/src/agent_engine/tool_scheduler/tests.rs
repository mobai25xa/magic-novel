use super::*;
use crate::agent_tools::contracts::ConfirmationPolicy;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::suspend::requires_confirmation;

#[derive(Clone)]
struct TestSink;

impl crate::agent_engine::emitter::EventSink for TestSink {
    fn emit_raw(
        &self,
        _event_type: &str,
        _payload: serde_json::Value,
    ) -> Result<(), crate::models::AppError> {
        Ok(())
    }
}

#[test]
fn test_group_calls_all_parallel() {
    let calls = vec![
        ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "workspace_map".to_string(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c2".to_string(),
            tool_name: "context_read".to_string(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c3".to_string(),
            tool_name: "context_search".to_string(),
            args: json!({}),
        },
    ];

    let groups = group_calls(&calls);
    assert_eq!(groups.len(), 1);
    match &groups[0] {
        ExecGroup::Parallel(batch) => assert_eq!(batch.len(), 3),
        _ => panic!("expected parallel group"),
    }
}

#[test]
fn test_group_calls_context_tools_parallel_batch() {
    let calls = vec![
        ToolCallInfo {
            llm_call_id: "c1".into(),
            tool_name: "workspace_map".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c2".into(),
            tool_name: "context_read".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c3".into(),
            tool_name: "context_search".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c4".into(),
            tool_name: "knowledge_read".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c5".into(),
            tool_name: "review_check".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c6".into(),
            tool_name: "context_read".into(),
            args: json!({}),
        },
    ];

    let groups = group_calls(&calls);
    assert_eq!(groups.len(), 1);
    match &groups[0] {
        ExecGroup::Parallel(batch) => assert_eq!(batch.len(), 6),
        _ => panic!("expected parallel group"),
    }
}

#[test]
fn test_group_calls_stateful_tools_sequential() {
    let calls = vec![
        ToolCallInfo {
            llm_call_id: "c1".into(),
            tool_name: "structure_edit".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c2".into(),
            tool_name: "draft_write".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c3".into(),
            tool_name: "knowledge_write".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c4".into(),
            tool_name: "todowrite".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c5".into(),
            tool_name: "askuser".into(),
            args: json!({}),
        },
    ];

    let groups = group_calls(&calls);
    assert_eq!(groups.len(), 5);
    for group in groups {
        match group {
            ExecGroup::Sequential(_) => {}
            _ => panic!("all stateful tools should be sequential"),
        }
    }
}

#[test]
fn test_group_calls_mixed() {
    let calls = vec![
        ToolCallInfo {
            llm_call_id: "c1".into(),
            tool_name: "context_read".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c2".into(),
            tool_name: "draft_write".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c3".into(),
            tool_name: "context_search".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c4".into(),
            tool_name: "structure_edit".into(),
            args: json!({}),
        },
    ];

    let groups = group_calls(&calls);
    assert_eq!(groups.len(), 4);
    match &groups[0] {
        ExecGroup::Parallel(batch) => assert_eq!(batch.len(), 1),
        _ => panic!("expected parallel group"),
    }
    match &groups[1] {
        ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "draft_write"),
        _ => panic!("expected sequential"),
    }
    match &groups[2] {
        ExecGroup::Parallel(batch) => assert_eq!(batch.len(), 1),
        _ => panic!("expected parallel group"),
    }
    match &groups[3] {
        ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "structure_edit"),
        _ => panic!("expected sequential"),
    }
}

#[test]
fn test_group_calls_all_sequential() {
    let calls = vec![
        ToolCallInfo {
            llm_call_id: "c1".into(),
            tool_name: "draft_write".into(),
            args: json!({}),
        },
        ToolCallInfo {
            llm_call_id: "c2".into(),
            tool_name: "structure_edit".into(),
            args: json!({}),
        },
    ];

    let groups = group_calls(&calls);
    assert_eq!(groups.len(), 2);
    match &groups[0] {
        ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "draft_write"),
        _ => panic!("expected sequential"),
    }
    match &groups[1] {
        ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "structure_edit"),
        _ => panic!("expected sequential"),
    }
}

#[test]
fn test_confirmation_matrix_matches_contract() {
    let confirm_scheduler = ToolScheduler::new(
        TestSink,
        "D:/p".to_string(),
        ApprovalMode::ConfirmWrites,
        ClarificationMode::Interactive,
        CancellationToken::new(),
    );
    let auto_scheduler = ToolScheduler::new(
        TestSink,
        "D:/p".to_string(),
        ApprovalMode::Auto,
        ClarificationMode::Interactive,
        CancellationToken::new(),
    );

    let read_call = ToolCallInfo {
        llm_call_id: "c0".to_string(),
        tool_name: "context_read".to_string(),
        args: json!({}),
    };
    let write_call = ToolCallInfo {
        llm_call_id: "c1".to_string(),
        tool_name: "draft_write".to_string(),
        args: json!({}),
    };

    assert!(!confirm_scheduler.needs_confirmation(&read_call));
    assert!(confirm_scheduler.needs_confirmation(&write_call));
    assert!(!auto_scheduler.needs_confirmation(&write_call));
}

#[test]
fn test_requires_confirmation_for_always_policy_in_auto_mode() {
    assert!(requires_confirmation(ConfirmationPolicy::Always, ApprovalMode::Auto,));
}

#[test]
fn test_headless_defer_blocks_askuser_suspend() {
    let scheduler = ToolScheduler::new(
        TestSink,
        "D:/p".to_string(),
        ApprovalMode::Auto,
        ClarificationMode::HeadlessDefer,
        CancellationToken::new(),
    );
    let tc = ToolCallInfo {
        llm_call_id: "c1".to_string(),
        tool_name: "askuser".to_string(),
        args: json!({
            "questions": [{
                "question": "Pick one",
                "topic": "style",
                "options": ["A", "B"]
            }]
        }),
    };

    let suspend = scheduler
        .build_tool_suspend(&tc, &[tc.clone()], 0)
        .expect("headless path should not error");
    assert!(suspend.is_none());
}

#[test]
fn test_build_tool_suspend_fills_path_from_active_chapter() {
    let scheduler = ToolScheduler::new(
        TestSink,
        "D:/p".to_string(),
        ApprovalMode::ConfirmWrites,
        ClarificationMode::Interactive,
        CancellationToken::new(),
    )
    .with_active_chapter_path(Some("vol_1/ch_1.json".to_string()));

    let tc = ToolCallInfo {
        llm_call_id: "c1".to_string(),
        tool_name: "askuser".to_string(),
        args: json!({
            "questions": [
                {
                    "question": "Pick one",
                    "topic": "style",
                    "options": ["A", "B"]
                }
            ]
        }),
    };

    let suspend = scheduler
        .build_tool_suspend(&tc, &[tc.clone()], 0)
        .expect("should build suspend")
        .expect("suspend should exist");

    assert_eq!(
        suspend
            .pending_tool_call
            .args
            .get("path")
            .and_then(|v| v.as_str()),
        Some("vol_1/ch_1.json")
    );
    assert!(suspend.pending_tool_call.args.get("chapter_path").is_none());
}
