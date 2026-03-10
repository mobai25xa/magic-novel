//! Legacy todo-worker dispatch helpers.
//!
//! NOTE: The todo-worker execution path is retired in favor of mission-worker.
//! We keep parsing helpers so legacy `todowrite.todos[].worker` fields remain
//! backward-compatible, but we do not execute worker sub-loops here.

use super::types::ToolCallInfo;

/// Extract TodoState from the most recent todowrite tool call args.
pub(crate) fn extract_worker_todo_items(
    tool_calls: &[ToolCallInfo],
) -> Vec<super::todowrite::TodoItem> {
    for tc in tool_calls.iter().rev() {
        if tc.tool_name == "todowrite" {
            if let Ok(state) = super::todowrite::parse_todo_input(&tc.args, "worker_extract") {
                return state.items;
            }
        }
    }
    Vec::new()
}

/// Check if any pending todo items have worker assignments.
pub(crate) fn has_worker_items(items: &[super::todowrite::TodoItem]) -> bool {
    items
        .iter()
        .any(|i| i.status == "pending" && i.worker.is_some())
}
