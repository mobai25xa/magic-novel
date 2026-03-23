use crate::models::AppError;

use super::errors::{tool_cancelled_result, tool_is_allowed, tool_not_allowed_result};
use super::review_hook::maybe_emit_post_write_review;
use super::super::emitter::EventSink;
use super::super::messages::AgentMessage;
use super::super::tool_dispatch::execute_tool_call;
use super::super::tool_errors::{
    get_tool_timeout, tool_join_error, tool_lock_error, tool_timeout_error, write_resource_key,
};
use super::super::tool_formatters::{build_tool_message, build_tool_trace};
use super::super::types::ToolCallInfo;
use super::{errors::cancelled_error, ToolScheduler};

impl<S: EventSink> ToolScheduler<S> {
    /// Execute a group of parallel-safe tools concurrently.
    pub(super) async fn execute_parallel(
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
                let schema_context = self.schema_context.clone();
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

                    let timeout_dur = get_tool_timeout(&tc.tool_name, &tc.args);
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
                                let schema_context = schema_context.clone();
                                move || {
                                    execute_tool_call(
                                        &tc,
                                        &project_path,
                                        &call_id,
                                        active_chapter_path.as_deref(),
                                        active_skill.as_deref(),
                                        schema_context,
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
    pub(super) async fn execute_single(&self, tc: &ToolCallInfo) -> Result<AgentMessage, AppError> {
        if self.cancel_token.is_cancelled() {
            return Err(cancelled_error(None));
        }

        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
        self.emitter.tool_call_started(tc, &call_id)?;

        if !self.is_tool_allowed(&tc.tool_name) {
            let result =
                tool_not_allowed_result(&tc.tool_name, &call_id, self.allowed_tools.as_deref());
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
        let schema_context = self.schema_context.clone();
        let tool_name_for_err = tc.tool_name.clone();
        let call_id_for_err = call_id.clone();

        let resource_key = write_resource_key(tc, &self.project_path);
        let timeout_dur = get_tool_timeout(&tc.tool_name, &tc.args);

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
                        schema_context,
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

    pub(super) async fn execute_disallowed(
        &self,
        tc: &ToolCallInfo,
    ) -> Result<AgentMessage, AppError> {
        let call_id = format!("tool_{}", uuid::Uuid::new_v4());
        self.emitter.tool_call_started(tc, &call_id)?;
        let result =
            tool_not_allowed_result(&tc.tool_name, &call_id, self.allowed_tools.as_deref());
        self.emitter.tool_call_progress(tc, &call_id, "error")?;
        let trace = Some(build_tool_trace(&tc.tool_name, &result));
        self.emitter
            .tool_call_finished(tc, &call_id, "error", trace)?;
        Ok(build_tool_message(tc, &result))
    }

    pub(super) fn is_tool_allowed(&self, tool_name: &str) -> bool {
        tool_is_allowed(self.allowed_tools.as_deref(), tool_name)
    }
}
