//! Agent Engine - Tool scheduler with parallel grouping and confirmation pause
//!
//! Aligned with docs/magic_plan/plan_agent/03-tool-scheduler-and-confirmation.md
//!
//! Uses the "flush-on-sequential" algorithm:
//! - Accumulate parallel_safe tools into a batch
//! - When a non-parallel_safe tool is encountered, flush the batch (join_all), then execute sequentially
//! - At the end, flush any remaining batch

use serde::{Deserialize, Serialize};
use serde_json::json;

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent_tools::contracts::ConfirmationPolicy;
use crate::agent_tools::registry::get_manifest;
use crate::models::{AppError, ErrorCode};
use crate::review::{engine as review_engine, types as review_types};

use super::emitter::EventSink;
use super::events::event_types;
use super::messages::AgentMessage;
use super::tool_dispatch::execute_tool_call;
use super::tool_errors::{
    get_tool_timeout, tool_join_error, tool_lock_error, tool_timeout_error, write_resource_key,
};
use super::tool_formatters::{
    build_tool_message, build_tool_trace, extract_askuser_questions, extract_questionnaire,
    is_askuser_call, validate_askuser_args,
};
use super::types::{ApprovalMode, ClarificationMode, StopReason, ToolCallInfo};

/// Result of executing a batch of tool calls
#[derive(Debug)]
pub struct BatchResult {
    /// Tool result messages to append to conversation
    pub tool_messages: Vec<AgentMessage>,
    /// If set, the loop should suspend
    pub suspend_reason: Option<SuspendInfo>,
    /// Number of tool calls executed
    pub executed_count: u32,
}

/// Info about why execution was suspended
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendInfo {
    pub reason: StopReason,
    /// The tool call that caused suspension
    pub pending_tool_call: ToolCallInfo,
    /// Call ID for the pending tool
    pub pending_call_id: String,
    /// Tool calls that still need execution after the pending one
    pub remaining_tool_calls: Vec<ToolCallInfo>,
    /// Tool results already collected before suspension
    pub completed_messages: Vec<AgentMessage>,
}

/// Groups of tool calls for execution ordering
enum ExecGroup {
    /// Tools that can run in parallel (all parallel_safe)
    Parallel(Vec<ToolCallInfo>),
    /// A single tool that must run sequentially (not parallel_safe)
    Sequential(ToolCallInfo),
}

#[derive(Clone)]
struct ResourceLockManager {
    locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Semaphore>>>,
}

impl ResourceLockManager {
    fn new() -> Self {
        Self {
            locks: Arc::new(dashmap::DashMap::new()),
        }
    }

    async fn with_write_lock<F, Fut, T>(
        &self,
        resource_key: Option<String>,
        call_id: Option<String>,
        run: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        let Some(key) = resource_key else {
            return run().await;
        };

        let semaphore = self
            .locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1)))
            .clone();

        let _permit = semaphore.acquire_owned().await.map_err(|_| AppError {
            code: ErrorCode::Internal,
            message: "resource lock closed".to_string(),
            details: Some(json!({
                "code": "E_TOOL_RESOURCE_LOCK_CLOSED",
                "resource_key": key,
                "call_id": call_id,
            })),
            recoverable: Some(true),
        })?;

        run().await
    }
}

pub struct ToolScheduler<S: EventSink> {
    emitter: S,
    project_path: String,
    approval_mode: ApprovalMode,
    clarification_mode: ClarificationMode,
    active_chapter_path: Option<String>,
    active_skill: Option<String>,
    allowed_tools: Option<Arc<HashSet<String>>>,
    cancel_token: CancellationToken,
    lock_manager: ResourceLockManager,
}

