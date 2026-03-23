use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent_engine::emitter::EventSink;
use crate::agent_engine::loop_engine::AgentLoop;
use crate::agent_engine::messages::{AgentMessage, ConversationState, Role};
use crate::agent_engine::types::{LoopConfig, StopReason};
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::streaming_turn::StreamingTurnEngine;
use crate::mission::delegate_types::{
    DelegateRequest, DelegateResult, OpenIssue, TaskResultStatus, TaskStopReason,
};
use crate::mission::result_types::{AgentTaskResult, EvidenceItem, TaskUsage};
use crate::models::AppError;

use super::runner::{DelegateRunContext, DelegateRunner};

#[derive(Clone, Default)]
struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn emit_raw(&self, _event_type: &str, _payload: serde_json::Value) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InProcessDelegateRunner {
    cancel_token: Option<CancellationToken>,
}

impl InProcessDelegateRunner {
    pub fn new() -> Self {
        Self { cancel_token: None }
    }

    pub fn with_cancel_token(mut self, cancel_token: CancellationToken) -> Self {
        self.cancel_token = Some(cancel_token);
        self
    }
}

#[async_trait]
impl DelegateRunner for InProcessDelegateRunner {
    fn runner_kind(&self) -> &'static str {
        "in_process"
    }

    async fn run_delegate(&self, context: DelegateRunContext) -> Result<DelegateResult, AppError> {
        let context = context.normalized();
        let request = context.request.clone();

        let result = run_delegate_in_process(context, self.cancel_token.clone()).await?;
        Ok(DelegateResult::from_agent_task_result(
            request.delegate_id,
            request.job_id,
            request.parent_task_id,
            result,
        ))
    }
}

async fn run_delegate_in_process(
    context: DelegateRunContext,
    external_cancel_token: Option<CancellationToken>,
) -> Result<AgentTaskResult, AppError> {
    let request = context.request.clone();
    validate_credentials(&context)?;

    let role_profile = context.role_profile.to_agent_profile();
    let effective_model = role_profile
        .model
        .as_deref()
        .unwrap_or(context.model.as_str())
        .trim()
        .to_string();

    let system_prompt = build_system_prompt(&context);
    let user_prompt = build_user_prompt(&request);
    let session_id = if request.delegate_id.is_empty() {
        format!("delegate_{}", uuid::Uuid::new_v4())
    } else {
        request.delegate_id.clone()
    };

    let mut conv = ConversationState::new(session_id);
    conv.messages.push(AgentMessage::system(system_prompt));
    conv.messages.push(AgentMessage::user(user_prompt));

    let loop_config = LoopConfig {
        max_rounds: role_profile.max_rounds,
        max_tool_calls: role_profile.max_tool_calls,
        autonomy_level: role_profile.approval_mode.to_autonomy_level(),
        capability_mode: role_profile.mode,
        approval_mode: role_profile.approval_mode,
        clarification_mode: role_profile.clarification_mode,
    };

    let sink = NoopEventSink;
    let cancel_token = external_cancel_token.unwrap_or_else(CancellationToken::new);
    let delegate_depth = if matches!(
        request.session_source,
        crate::mission::role_profile::SessionSource::Delegate
    ) {
        1
    } else {
        0
    };

    let agent_loop = AgentLoop::new(
        sink.clone(),
        loop_config,
        context.project_path.clone(),
        cancel_token.clone(),
    )
    .with_mission_id(Some(context.mission_id.clone()))
    .with_provider_info(
        context.provider.clone(),
        effective_model.clone(),
        context.base_url.clone(),
        context.api_key.clone(),
    )
    .with_capability_policy(role_profile.capability_policy())
    .with_session_source(request.session_source)
    .with_delegate_depth(delegate_depth);

    let router = build_router(
        &context.provider,
        context.base_url.clone(),
        context.api_key.clone(),
        RetryConfig::worker(),
    );
    let streaming_engine =
        StreamingTurnEngine::new(router, sink, context.provider.clone(), effective_model)
            .with_cancel_token(cancel_token);

    let loop_result = agent_loop.run(&mut conv, &streaming_engine).await?;
    Ok(build_task_result(
        &request,
        &context,
        &conv,
        &loop_result.stop_reason,
        loop_result.rounds_executed,
        loop_result.total_tool_calls,
        loop_result.latency_ms,
    ))
}

fn validate_credentials(context: &DelegateRunContext) -> Result<(), AppError> {
    if context.base_url.trim().is_empty() || context.api_key.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "in-process delegate run requires base_url and api_key",
        ));
    }
    Ok(())
}

fn build_system_prompt(context: &DelegateRunContext) -> String {
    format!(
        "{}\n\n[Delegate Runtime]\nMission ID: {}\nActor ID: {}\nDelegate ID: {}\nMission Dir: {}\nSession Source: {}\n\nPolicy: for multi-step changes, call todowrite first with milestone-level, user-verifiable tasks and keep one in_progress item.",
        context.role_profile.system_prompt,
        context.mission_id,
        context.actor_id,
        context.request.delegate_id,
        context.mission_dir,
        context.request.session_source.as_str(),
    )
}

