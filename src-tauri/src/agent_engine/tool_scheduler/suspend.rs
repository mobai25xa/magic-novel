use crate::agent_tools::contracts::ConfirmationPolicy;
use crate::agent_tools::definition::{ToolSuspendContext, ToolSuspendEmitter};
use crate::agent_tools::registry::{get_manifest, get_tool};
use crate::models::AppError;

use super::super::emitter::EventSink;
use super::super::types::{ApprovalMode, ToolCallInfo};
use super::{SuspendInfo, ToolScheduler};

impl<S: EventSink> ToolScheduler<S> {
    pub(super) fn build_tool_suspend(
        &self,
        tc: &ToolCallInfo,
        tool_calls: &[ToolCallInfo],
        consumed_calls: usize,
    ) -> Result<Option<SuspendInfo>, AppError> {
        let Some(tool) = get_tool(&tc.tool_name) else {
            return Ok(None);
        };

        let emitter: &dyn ToolSuspendEmitter = &self.emitter;
        let ctx = ToolSuspendContext {
            clarification_mode: self.clarification_mode,
            active_chapter_path: self.active_chapter_path.as_deref(),
            emitter,
            tool_calls,
            consumed_calls,
        };

        let Some(plan) = tool.try_build_suspend(tc, &ctx)? else {
            return Ok(None);
        };

        Ok(Some(SuspendInfo {
            reason: plan.reason,
            pending_tool_call: plan.pending_tool_call,
            pending_call_id: plan.pending_call_id,
            remaining_tool_calls: plan.remaining_tool_calls,
            completed_messages: Vec::new(),
        }))
    }

    /// Check if a tool call requires user confirmation.
    pub(super) fn needs_confirmation(&self, tc: &ToolCallInfo) -> bool {
        let manifest = match get_manifest(&tc.tool_name) {
            Some(m) => m,
            None => return false,
        };

        requires_confirmation(manifest.confirmation, self.approval_mode)
    }
}

pub(super) fn requires_confirmation(policy: ConfirmationPolicy, approval_mode: ApprovalMode) -> bool {
    match (policy, approval_mode) {
        (ConfirmationPolicy::Never, _) => false,
        (ConfirmationPolicy::SensitiveWrite, ApprovalMode::ConfirmWrites) => true,
        (ConfirmationPolicy::SensitiveWrite, ApprovalMode::Auto) => false,
        (ConfirmationPolicy::Always, _) => true,
    }
}