impl<S: EventSink> ToolScheduler<S> {
    pub fn new(
        emitter: S,
        project_path: String,
        approval_mode: ApprovalMode,
        clarification_mode: ClarificationMode,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            emitter,
            project_path,
            approval_mode,
            clarification_mode,
            active_chapter_path: None,
            active_skill: None,
            allowed_tools: None,
            cancel_token,
            lock_manager: ResourceLockManager::new(),
        }
    }

    pub fn with_active_chapter_path(mut self, active_chapter_path: Option<String>) -> Self {
        self.active_chapter_path = active_chapter_path
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        self
    }

    pub fn with_active_skill(mut self, active_skill: Option<String>) -> Self {
        self.active_skill = active_skill
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        self
    }

    pub fn with_allowed_tools(mut self, allowed_tools: Option<Vec<String>>) -> Self {
        self.allowed_tools = allowed_tools
            .map(|tools| {
                tools
                    .into_iter()
                    .map(|t| t.trim().to_ascii_lowercase())
                    .filter(|t| !t.is_empty())
                    .collect::<HashSet<_>>()
            })
            .filter(|set| !set.is_empty())
            .map(Arc::new);
        self
    }

    /// Execute a batch of tool calls with parallel grouping and confirmation checks.
    pub async fn execute_batch(
        &self,
        tool_calls: Vec<ToolCallInfo>,
    ) -> Result<BatchResult, AppError> {
        if self.cancel_token.is_cancelled() {
            return Err(cancelled_error(None));
        }

        let groups = group_calls(&tool_calls);
        let mut tool_messages = Vec::new();
        let mut executed_count = 0_u32;
        let mut consumed_calls = 0_usize;

        for group in groups {
            if self.cancel_token.is_cancelled() {
                return Err(cancelled_error(None));
            }
            match group {
                ExecGroup::Parallel(calls) => {
                    let call_count = calls.len();
                    let results = self.execute_parallel(calls).await?;
                    consumed_calls += call_count;
                    executed_count += results.len() as u32;
                    tool_messages.extend(results);
                }
                ExecGroup::Sequential(tc) => {
                    if !self.is_tool_allowed(&tc.tool_name) {
                        let msg = self.execute_disallowed(&tc).await?;
                        consumed_calls += 1;
                        executed_count += 1;
                        tool_messages.push(msg);
                        continue;
                    }

                    if self.cancel_token.is_cancelled() {
                        return Err(cancelled_error(None));
                    }

                    if let Some(mut suspend) =
                        self.build_askuser_suspend(&tc, &tool_calls, consumed_calls)?
                    {
                        suspend.completed_messages = tool_messages.clone();
                        return Ok(BatchResult {
                            tool_messages: tool_messages.clone(),
                            suspend_reason: Some(suspend),
                            executed_count,
                        });
                    }

                    // Check if confirmation is needed
                    if self.needs_confirmation(&tc) {
                        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
                        self.emitter
                            .waiting_for_confirmation(&tc, &call_id, "sensitive_write")?;

                        let remaining_tool_calls = tool_calls
                            .iter()
                            .skip(consumed_calls + 1)
                            .cloned()
                            .collect::<Vec<_>>();

                        return Ok(BatchResult {
                            tool_messages: tool_messages.clone(),
                            suspend_reason: Some(SuspendInfo {
                                reason: StopReason::WaitingConfirmation,
                                pending_tool_call: tc,
                                pending_call_id: call_id,
                                remaining_tool_calls,
                                completed_messages: tool_messages,
                            }),
                            executed_count,
                        });
                    }

                    let result = self.execute_single(&tc).await?;
                    consumed_calls += 1;
                    executed_count += 1;
                    tool_messages.push(result);
                }
            }
        }

        Ok(BatchResult {
            tool_messages,
            suspend_reason: None,
            executed_count,
        })
    }

    /// Execute a group of parallel-safe tools concurrently.
    async fn execute_parallel(
        &self,
        calls: Vec<ToolCallInfo>,
    ) -> Result<Vec<AgentMessage>, AppError> {
        if self.cancel_token.is_cancelled() {
            return Err(cancelled_error(None));
        }

        let allowed_tools = self.allowed_tools.clone();
        let cancel_token = self.cancel_token.clone();
        let futs: Vec<_> = calls
            .iter()
            .map(|tc| {
                let tc = tc.clone();
                let project_path = self.project_path.clone();
                let emitter = self.emitter.clone();
                let lock_manager = self.lock_manager.clone();
                let active_chapter_path = self.active_chapter_path.clone();
                let active_skill = self.active_skill.clone();
                let allowed_tools = allowed_tools.clone();
                let cancel_token = cancel_token.clone();
                async move {
                    let call_id = format!("tool_{}", uuid::Uuid::new_v4());
                    emitter.tool_call_started(&tc, &call_id).ok();

                    if cancel_token.is_cancelled() {
                        let result = tool_cancelled_result(&tc.tool_name, &call_id);
                        emitter.tool_call_progress(&tc, &call_id, "error").ok();
                        let trace = Some(build_tool_trace(&tc.tool_name, &result));
                        emitter
                            .tool_call_finished(&tc, &call_id, "error", trace)
                            .ok();
                        return build_tool_message(&tc, &result);
                    }

                    if !tool_is_allowed(allowed_tools.as_deref(), &tc.tool_name) {
                        let result = tool_not_allowed_result(
                            &tc.tool_name,
                            &call_id,
                            allowed_tools.as_deref(),
                        );
                        emitter.tool_call_progress(&tc, &call_id, "error").ok();
                        let trace = Some(build_tool_trace(&tc.tool_name, &result));
                        emitter
                            .tool_call_finished(&tc, &call_id, "error", trace)
                            .ok();
                        return build_tool_message(&tc, &result);
                    }

                    let timeout_dur = get_tool_timeout(&tc.tool_name);
                    let resource_key = write_resource_key(&tc, &project_path);

                    let result = tokio::select! {
                        _ = cancel_token.cancelled() => {
                            tool_cancelled_result(&tc.tool_name, &call_id)
                        }
                        guarded = lock_manager.with_write_lock(resource_key, Some(call_id.clone()), || async {
                            if cancel_token.is_cancelled() {
                                return Ok(tool_cancelled_result(&tc.tool_name, &call_id));
                            }

                            let blocking_fut = tokio::task::spawn_blocking({
                                let tc = tc.clone();
                                let call_id = call_id.clone();
                                let project_path = project_path.clone();
                                let active_chapter_path = active_chapter_path.clone();
                                let active_skill = active_skill.clone();
                                move || {
                                    execute_tool_call(
                                        &tc,
                                        &project_path,
                                        &call_id,
                                        active_chapter_path.as_deref(),
                                        active_skill.as_deref(),
                                    )
                                }
                            });
                            let outcome = tokio::select! {
                                _ = cancel_token.cancelled() => {
                                    tool_cancelled_result(&tc.tool_name, &call_id)
                                }
                                timed = tokio::time::timeout(timeout_dur, blocking_fut) => {
                                    match timed {
                                        Ok(join_result) => join_result.unwrap_or_else(|e| {
                                            tool_join_error(&tc.tool_name, &call_id, &e.to_string())
                                        }),
                                        Err(_elapsed) => tool_timeout_error(&tc.tool_name, &call_id, timeout_dur),
                                    }
                                }
                            };
                            Ok(outcome)
                        }) => {
                            guarded.unwrap_or_else(|e| tool_lock_error(&tc.tool_name, &call_id, &e))
                        }
                    };

                    let status = if result.ok { "ok" } else { "error" };
                    let progress = if result.ok { "done" } else { "error" };
                    emitter.tool_call_progress(&tc, &call_id, progress).ok();
                    let trace = Some(build_tool_trace(&tc.tool_name, &result));
                    emitter
                        .tool_call_finished(&tc, &call_id, status, trace)
                        .ok();

                    maybe_emit_post_write_review(
                        &emitter,
                        &cancel_token,
                        &project_path,
                        &tc,
                        &call_id,
                        &result,
                    )
                    .await;

                    build_tool_message(&tc, &result)
                }
            })
            .collect();

        let joined = futures::future::join_all(futs);
        let results = tokio::select! {
            _ = self.cancel_token.cancelled() => {
                return Err(cancelled_error(None));
            }
            results = joined => results,
        };

        Ok(results)
    }

    /// Execute a single tool call.
    async fn execute_single(&self, tc: &ToolCallInfo) -> Result<AgentMessage, AppError> {
        if self.cancel_token.is_cancelled() {
            return Err(cancelled_error(None));
        }

        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
        self.emitter.tool_call_started(tc, &call_id)?;

        if !self.is_tool_allowed(&tc.tool_name) {
            let result = tool_not_allowed_result(&tc.tool_name, &call_id, self.allowed_tools.as_deref());
            self.emitter.tool_call_progress(tc, &call_id, "error")?;
            let trace = Some(build_tool_trace(&tc.tool_name, &result));
            self.emitter
                .tool_call_finished(tc, &call_id, "error", trace)?;
            return Ok(build_tool_message(tc, &result));
        }

        let tc_clone = tc.clone();
        let call_id_clone = call_id.clone();
        let project_path = self.project_path.clone();
        let active_chapter_path = self.active_chapter_path.clone();
        let active_skill = self.active_skill.clone();
        let tool_name_for_err = tc.tool_name.clone();
        let call_id_for_err = call_id.clone();

        let timeout_dur = get_tool_timeout(&tc.tool_name);
        let resource_key = write_resource_key(tc, &self.project_path);

        let result = tokio::select! {
            _ = self.cancel_token.cancelled() => {
                tool_cancelled_result(&tc.tool_name, &call_id)
            }
            guarded = self.lock_manager.with_write_lock(resource_key, Some(call_id.clone()), || async {
                if self.cancel_token.is_cancelled() {
                    return Ok(tool_cancelled_result(&tc_clone.tool_name, &call_id_clone));
                }

                let blocking_fut = tokio::task::spawn_blocking(move || {
                    execute_tool_call(
                        &tc_clone,
                        &project_path,
                        &call_id_clone,
                        active_chapter_path.as_deref(),
                        active_skill.as_deref(),
                    )
                });

                let outcome = tokio::select! {
                    _ = self.cancel_token.cancelled() => tool_cancelled_result(&tool_name_for_err, &call_id_for_err),
                    timed = tokio::time::timeout(timeout_dur, blocking_fut) => {
                        match timed {
                            Ok(join_result) => join_result.unwrap_or_else(|e| {
                                tool_join_error(&tool_name_for_err, &call_id_for_err, &e.to_string())
                            }),
                            Err(_elapsed) => tool_timeout_error(&tool_name_for_err, &call_id_for_err, timeout_dur),
                        }
                    }
                };
                Ok(outcome)
            }) => guarded.unwrap_or_else(|e| tool_lock_error(&tc.tool_name, &call_id, &e)),
        };

        let status = if result.ok { "ok" } else { "error" };
        let progress = if result.ok { "done" } else { "error" };
        self.emitter.tool_call_progress(tc, &call_id, progress)?;
        let trace = Some(build_tool_trace(&tc.tool_name, &result));
        self.emitter
            .tool_call_finished(tc, &call_id, status, trace)?;

        maybe_emit_post_write_review(
            &self.emitter,
            &self.cancel_token,
            &self.project_path,
            tc,
            &call_id,
            &result,
        )
        .await;

        Ok(build_tool_message(tc, &result))
    }

    async fn execute_disallowed(&self, tc: &ToolCallInfo) -> Result<AgentMessage, AppError> {
        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
        self.emitter.tool_call_started(tc, &call_id)?;
        let result = tool_not_allowed_result(&tc.tool_name, &call_id, self.allowed_tools.as_deref());
        self.emitter.tool_call_progress(tc, &call_id, "error")?;
        let trace = Some(build_tool_trace(&tc.tool_name, &result));
        self.emitter
            .tool_call_finished(tc, &call_id, "error", trace)?;
        Ok(build_tool_message(tc, &result))
    }

    fn is_tool_allowed(&self, tool_name: &str) -> bool {
        tool_is_allowed(self.allowed_tools.as_deref(), tool_name)
    }

    fn build_askuser_suspend(
        &self,
        tc: &ToolCallInfo,
        tool_calls: &[ToolCallInfo],
        consumed_calls: usize,
    ) -> Result<Option<SuspendInfo>, AppError> {
        if !self.clarification_mode.exposes_askuser() || !is_askuser_call(tc) {
            return Ok(None);
        }

        if let Err(error) = validate_askuser_args(&tc.args) {
            tracing::warn!(
                target: "agent_engine",
                tool = "askuser",
                error = %error,
                "invalid askuser payload; falling back to runtime error"
            );
            return Ok(None);
        }

        // Try structured JSON first, then questionnaire DSL
        let structured_questions = extract_askuser_questions(tc);
        let questionnaire = extract_questionnaire(tc);

        // Must have at least one valid format
        if structured_questions.is_none() && questionnaire.is_none() {
            return Ok(None);
        }

        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
        let mut pending_tool_call = tc.clone();
        if pending_tool_call.args.as_object().is_none() {
            pending_tool_call.args = json!({});
        }
        if pending_tool_call
            .args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            if let Some(path) = self.active_chapter_path.as_deref() {
                pending_tool_call.args["path"] = json!(path);
            }
        }

        self.emitter.askuser_requested(
            &pending_tool_call,
            &call_id,
            structured_questions.as_ref(),
            questionnaire.as_deref(),
        )?;

        let remaining_tool_calls = tool_calls
            .iter()
            .skip(consumed_calls + 1)
            .cloned()
            .collect::<Vec<_>>();

        Ok(Some(SuspendInfo {
            reason: StopReason::WaitingAskuser,
            pending_tool_call,
            pending_call_id: call_id,
            remaining_tool_calls,
            completed_messages: Vec::new(),
        }))
    }

    /// Check if a tool call requires user confirmation.
    fn needs_confirmation(&self, tc: &ToolCallInfo) -> bool {
        let manifest = match get_manifest(&tc.tool_name) {
            Some(m) => m,
            None => return false,
        };

        requires_confirmation(manifest.confirmation, self.approval_mode)
    }
}