fn build_user_prompt(request: &DelegateRequest) -> String {
    let refs = if request.input_refs.is_empty() {
        "- (none)".to_string()
    } else {
        request
            .input_refs
            .iter()
            .map(|r| format!("- {}: {}", r.kind, r.value))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let expected = if request.expected_outputs.is_empty() {
        "- (none)".to_string()
    } else {
        request
            .expected_outputs
            .iter()
            .map(|r| format!("- {}: {}", r.kind, r.value))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Delegate goal:\n{}\n\nInput refs:\n{}\n\nExpected outputs:\n{}",
        request.goal, refs, expected
    )
}

fn assistant_summary(conv: &ConversationState) -> Option<String> {
    conv.messages
        .iter()
        .rev()
        .find(|m| matches!(m.role, Role::Assistant))
        .map(|m| m.text_content())
        .filter(|text| !text.is_empty())
}

fn map_stop_reason(stop_reason: &StopReason) -> (TaskResultStatus, TaskStopReason) {
    match stop_reason {
        StopReason::Success => (TaskResultStatus::Completed, TaskStopReason::Success),
        StopReason::Cancel => (TaskResultStatus::Cancelled, TaskStopReason::Cancelled),
        StopReason::Error => (TaskResultStatus::Failed, TaskStopReason::Error),
        StopReason::Limit => (TaskResultStatus::Blocked, TaskStopReason::Limit),
        StopReason::WaitingConfirmation => (
            TaskResultStatus::Blocked,
            TaskStopReason::WaitingConfirmation,
        ),
        StopReason::WaitingAskuser => (TaskResultStatus::Blocked, TaskStopReason::WaitingAskuser),
    }
}

fn next_actions_for_status(status: TaskResultStatus, stop_reason: TaskStopReason) -> Vec<String> {
    match (status, stop_reason) {
        (TaskResultStatus::Completed, _) => Vec::new(),
        (TaskResultStatus::Cancelled, _) => vec!["resume or rerun the delegate".to_string()],
        (TaskResultStatus::Failed, _) => vec!["inspect delegate summary and retry".to_string()],
        (TaskResultStatus::Blocked, TaskStopReason::Limit) => {
            vec!["adjust limits or split the delegate goal".to_string()]
        }
        (TaskResultStatus::Blocked, TaskStopReason::WaitingConfirmation) => {
            vec!["provide approval from parent runtime".to_string()]
        }
        (TaskResultStatus::Blocked, TaskStopReason::WaitingAskuser) => {
            vec!["convert the task to interactive handling".to_string()]
        }
        (TaskResultStatus::Blocked, _) => vec!["inspect open issues before resuming".to_string()],
    }
}

fn build_task_result(
    request: &DelegateRequest,
    context: &DelegateRunContext,
    conv: &ConversationState,
    stop_reason: &StopReason,
    rounds_executed: u32,
    total_tool_calls: u32,
    latency_ms: u64,
) -> AgentTaskResult {
    let (status, mapped_stop_reason) = map_stop_reason(stop_reason);
    let summary = assistant_summary(conv).unwrap_or_else(|| {
        format!(
            "Delegate '{}' stopped after {} rounds ({} tool calls)",
            request.delegate_id, rounds_executed, total_tool_calls
        )
    });

    let mut open_issues = Vec::new();
    if !matches!(status, TaskResultStatus::Completed) {
        open_issues.push(OpenIssue {
            code: Some(format!("delegate::{mapped_stop_reason:?}").to_ascii_lowercase()),
            summary: summary.clone(),
            blocking: true,
        });
    }

    AgentTaskResult {
        task_id: if request.parent_task_id.is_empty() {
            request.delegate_id.clone()
        } else {
            request.parent_task_id.clone()
        },
        actor_id: context.actor_id.clone(),
        goal: request.goal.clone(),
        status,
        stop_reason: mapped_stop_reason,
        result_summary: summary.clone(),
        evidence: vec![EvidenceItem {
            kind: "assistant_summary".to_string(),
            summary,
            value: None,
        }],
        open_issues,
        next_actions: next_actions_for_status(status, mapped_stop_reason),
        usage: Some(TaskUsage {
            rounds_executed,
            total_tool_calls,
            latency_ms,
            llm_usage: conv.last_usage.clone(),
        }),
        ..AgentTaskResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn map_stop_reason_marks_waiting_confirmation_as_blocked() {
        let (status, stop_reason) = map_stop_reason(&StopReason::WaitingConfirmation);
        assert_eq!(status, TaskResultStatus::Blocked);
        assert_eq!(stop_reason, TaskStopReason::WaitingConfirmation);
        assert_eq!(
            next_actions_for_status(status, stop_reason),
            vec!["provide approval from parent runtime".to_string()]
        );
    }

    #[test]
    fn map_stop_reason_marks_cancel_as_cancelled() {
        let (status, stop_reason) = map_stop_reason(&StopReason::Cancel);
        assert_eq!(status, TaskResultStatus::Cancelled);
        assert_eq!(stop_reason, TaskStopReason::Cancelled);
        assert_eq!(
            next_actions_for_status(status, stop_reason),
            vec!["resume or rerun the delegate".to_string()]
        );
    }

    #[test]
    fn runner_with_cancel_token_preserves_external_signal() {
        let token = CancellationToken::new();
        let child = token.child_token();

        let runner = InProcessDelegateRunner::new().with_cancel_token(token);

        runner
            .cancel_token
            .as_ref()
            .expect("cancel token should be set")
            .cancel();

        assert!(child.is_cancelled());
    }
}
