//! Orchestrator-Worker dispatch: runs sub-loops for worker-assigned todo items.

use tokio_util::sync::CancellationToken;

use crate::models::AppError;

use super::emitter::EventSink;
use super::loop_engine::AgentLoop;
use super::messages::{AgentMessage, ConversationState, Role};
use super::types::{LoopConfig, StopReason, ToolCallInfo};

#[derive(Debug, Clone)]
pub(crate) struct WorkerResult {
    pub(crate) task_text: String,
    pub(crate) worker_name: String,
    pub(crate) ok: bool,
    pub(crate) summary: String,
}

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

/// Dispatch pending worker-assigned todo items to sub-AgentLoops.
pub(crate) async fn dispatch_worker_items<S: EventSink + Clone + 'static>(
    items: &[super::todowrite::TodoItem],
    emitter: &S,
    project_path: &str,
    provider_name: &str,
    model: &str,
    base_url: &str,
    api_key: &str,
    active_chapter_path: Option<&str>,
    cancel_token: &CancellationToken,
) -> Result<Vec<WorkerResult>, AppError> {
    let worker_defs = crate::services::global_config::load_worker_definitions();
    let mut results = Vec::new();

    for item in items {
        if item.status != "pending" {
            continue;
        }
        let worker_name = match &item.worker {
            Some(w) => w,
            None => continue,
        };

        if cancel_token.is_cancelled() {
            results.push(WorkerResult {
                task_text: item.text.clone(),
                worker_name: worker_name.clone(),
                ok: false,
                summary: "Cancelled".to_string(),
            });
            break;
        }

        let worker_def = match worker_defs.iter().find(|d| d.name == *worker_name) {
            Some(d) => d.clone(),
            None => {
                results.push(WorkerResult {
                    task_text: item.text.clone(),
                    worker_name: worker_name.clone(),
                    ok: false,
                    summary: format!("Worker '{}' not found in ~/.magic/workers/", worker_name),
                });
                continue;
            }
        };

        let result = run_worker_sub_loop(
            &worker_def,
            &item.text,
            emitter,
            project_path,
            provider_name,
            model,
            base_url,
            api_key,
            active_chapter_path,
            cancel_token,
        )
        .await;

        results.push(result);
    }

    Ok(results)
}

/// Run a single worker sub-loop with its own system prompt and tool whitelist.
async fn run_worker_sub_loop<S: EventSink + Clone + 'static>(
    worker_def: &crate::services::global_config::WorkerDefinition,
    task_text: &str,
    emitter: &S,
    project_path: &str,
    provider_name: &str,
    model: &str,
    base_url: &str,
    api_key: &str,
    active_chapter_path: Option<&str>,
    cancel_token: &CancellationToken,
) -> WorkerResult {
    let effective_model = worker_def.model.as_deref().unwrap_or(model);

    let worker_config = LoopConfig::headless_worker(
        worker_def.max_rounds.unwrap_or(10),
        worker_def.max_tool_calls.unwrap_or(30),
    );

    let worker_loop = AgentLoop::new(
        emitter.clone(),
        worker_config,
        project_path.to_string(),
        cancel_token.clone(),
    )
    .with_provider_info(
        provider_name.to_string(),
        effective_model.to_string(),
        base_url.to_string(),
        api_key.to_string(),
    )
    .with_active_chapter_path(active_chapter_path.map(String::from))
    .with_tool_whitelist(Some(worker_def.tool_whitelist.clone()));

    // Build worker conversation with its own system prompt
    let session_id = format!("worker_{}_{}", worker_def.name, uuid::Uuid::new_v4());
    let mut worker_state = ConversationState::new(session_id);
    worker_state
        .messages
        .push(AgentMessage::system(worker_def.system_prompt.clone()));
    worker_state
        .messages
        .push(AgentMessage::user(task_text.to_string()));

    // Build streaming engine for the worker
    let retry_config = crate::llm::router::RetryConfig::worker();
    let router = crate::llm::router_factory::build_router(
        provider_name,
        base_url.to_string(),
        api_key.to_string(),
        retry_config,
    );
    let streaming_engine = crate::llm::streaming_turn::StreamingTurnEngine::new(
        router,
        emitter.clone(),
        provider_name.to_string(),
        effective_model.to_string(),
    )
    .with_cancel_token(cancel_token.clone());

    tracing::info!(
        target: "agent_engine",
        worker = %worker_def.name,
        task = %task_text,
        model = %effective_model,
        tools = ?worker_def.tool_whitelist,
        "starting worker sub-loop"
    );

    match Box::pin(worker_loop.run(&mut worker_state, &streaming_engine)).await {
        Ok(result) => {
            // Extract the final assistant text as summary
            let assistant_text = worker_state
                .messages
                .iter()
                .rev()
                .find(|m| m.role == Role::Assistant)
                .map(|m| m.text_content())
                .unwrap_or_default();
            let summary = if assistant_text.len() > 500 {
                format!("{}...", &assistant_text[..500])
            } else if assistant_text.is_empty() {
                format!(
                    "Completed: {} rounds, {} tool calls",
                    result.rounds_executed, result.total_tool_calls
                )
            } else {
                assistant_text
            };

            WorkerResult {
                task_text: task_text.to_string(),
                worker_name: worker_def.name.clone(),
                ok: matches!(result.stop_reason, StopReason::Success),
                summary,
            }
        }
        Err(e) => WorkerResult {
            task_text: task_text.to_string(),
            worker_name: worker_def.name.clone(),
            ok: false,
            summary: format!("Worker failed: {}", e.message),
        },
    }
}

/// Format worker results as a system message for the main loop.
pub(crate) fn format_worker_results(results: &[WorkerResult]) -> String {
    let lines: Vec<String> = results
        .iter()
        .map(|r| {
            let status = if r.ok { "OK" } else { "FAIL" };
            format!(
                "- [{}] worker={}, task=\"{}\": {}",
                status, r.worker_name, r.task_text, r.summary
            )
        })
        .collect();
    format!("[Worker Results]\n{}", lines.join("\n"))
}
