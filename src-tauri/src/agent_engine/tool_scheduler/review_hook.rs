use serde_json::json;

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::review::{engine as review_engine, types as review_types};

use super::super::emitter::EventSink;
use super::super::events::event_types;
use super::super::types::ToolCallInfo;

fn extract_post_write_review_target_ref(
    tc: &ToolCallInfo,
    result: &crate::agent_tools::contracts::ToolResult<serde_json::Value>,
) -> Option<String> {
    if tc.tool_name != "draft_write" {
        return None;
    }
    if !result.ok {
        return None;
    }
    let target_ref = tc.args.get("target_ref").and_then(|v| v.as_str()).unwrap_or("");
    let target_ref = target_ref.trim();
    if target_ref.is_empty() {
        return None;
    }

    let path = target_ref.strip_prefix("chapter:").unwrap_or(target_ref).trim();
    if path.is_empty() || !path.ends_with(".json") {
        return None;
    }

    Some(path.to_string())
}

pub(super) async fn maybe_emit_post_write_review<S: EventSink>(
    emitter: &S,
    cancel_token: &CancellationToken,
    project_path: &str,
    tc: &ToolCallInfo,
    call_id: &str,
    result: &crate::agent_tools::contracts::ToolResult<serde_json::Value>,
) {
    if cancel_token.is_cancelled() {
        return;
    }
    if emitter.source_kind() != "agent" {
        return;
    }

    let Some(target_ref) = extract_post_write_review_target_ref(tc, result) else {
        return;
    };

    let project_path = project_path.to_string();
    let target_ref_for_run = target_ref.clone();

    let report = match tokio::task::spawn_blocking(move || {
        let input = review_types::ReviewRunInput {
            scope_ref: format!("chapter:{target_ref_for_run}"),
            target_refs: vec![target_ref_for_run],
            branch_id: None,
            review_types: vec![review_types::ReviewType::WordCount],
            task_card_ref: None,
            context_pack_ref: None,
            effective_rules_fingerprint: None,
            severity_threshold: None,
        };
        review_engine::run_review(Path::new(&project_path), input)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::warn!(
                target: "review",
                error = %e,
                target_ref = %target_ref,
                "post-write review failed"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                target: "review",
                error = %e,
                target_ref = %target_ref,
                "post-write review join error"
            );
            return;
        }
    };

    let mut warn = 0_i32;
    let mut block = 0_i32;
    for i in &report.issues {
        match i.severity {
            review_types::ReviewSeverity::Warn => warn += 1,
            review_types::ReviewSeverity::Block => block += 1,
            _ => {}
        }
    }

    let payload = json!({
        "hook": "post_write",
        "call_id": call_id,
        "llm_call_id": tc.llm_call_id.as_str(),
        "tool_name": tc.tool_name.as_str(),
        "target_ref": target_ref,
        "revision_after": result.meta.revision_after,
        "issue_counts": {
            "total": report.issues.len() as i32,
            "warn": warn,
            "block": block,
        },
        "overall_status": report.overall_status,
        "recommended_action": report.recommended_action,
        "generated_at": report.generated_at,
        "report": report,
    });

    let _ = emitter.emit_raw(event_types::REVIEW_RECORDED, payload);
}