fn extract_post_write_review_target_ref(
    tc: &ToolCallInfo,
    result: &crate::agent_tools::contracts::ToolResult<serde_json::Value>,
) -> Option<String> {
    if tc.tool_name != "edit" {
        return None;
    }
    if !result.ok {
        return None;
    }
    let data = result.data.as_ref()?;

    let mode = data.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    if mode != "commit" {
        return None;
    }
    let target = data.get("target").and_then(|v| v.as_str()).unwrap_or("");
    if target != "chapter_content" {
        return None;
    }
    let accepted = data
        .get("accepted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !accepted {
        return None;
    }

    let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let path = path.trim();
    if path.is_empty() || !path.ends_with(".json") {
        return None;
    }

    Some(path.to_string())
}

async fn maybe_emit_post_write_review<S: EventSink>(
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

fn tool_is_allowed(allowed_tools: Option<&HashSet<String>>, tool_name: &str) -> bool {
    let Some(allowed) = allowed_tools else {
        return true;
    };
    allowed.contains(&tool_name.trim().to_ascii_lowercase())
}

fn tool_not_allowed_result(
    tool_name: &str,
    call_id: &str,
    allowed_tools: Option<&HashSet<String>>,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};

    let allowed = allowed_tools
        .map(|set| {
            let mut tools = set.iter().cloned().collect::<Vec<_>>();
            tools.sort();
            tools
        })
        .unwrap_or_default();

    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_TOOL_NOT_ALLOWED".to_string(),
            message: format!("tool '{}' is not allowed in this turn", tool_name),
            retryable: false,
            fault_domain: FaultDomain::Policy,
            details: Some(json!({
                "tool": tool_name,
                "allowed_tools": allowed,
            })),
        }),
        meta: ToolMeta {
            tool: tool_name.to_string(),
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

fn tool_cancelled_result(
    tool_name: &str,
    call_id: &str,
) -> crate::agent_tools::contracts::ToolResult<serde_json::Value> {
    use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};

    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: "E_CANCELLED".to_string(),
            message: "cancelled".to_string(),
            retryable: false,
            fault_domain: FaultDomain::Policy,
            details: Some(json!({ "tool": tool_name })),
        }),
        meta: ToolMeta {
            tool: tool_name.to_string(),
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

fn cancelled_error(call_id: Option<&str>) -> AppError {
    AppError {
        code: ErrorCode::Internal,
        message: "cancelled".to_string(),
        details: Some(json!({
            "code": "E_CANCELLED",
            "call_id": call_id,
        })),
        recoverable: Some(true),
    }
}

fn requires_confirmation(policy: ConfirmationPolicy, approval_mode: ApprovalMode) -> bool {
    match (policy, approval_mode) {
        (ConfirmationPolicy::Never, _) => false,
        (ConfirmationPolicy::SensitiveWrite, ApprovalMode::ConfirmWrites) => true,
        (ConfirmationPolicy::SensitiveWrite, ApprovalMode::Auto) => false,
        (ConfirmationPolicy::Always, _) => true,
    }
}

/// Flush-on-sequential grouping algorithm.
///
/// Groups tool calls into parallel batches (all parallel_safe) and sequential singles.
fn group_calls(calls: &[ToolCallInfo]) -> Vec<ExecGroup> {
    let mut groups = Vec::new();
    let mut parallel_batch: Vec<ToolCallInfo> = Vec::new();

    for tc in calls {
        let is_parallel = get_manifest(&tc.tool_name)
            .map(|m| m.parallel_safe)
            .unwrap_or(false);

        if is_parallel {
            parallel_batch.push(tc.clone());
        } else {
            // Flush accumulated parallel batch first
            if !parallel_batch.is_empty() {
                groups.push(ExecGroup::Parallel(std::mem::take(&mut parallel_batch)));
            }
            groups.push(ExecGroup::Sequential(tc.clone()));
        }
    }

    // Flush remaining parallel batch
    if !parallel_batch.is_empty() {
        groups.push(ExecGroup::Parallel(parallel_batch));
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

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
                tool_name: "read".to_string(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c2".to_string(),
                tool_name: "grep".to_string(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c3".to_string(),
                tool_name: "ls".to_string(),
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
                tool_name: "read".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c2".into(),
                tool_name: "outline".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c3".into(),
                tool_name: "character_sheet".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c4".into(),
                tool_name: "search_knowledge".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c5".into(),
                tool_name: "ls".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c6".into(),
                tool_name: "grep".into(),
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
                tool_name: "edit".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c2".into(),
                tool_name: "create".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c3".into(),
                tool_name: "todowrite".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c4".into(),
                tool_name: "askuser".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c5".into(),
                tool_name: "skill".into(),
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
                tool_name: "read".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c2".into(),
                tool_name: "edit".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c3".into(),
                tool_name: "grep".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c4".into(),
                tool_name: "edit".into(),
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
            ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "edit"),
            _ => panic!("expected sequential"),
        }
        match &groups[2] {
            ExecGroup::Parallel(batch) => assert_eq!(batch.len(), 1),
            _ => panic!("expected parallel group"),
        }
        match &groups[3] {
            ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "edit"),
            _ => panic!("expected sequential"),
        }
    }

    #[test]
    fn test_group_calls_all_sequential() {
        let calls = vec![
            ToolCallInfo {
                llm_call_id: "c1".into(),
                tool_name: "edit".into(),
                args: json!({}),
            },
            ToolCallInfo {
                llm_call_id: "c2".into(),
                tool_name: "create".into(),
                args: json!({}),
            },
        ];

        let groups = group_calls(&calls);
        assert_eq!(groups.len(), 2);
        match &groups[0] {
            ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "edit"),
            _ => panic!("expected sequential"),
        }
        match &groups[1] {
            ExecGroup::Sequential(tc) => assert_eq!(tc.tool_name, "create"),
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
            tool_name: "read".to_string(),
            args: json!({}),
        };
        let edit_call = ToolCallInfo {
            llm_call_id: "c1".to_string(),
            tool_name: "edit".to_string(),
            args: json!({}),
        };

        assert!(!confirm_scheduler.needs_confirmation(&read_call));
        assert!(confirm_scheduler.needs_confirmation(&edit_call));
        assert!(!auto_scheduler.needs_confirmation(&edit_call));
    }

    #[test]
    fn test_requires_confirmation_for_always_policy_in_auto_mode() {
        assert!(requires_confirmation(
            ConfirmationPolicy::Always,
            ApprovalMode::Auto,
        ));
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
            .build_askuser_suspend(&tc, &[tc.clone()], 0)
            .expect("headless path should not error");
        assert!(suspend.is_none());
    }

    #[test]
    fn test_build_askuser_suspend_fills_path_from_active_chapter() {
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
            .build_askuser_suspend(&tc, &[tc.clone()], 0)
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
}
