//! Worker process binary for Mission system.
//!
//! Reads NDJSON instructions from stdin, executes agent tasks, writes events to stdout.
//! Tracing output goes to stderr (stdout is the protocol channel).
//!
//! Based on docs/magic_plan/plan_agent_parallel/03-dev3-mission-owner.md

use std::io::{self, Write};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio_util::sync::CancellationToken;

use magic_novel_lib::agent_engine::emitter::StdoutEventSink;
use magic_novel_lib::agent_engine::loop_engine::AgentLoop;
use magic_novel_lib::agent_engine::messages::{AgentMessage, ConversationState};
use magic_novel_lib::agent_engine::types::LoopConfig;
use magic_novel_lib::llm::router::RetryConfig;
use magic_novel_lib::llm::router_factory::build_router;
use magic_novel_lib::llm::streaming_turn::StreamingTurnEngine;
use magic_novel_lib::mission::types::{HandoffEntry, INTEGRATOR_FEATURE_ID};
use magic_novel_lib::mission::worker_profile::{
    builtin_general_worker_profile, builtin_integrator_worker_profile, WorkerProfile,
};
use magic_novel_lib::mission::worker_protocol::*;

#[derive(Clone)]
struct WorkerState {
    worker_id: String,
    project_path: String,
    mission_dir: String,
    initialized: bool,
}

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
    let (result_tx, mut result_rx) = mpsc::channel::<(String, String, HandoffEntry)>(1);

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
            let cancel_check = cancel_token.clone();
            let result = match execute_feature(&ws, &payload, cancel_token).await {
                Ok(_summary) if cancel_check.is_cancelled() => Err("turn cancelled".into()),
                other => other,
            };

            {
                let mut guard = cancel_exec.lock().await;
                *guard = None;
            }

            let handoff = match result {
                Ok(summary) => HandoffEntry {
                    feature_id: feature_id.clone(),
                    worker_id,
                    ok: true,
                    summary,
                    commands_run: Vec::new(),
                    artifacts: Vec::new(),
                    issues: Vec::new(),
                },
                Err(e) => {
                    let err_msg = format!("{e}");
                    tracing::error!(
                        target: "agent_worker",
                        feature_id = %feature_id,
                        error = %err_msg,
                        "feature execution failed"
                    );
                    HandoffEntry {
                        feature_id: feature_id.clone(),
                        worker_id,
                        ok: false,
                        summary: format!("error: {err_msg}"),
                        commands_run: Vec::new(),
                        artifacts: Vec::new(),
                        issues: vec![err_msg],
                    }
                }
            };

            if result_tx.send((feature_id, session_id, handoff)).await.is_err() {
                break;
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        tokio::select! {
            maybe_result = result_rx.recv() => {
                let Some((feature_id, session_id, handoff)) = maybe_result else {
                    break;
                };
                let ok = handoff.ok;
                send_event(&WorkerEvent::feature_completed(
                    &feature_id,
                    ok,
                    session_id,
                    handoff,
                ));
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
                        tracing::warn!(
                            target: "agent_worker",
                            line = %line,
                            error = %e,
                            "failed to parse instruction"
                        );
                        send_event(&WorkerEvent::ack(
                            "err",
                            false,
                            Some(format!("parse error: {e}")),
                        ));
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
                                    Some(format!("payload parse error: {e}")),
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
                                Some("not initialized".to_string()),
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
                                    Some(format!("payload parse error: {e}")),
                                ));
                                continue;
                            }
                        };

                        match feature_tx.try_send(payload) {
                            Ok(()) => send_event(&WorkerEvent::ack(&instruction.id, true, None)),
                            Err(_) => send_event(&WorkerEvent::ack(
                                &instruction.id,
                                false,
                                Some("worker is busy executing another feature".to_string()),
                            )),
                        }
                    }

                    InstructionType::Cancel => {
                        tracing::info!(target: "agent_worker", "cancel received");
                        let guard = cancel_token_holder.lock().await;
                        if let Some(token) = guard.as_ref() {
                            token.cancel();
                            tracing::info!(target: "agent_worker", "cancellation token signalled");
                        } else {
                            tracing::info!(target: "agent_worker", "cancel received but no feature running");
                        }
                        send_event(&WorkerEvent::ack(&instruction.id, true, None));
                    }

                    InstructionType::Shutdown => {
                        tracing::info!(target: "agent_worker", "shutdown received, exiting");
                        let guard = cancel_token_holder.lock().await;
                        if let Some(token) = guard.as_ref() {
                            token.cancel();
                        }
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
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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

    let mut worker_profile: WorkerProfile = payload.worker_profile.clone().unwrap_or_else(|| {
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
    });
    if worker_profile.tool_whitelist.is_empty() {
        worker_profile.tool_whitelist = builtin_general_worker_profile().tool_whitelist;
    }

    let effective_model = worker_profile
        .model
        .as_deref()
        .unwrap_or(payload.model.as_str())
        .trim()
        .to_string();

    let tools_list = if worker_profile.tool_whitelist.is_empty() {
        "(none)".to_string()
    } else {
        worker_profile.tool_whitelist.join(", ")
    };

    // Build system prompt
    let system_prompt = format!(
        "{}\n\n[Mission Context]\nMission ID: {}\nWorker ID: {}\nFeature ID: {}\nFeature Description: {}\nProject Path: {}\nMission Dir: {}\n\nAvailable tools: {}\n\nPolicy: for multi-step changes, call todowrite first with milestone-level, user-verifiable tasks and keep one in_progress item.",
        worker_profile.system_prompt.trim(),
        mission_id,
        worker_id,
        payload.feature.id,
        payload.feature.description,
        ws.project_path,
        ws.mission_dir,
        tools_list,
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

    // Build loop config (auto approval + headless clarification for worker execution)
    let loop_config =
        LoopConfig::headless_worker(worker_profile.max_rounds, worker_profile.max_tool_calls);

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
    .with_tool_whitelist(Some(worker_profile.tool_whitelist.clone()))
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

    // Build summary from conversation: extract last assistant text
    let summary = conv
        .messages
        .iter()
        .rev()
        .find(|m| {
            matches!(
                m.role,
                magic_novel_lib::agent_engine::messages::Role::Assistant
            )
        })
        .map(|m| m.text_content())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            format!(
                "Feature '{}' completed in {} rounds ({} tool calls)",
                payload.feature.id, loop_result.rounds_executed, loop_result.total_tool_calls,
            )
        });

    Ok(summary)
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
