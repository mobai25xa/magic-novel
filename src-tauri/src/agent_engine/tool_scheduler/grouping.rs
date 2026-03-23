use crate::agent_tools::registry::get_manifest;

use super::super::types::ToolCallInfo;

/// Groups of tool calls for execution ordering
pub(super) enum ExecGroup {
    /// Tools that can run in parallel (all parallel_safe)
    Parallel(Vec<ToolCallInfo>),
    /// A single tool that must run sequentially (not parallel_safe)
    Sequential(ToolCallInfo),
}

/// Flush-on-sequential grouping algorithm.
///
/// Groups tool calls into parallel batches (all parallel_safe) and sequential singles.
pub(super) fn group_calls(calls: &[ToolCallInfo]) -> Vec<ExecGroup> {
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
