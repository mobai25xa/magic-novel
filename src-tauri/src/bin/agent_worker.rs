//! Worker process binary for Mission system.
//!
//! Reads NDJSON instructions from stdin, executes agent tasks, writes events to stdout.
//! Tracing output goes to stderr (stdout is the protocol channel).
//!
//! Based on docs/magic_plan/plan_agent_parallel/03-dev3-mission-owner.md

use std::io::{self, Write};
use std::sync::Arc;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio_util::sync::CancellationToken;

use magic_novel_lib::agent_engine::emitter::StdoutEventSink;
use magic_novel_lib::agent_engine::loop_engine::AgentLoop;
use magic_novel_lib::agent_engine::messages::{AgentMessage, ConversationState};
use magic_novel_lib::agent_engine::types::{LoopConfig, StopReason};
use magic_novel_lib::llm::router::RetryConfig;
use magic_novel_lib::llm::router_factory::build_router;
use magic_novel_lib::llm::streaming_turn::StreamingTurnEngine;
use magic_novel_lib::mission::agent_profile::{AgentProfile, SessionSource};
use magic_novel_lib::mission::result_types::{
    AgentTaskResult, ArtifactRef, ChangedPath, ChangedPathKind, EvidenceItem, OpenIssue,
    TaskResultStatus, TaskStopReason, TaskUsage,
};
use magic_novel_lib::mission::types::INTEGRATOR_FEATURE_ID;
use magic_novel_lib::mission::worker_profile::{
    builtin_general_worker_profile, builtin_integrator_worker_profile,
};
use magic_novel_lib::mission::worker_protocol::*;

#[derive(Clone)]
struct WorkerState {
    worker_id: String,
    project_path: String,
    mission_dir: String,
    initialized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstructionParseDiagnostic {
    ack_error: String,
    terminate_worker: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandAckDiagnostic {
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyInstructionPayload {}

#[tokio::main]
async fn main() {
    // Init tracing to stderr (stdout is protocol channel)
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .init();

    tracing::info!(target: "agent_worker", "worker process starting");

    let state: Arc<TokioMutex<Option<WorkerState>>> = Arc::new(TokioMutex::new(None));
    let cancel_token_holder: Arc<TokioMutex<Option<CancellationToken>>> =
        Arc::new(TokioMutex::new(None));

    let (feature_tx, mut feature_rx) = mpsc::channel::<StartFeaturePayload>(1);
    let (result_tx, mut result_rx) = mpsc::channel::<(String, String, AgentTaskResult)>(1);

    let state_exec = Arc::clone(&state);
    let cancel_exec = Arc::clone(&cancel_token_holder);
    tokio::spawn(async move {
        while let Some(payload) = feature_rx.recv().await {
            let ws = {
                let guard = state_exec.lock().await;
                guard.clone()
            };

            let Some(ws) = ws else {
                continue;
            };

            tracing::info!(
                target: "agent_worker",
                worker_id = %ws.worker_id,
                feature_id = %payload.feature.id,
                model = %payload.model,
                provider = %payload.provider,
                "executing feature"
            );

            let cancel_token = CancellationToken::new();
            {
                let mut guard = cancel_exec.lock().await;
                *guard = Some(cancel_token.clone());
            }

            let feature_id = payload.feature.id.clone();
            let session_id = payload.session_id.clone();
            let worker_id = ws.worker_id.clone();
            let result = match execute_feature(&ws, &payload, cancel_token).await {
                Ok(result) => result,
                Err(e) => {
                    let err_msg = format!("{e}");
                    tracing::error!(
                        target: "agent_worker",
                        feature_id = %feature_id,
                        error = %err_msg,
                        "feature execution failed"
                    );
                    build_failed_task_result(
                        &feature_id,
                        &worker_id,
                        &payload.feature.description,
                        TaskResultStatus::Failed,
                        TaskStopReason::Error,
                        format!("error: {err_msg}"),
                        vec![OpenIssue {
                            code: Some("E_WORKER_EXECUTION_FAILED".to_string()),
                            summary: err_msg,
                            blocking: true,
                        }],
                    )
                }
            };

            {
                let mut guard = cancel_exec.lock().await;
                *guard = None;
            }

            if result_tx
                .send((feature_id, session_id, result))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        tokio::select! {
            maybe_result = result_rx.recv() => {
                let Some((feature_id, session_id, result)) = maybe_result else {
                    break;
                };
                send_event(&WorkerEvent::feature_completed(&feature_id, session_id, result));
            }
            line_result = lines.next_line() => {
                let line = match line_result {
                    Ok(Some(line)) => line,
                    Ok(None) => break,
                    Err(e) => {
                        tracing::error!(target: "agent_worker", error = %e, "stdin read error");
                        break;
                    }
                };

                if line.trim().is_empty() {
                    continue;
                }

                let instruction = match WorkerInstruction::from_ndjson_line(&line) {
                    Ok(i) => i,
                    Err(e) => {
                        let error_message = format!("{e}");
                        let diagnostic = classify_instruction_parse_error(&error_message);
                        tracing::warn!(
                            target: "agent_worker",
                            line = %line,
                            error = %e,
                            "failed to parse instruction"
                        );
                        send_event(&WorkerEvent::ack(
                            "err",
                            false,
                            Some(diagnostic.ack_error.clone()),
                        ));
                        if diagnostic.terminate_worker {
                            tracing::error!(
                                target: "agent_worker",
                                error = %error_message,
                                "protocol incompatibility detected, terminating worker"
                            );
                            break;
                        }
                        continue;
                    }
                };

                match instruction.instruction_type {
                    InstructionType::Initialize => {
                        let payload: InitializePayload = match serde_json::from_value(instruction.payload) {
                            Ok(p) => p,
                            Err(e) => {
                                send_event(&WorkerEvent::ack(
                                    &instruction.id,
                                    false,
                                    Some(payload_parse_error("initialize", &e)),
                                ));
                                continue;
                            }
                        };

                        tracing::info!(
                            target: "agent_worker",
                            worker_id = %payload.worker_id,
                            project_path = %payload.project_path,
                            "initialized"
                        );

                        {
                            let mut guard = state.lock().await;
                            *guard = Some(WorkerState {
                                worker_id: payload.worker_id,
                                project_path: payload.project_path,
                                mission_dir: payload.mission_dir,
                                initialized: true,
                            });
                        }

                        send_event(&WorkerEvent::ack(&instruction.id, true, None));
                    }

                    InstructionType::StartFeature => {
                        let initialized = {
                            let guard = state.lock().await;
                            guard.as_ref().map(|s| s.initialized).unwrap_or(false)
                        };

                        if !initialized {
                            send_event(&WorkerEvent::ack(
                                &instruction.id,
                                false,
                                Some(start_feature_not_initialized_error()),
                            ));
                            continue;
                        }

                        let payload: StartFeaturePayload = match serde_json::from_value(instruction.payload)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                send_event(&WorkerEvent::ack(
                                    &instruction.id,
                                    false,
                                    Some(payload_parse_error("start_feature", &e)),
                                ));
                                continue;
                            }
                        };

                        match feature_tx.try_send(payload) {
                            Ok(()) => send_event(&WorkerEvent::ack(&instruction.id, true, None)),
                            Err(error) => send_event(&WorkerEvent::ack(
                                &instruction.id,
                                false,
                                Some(start_feature_dispatch_error(&error)),
                            )),
                        }
                    }

                    InstructionType::Cancel => {
                        let payload: CancelPayload = match serde_json::from_value(instruction.payload) {
                            Ok(payload) => payload,
                            Err(error) => {
                                send_event(&WorkerEvent::ack(
                                    &instruction.id,
                                    false,
                                    Some(payload_parse_error("cancel", &error)),
                                ));
                                continue;
                            }
                        };
                        let turn_id = payload.turn_id;
                        tracing::info!(
                            target: "agent_worker",
                            turn_id = ?turn_id,
                            "cancel received"
                        );
                        let has_running_feature = {
                            let guard = cancel_token_holder.lock().await;
                            if let Some(token) = guard.as_ref() {
                                token.cancel();
                                true
                            } else {
                                false
                            }
                        };
                        if has_running_feature {
                            tracing::info!(
                                target: "agent_worker",
                                turn_id = ?turn_id,
                                "cancellation token signalled"
                            );
                        } else {
                            tracing::warn!(
                                target: "agent_worker",
                                turn_id = ?turn_id,
                                "cancel rejected because no feature is running"
                            );
                        }
                        let ack = cancel_ack_diagnostic(turn_id, has_running_feature);
                        send_event(&WorkerEvent::ack(&instruction.id, ack.ok, ack.error));
                    }

                    InstructionType::Shutdown => {
                        if let Err(error) =
                            validate_empty_instruction_payload("shutdown", instruction.payload)
                        {
                            send_event(&WorkerEvent::ack(&instruction.id, false, Some(error)));
                            continue;
                        }
                        let had_running_feature = {
                            let guard = cancel_token_holder.lock().await;
                            if let Some(token) = guard.as_ref() {
                                token.cancel();
                                true
                            } else {
                                false
                            }
                        };
                        tracing::info!(
                            target: "agent_worker",
                            had_running_feature = had_running_feature,
                            "{}",
                            shutdown_diagnostic_message(had_running_feature)
                        );
                        send_event(&WorkerEvent::ack(&instruction.id, true, None));
                        break;
                    }

                    InstructionType::Ping => {
                        send_event(&WorkerEvent::pong(&instruction.id));
                    }
                }
            }
        }
    }

    tracing::info!(target: "agent_worker", "worker process exiting");
}

/// Execute a single feature by running the full AgentLoop with a StdoutEventSink.
///
/// Uses `StreamingTurnEngine` with the same provider/router stack as main agent flow.
/// Events are written to stdout as WorkerEvent::AgentEvent NDJSON lines.
async fn execute_feature(
    ws: &WorkerState,
    payload: &StartFeaturePayload,
    cancel_token: CancellationToken,
) -> Result<AgentTaskResult, Box<dyn std::error::Error + Send + Sync>> {
    let base_url = payload.base_url.trim().to_string();
    let api_key = payload.api_key.trim().to_string();
    if base_url.is_empty() || api_key.is_empty() {
        return Err("missing base_url/api_key in worker start payload".into());
    }

    let mission_id = payload.mission_id.trim();
    let mission_id = if mission_id.is_empty() {
        "unknown"
    } else {
        mission_id
    };
    let worker_id = payload.worker_id.trim();
    let worker_id = if worker_id.is_empty() {
        ws.worker_id.as_str()
    } else {
        worker_id
    };

    let agent_profile = resolve_agent_profile(payload);
    let effective_model = agent_profile
        .model
        .as_deref()
        .unwrap_or(payload.model.as_str())
        .trim()
        .to_string();

    // Build system prompt
    let system_prompt = format!(
        "{}\n\n[Mission Context]\nMission ID: {}\nWorker ID: {}\nFeature ID: {}\nFeature Description: {}\nProject Path: {}\nMission Dir: {}\nSession Source: {}\nCapability Preset: {}\n\nTool exposure is enforced by the runtime capability policy for this worker.\n\nPolicy: for multi-step changes, call todowrite first with milestone-level, user-verifiable tasks and keep one in_progress item.",
        agent_profile.system_prompt(),
        mission_id,
        worker_id,
        payload.feature.id,
        payload.feature.description,
        ws.project_path,
        ws.mission_dir,
        payload.session_source.as_str(),
        agent_profile.capability_preset.as_str(),
    );

    // Build user message
    let user_text =
        format!(
        "Please complete this feature:\n{}\n\nExpected behavior:\n{}\n\nVerification steps:\n{}",
        payload.feature.description,
        payload.feature
            .expected_behavior
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        payload.feature
            .verification_steps
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Build ConversationState
    let mut conv = ConversationState::new(payload.session_id.clone());
    conv.messages.push(AgentMessage::system(system_prompt));
    conv.messages.push(AgentMessage::user(user_text));

    // Infer active chapter path (best-effort)
    let active_chapter_path = infer_active_chapter_path(&payload.feature.write_paths);

    // Build StdoutEventSink (turn_id = 1 for worker-initiated turns)
    let sink = StdoutEventSink::new(
        payload.session_id.clone(),
        1,
        mission_id.to_string(),
        worker_id.to_string(),
    );

    // Emit turn_started
    use magic_novel_lib::agent_engine::emitter::EventSink;
    sink.turn_started(
        &payload.feature.description,
        &payload.provider,
        &effective_model,
    )
    .ok();

    let loop_config = LoopConfig {
        max_rounds: agent_profile.max_rounds,
        max_tool_calls: agent_profile.max_tool_calls,
        autonomy_level: agent_profile.approval_mode.to_autonomy_level(),
        capability_mode: agent_profile.mode,
        approval_mode: agent_profile.approval_mode,
        clarification_mode: agent_profile.clarification_mode,
    };

    // Build and run AgentLoop
    let agent_loop = AgentLoop::new(
        sink.clone(),
        loop_config,
        ws.project_path.clone(),
        cancel_token.clone(),
    )
    .with_mission_id(Some(mission_id.to_string()))
    .with_provider_info(
        payload.provider.clone(),
        effective_model.clone(),
        base_url.clone(),
        api_key.clone(),
    )
    .with_capability_policy(agent_profile.capability_policy())
    .with_session_source(payload.session_source)
    .with_delegate_depth(
        if matches!(payload.session_source, SessionSource::Delegate) {
            1
        } else {
            0
        },
    )
    .with_active_chapter_path(active_chapter_path)
    .with_active_skill(
        Some(payload.feature.skill.clone())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
    );

    let router = build_router(&payload.provider, base_url, api_key, RetryConfig::worker());
    let streaming_engine =
        StreamingTurnEngine::new(router, sink, payload.provider.clone(), effective_model)
            .with_cancel_token(cancel_token);

    let loop_result = agent_loop.run(&mut conv, &streaming_engine).await?;

    tracing::info!(
        target: "agent_worker",
        feature_id = %payload.feature.id,
        stop_reason = ?loop_result.stop_reason,
        rounds = loop_result.rounds_executed,
        tool_calls = loop_result.total_tool_calls,
        latency_ms = loop_result.latency_ms,
        "feature AgentLoop completed"
    );

    Ok(build_task_result(
        payload,
        worker_id,
        &conv,
        &loop_result.stop_reason,
        loop_result.rounds_executed,
        loop_result.total_tool_calls,
        loop_result.latency_ms,
    ))
}

fn infer_active_chapter_path(write_paths: &[String]) -> Option<String> {
    if write_paths.len() != 1 {
        return None;
    }

    let raw = write_paths[0].trim();
    if raw.is_empty() {
        return None;
    }

    let normalized = raw.replace('\\', "/");
    if normalized.ends_with(".json") {
        Some(normalized)
    } else {
        None
    }
}

fn resolve_agent_profile(payload: &StartFeaturePayload) -> AgentProfile {
    payload
        .agent_profile
        .clone()
        .unwrap_or_else(|| default_worker_profile_for_payload(payload))
}

fn default_worker_profile_for_payload(payload: &StartFeaturePayload) -> AgentProfile {
    if payload.feature.id == INTEGRATOR_FEATURE_ID
        || payload
            .feature
            .skill
            .trim()
            .eq_ignore_ascii_case("integrator")
    {
        builtin_integrator_worker_profile()
    } else {
        builtin_general_worker_profile()
    }
}

fn build_task_result(
    payload: &StartFeaturePayload,
    worker_id: &str,
    conv: &ConversationState,
    stop_reason: &StopReason,
    rounds_executed: u32,
    total_tool_calls: u32,
    latency_ms: u64,
) -> AgentTaskResult {
    let (status, mapped_stop_reason) = map_stop_reason(stop_reason);
    let summary = assistant_summary(conv).unwrap_or_else(|| {
        format!(
            "Feature '{}' stopped after {} rounds ({} tool calls)",
            payload.feature.id, rounds_executed, total_tool_calls
        )
    });

    let mut open_issues = Vec::new();
    if !matches!(status, TaskResultStatus::Completed) {
        open_issues.push(OpenIssue {
            code: Some(format!("worker::{mapped_stop_reason:?}").to_ascii_lowercase()),
            summary: summary.clone(),
            blocking: true,
        });
    }

    AgentTaskResult {
        task_id: payload.feature.id.clone(),
        actor_id: worker_id.to_string(),
        goal: payload.feature.description.clone(),
        status,
        stop_reason: mapped_stop_reason,
        result_summary: summary.clone(),
        changed_paths: payload
            .feature
            .write_paths
            .iter()
            .map(|path| ChangedPath {
                path: path.trim().to_string(),
                change_kind: ChangedPathKind::Modified,
            })
            .filter(|path| !path.path.is_empty())
            .collect(),
        artifacts: payload
            .feature
            .write_paths
            .iter()
            .map(|path| ArtifactRef {
                kind: "write_path".to_string(),
                value: path.trim().to_string(),
                description: None,
            })
            .filter(|artifact| !artifact.value.is_empty())
            .collect(),
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
    }
}

fn build_failed_task_result(
    feature_id: &str,
    worker_id: &str,
    goal: &str,
    status: TaskResultStatus,
    stop_reason: TaskStopReason,
    summary: String,
    open_issues: Vec<OpenIssue>,
) -> AgentTaskResult {
    AgentTaskResult {
        task_id: feature_id.to_string(),
        actor_id: worker_id.to_string(),
        goal: goal.to_string(),
        status,
        stop_reason,
        result_summary: summary.clone(),
        evidence: vec![EvidenceItem {
            kind: "worker_error".to_string(),
            summary,
            value: None,
        }],
        open_issues,
        ..AgentTaskResult::default()
    }
}

fn assistant_summary(conv: &ConversationState) -> Option<String> {
    conv.messages
        .iter()
        .rev()
        .find(|m| {
            matches!(
                m.role,
                magic_novel_lib::agent_engine::messages::Role::Assistant
            )
        })
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
        (TaskResultStatus::Failed, _) => vec!["inspect worker summary and retry".to_string()],
        (TaskResultStatus::Blocked, TaskStopReason::Limit) => {
            vec!["adjust limits or split the feature".to_string()]
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

fn is_protocol_incompatibility(error_message: &str) -> bool {
    error_message
        .to_ascii_lowercase()
        .contains("protocol schema mismatch")
}

fn classify_instruction_parse_error(error_message: &str) -> InstructionParseDiagnostic {
    if is_protocol_incompatibility(error_message) {
        InstructionParseDiagnostic {
            ack_error: format!("protocol incompatibility: {error_message}"),
            terminate_worker: true,
        }
    } else {
        InstructionParseDiagnostic {
            ack_error: format!("instruction parse error: {error_message}"),
            terminate_worker: false,
        }
    }
}

fn payload_parse_error(instruction_name: &str, error: &serde_json::Error) -> String {
    format!("{instruction_name} payload parse error: {error}")
}

fn start_feature_not_initialized_error() -> String {
    "start_feature rejected: worker is not initialized".to_string()
}

fn start_feature_dispatch_error<T>(error: &TrySendError<T>) -> String {
    match error {
        TrySendError::Full(_) => {
            "start_feature rejected: worker is busy executing another feature".to_string()
        }
        TrySendError::Closed(_) => {
            "start_feature rejected: worker execution loop is unavailable".to_string()
        }
    }
}

fn cancel_ack_diagnostic(
    requested_turn_id: Option<u32>,
    has_running_feature: bool,
) -> CommandAckDiagnostic {
    if has_running_feature {
        CommandAckDiagnostic {
            ok: true,
            error: None,
        }
    } else {
        let context = requested_turn_id
            .map(|turn_id| format!(" (requested turn_id={turn_id})"))
            .unwrap_or_default();
        CommandAckDiagnostic {
            ok: false,
            error: Some(format!(
                "cancel rejected: no active feature is running{context}"
            )),
        }
    }
}

fn validate_empty_instruction_payload(
    instruction_name: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    serde_json::from_value::<EmptyInstructionPayload>(payload)
        .map(|_| ())
        .map_err(|error| payload_parse_error(instruction_name, &error))
}

fn shutdown_diagnostic_message(had_running_feature: bool) -> &'static str {
    if had_running_feature {
        "shutdown received while feature is running; signalling cancellation before exit"
    } else {
        "shutdown received while idle; exiting"
    }
}

/// Write a WorkerEvent as NDJSON to stdout with mandatory flush.
fn send_event(event: &WorkerEvent) {
    match serde_json::to_string(event) {
        Ok(line) => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(line.as_bytes());
            let _ = handle.write_all(b"\n");
            // CRITICAL: must flush per protocol spec
            let _ = handle.flush();
        }
        Err(e) => {
            tracing::error!(target: "agent_worker", error = %e, "failed to serialize event");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_instruction_parse_error_marks_protocol_incompatibility() {
        let diagnostic =
            classify_instruction_parse_error("protocol schema mismatch: expected 1, got 2");

        assert_eq!(
            diagnostic.ack_error,
            "protocol incompatibility: protocol schema mismatch: expected 1, got 2"
        );
        assert!(diagnostic.terminate_worker);
    }

    #[test]
    fn classify_instruction_parse_error_marks_plain_parse_error() {
        let diagnostic = classify_instruction_parse_error("expected value at line 1 column 2");

        assert_eq!(
            diagnostic.ack_error,
            "instruction parse error: expected value at line 1 column 2"
        );
        assert!(!diagnostic.terminate_worker);
    }

    #[test]
    fn payload_parse_error_includes_instruction_name() {
        let error = serde_json::from_str::<StartFeaturePayload>("{\"broken\"").unwrap_err();

        assert!(payload_parse_error("start_feature", &error)
            .starts_with("start_feature payload parse error:"));
    }

    #[test]
    fn start_feature_dispatch_error_distinguishes_busy_and_closed() {
        let (tx_full, mut rx_full) = mpsc::channel::<u8>(1);
        tx_full.try_send(1).unwrap();
        let full_error = tx_full.try_send(2).unwrap_err();
        assert_eq!(
            start_feature_dispatch_error(&full_error),
            "start_feature rejected: worker is busy executing another feature"
        );

        let _ = rx_full.try_recv();
        let (tx_closed, rx_closed) = mpsc::channel::<u8>(1);
        drop(rx_closed);
        let closed_error = tx_closed.try_send(1).unwrap_err();
        assert_eq!(
            start_feature_dispatch_error(&closed_error),
            "start_feature rejected: worker execution loop is unavailable"
        );
    }

    #[test]
    fn cancel_ack_diagnostic_rejects_idle_worker() {
        let diagnostic = cancel_ack_diagnostic(Some(7), false);

        assert!(!diagnostic.ok);
        assert_eq!(
            diagnostic.error.as_deref(),
            Some("cancel rejected: no active feature is running (requested turn_id=7)")
        );
    }

    #[test]
    fn cancel_ack_diagnostic_accepts_active_worker() {
        let diagnostic = cancel_ack_diagnostic(None, true);

        assert!(diagnostic.ok);
        assert!(diagnostic.error.is_none());
    }

    #[test]
    fn validate_empty_instruction_payload_rejects_unexpected_fields() {
        let error = validate_empty_instruction_payload("shutdown", json!({"reason": "stop"}))
            .expect_err("shutdown payload with fields should be rejected");

        assert!(error.starts_with("shutdown payload parse error:"));
    }

    #[test]
    fn shutdown_diagnostic_message_distinguishes_running_and_idle() {
        assert_eq!(
            shutdown_diagnostic_message(true),
            "shutdown received while feature is running; signalling cancellation before exit"
        );
        assert_eq!(
            shutdown_diagnostic_message(false),
            "shutdown received while idle; exiting"
        );
    }
}
