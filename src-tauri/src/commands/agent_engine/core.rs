//! Tauri commands for Agent Engine v2
//!
//! - agent_turn_start: initiate a new agent turn
//! - agent_turn_cancel: cancel a running turn
//! - agent_turn_resume: resume a suspended turn (confirmation/askuser)

use serde::{Deserialize, Serialize};
use tauri::command;
use tokio_util::sync::CancellationToken;

use crate::agent_engine::emitter::{AgentEventEmitter, EventSink};
use crate::agent_engine::loop_engine::AgentLoop;
use crate::agent_engine::messages::{AgentMessage, ContentBlock, ConversationState, Role};
use crate::agent_engine::persistence::SessionPersistenceSink;
use crate::agent_engine::prompt_assembler::role_layer::PromptRole;
use crate::agent_engine::session_state;
use crate::agent_engine::tool_formatters::{build_tool_message, build_tool_trace};
use crate::agent_engine::tool_routing::resolve_turn_tool_exposure;
use crate::agent_engine::turn::OpenAiDirectTurnEngine;
use crate::agent_engine::types::{
    AgentMode, AgentTurnStartResult, ApprovalMode, ClarificationMode, LoopConfig, ResumeInput,
    StopReason, DEFAULT_PROVIDER,
};
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::streaming_turn::StreamingTurnEngine;
use crate::models::{AppError, ErrorCode};
use crate::services::{
    load_openai_search_settings, save_runtime_snapshot_from_input, RuntimeSnapshotUpsertInput,
    SessionRuntimeState,
};

use super::common::{build_loop_config, resolve_turn_provider_config};

#[cfg(test)]
fn default_system_prompt() -> &'static str {
    super::prompt::default_system_prompt()
}

#[cfg(test)]
fn apply_system_prompt(state: &mut ConversationState, system_prompt: Option<&str>) {
    super::prompt::apply_system_prompt(state, system_prompt)
}

fn apply_system_prompt_with_config(
    state: &mut ConversationState,
    system_prompt: Option<&str>,
    loop_config: &crate::agent_engine::types::LoopConfig,
    provider: Option<&str>,
    model: Option<&str>,
    role: Option<PromptRole>,
    reminder: Option<crate::agent_engine::prompt_assembler::reminder_layer::ReminderText>,
) {
    super::prompt::apply_system_prompt_with_config(
        state,
        system_prompt,
        loop_config,
        provider,
        model,
        role,
        reminder,
    )
}

fn parse_prompt_role(value: &str) -> Option<PromptRole> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("orchestrator") {
        Some(PromptRole::Orchestrator)
    } else if trimmed.eq_ignore_ascii_case("context") {
        Some(PromptRole::Context)
    } else if trimmed.eq_ignore_ascii_case("draft") {
        Some(PromptRole::Draft)
    } else if trimmed.eq_ignore_ascii_case("review") {
        Some(PromptRole::Review)
    } else if trimmed.eq_ignore_ascii_case("knowledge") {
        Some(PromptRole::Knowledge)
    } else {
        None
    }
}

/// Editor state snapshot passed from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEditorState {
    #[serde(default)]
    pub selected_text: Option<String>,
    #[serde(default)]
    pub cursor_paragraph: Option<String>,
    #[serde(default)]
    pub cursor_paragraph_index: Option<u32>,
    #[serde(default)]
    pub total_paragraphs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnStartInput {
    pub session_id: String,
    #[serde(default)]
    pub client_request_id: Option<String>,
    pub user_text: String,
    pub project_path: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    /// OpenAI-compatible base URL (must be provided by frontend settings)
    #[serde(default)]
    pub base_url: Option<String>,
    /// API key (must be provided by frontend settings)
    #[serde(default)]
    pub api_key: Option<String>,
    /// System prompt to prepend
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional prompt role override (orchestrator/context/draft/review/knowledge)
    #[serde(default)]
    pub prompt_role: Option<String>,
    /// Active chapter path used by askuser/edit context
    #[serde(default)]
    pub active_chapter_path: Option<String>,
    /// Session/orchestrator-provided active skill (preferred over legacy tool invocation)
    #[serde(default)]
    pub active_skill: Option<String>,
    #[serde(default)]
    pub capability_mode: Option<AgentMode>,
    #[serde(default)]
    pub approval_mode: Option<ApprovalMode>,
    #[serde(default)]
    pub clarification_mode: Option<ClarificationMode>,
    /// Editor state snapshot (selection, cursor position)
    #[serde(default)]
    pub editor_state: Option<AgentEditorState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnCancelInput {
    pub session_id: String,
    pub turn_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnResumeInput {
    pub session_id: String,
    pub turn_id: u32,
    pub resume_input: ResumeInput,
}

#[derive(Clone)]
struct SnapshotContext {
    project_path: String,
    session_id: String,
    provider_name: String,
    model: String,
    base_url: String,
    system_prompt: Option<String>,
    active_chapter_path: Option<String>,
    active_skill: Option<String>,
    loop_config: LoopConfig,
    session_canon_revision: Option<i64>,
    session_branch_id: Option<String>,
}

#[derive(Debug)]
struct HydratedSessionStart {
    conversation: ConversationState,
    hydration_status: String,
    next_turn_id: u32,
    active_skill: Option<String>,
}

#[derive(Debug)]
struct PreparedStartTurn {
    conversation: ConversationState,
    hydration_status: String,
    turn_id: u32,
    cancel_token: CancellationToken,
    active_skill: Option<String>,
}

#[derive(Debug)]
struct PreparedResumeTurn {
    suspended: session_state::SuspendedTurnState,
    turn_id: u32,
    cancel_token: CancellationToken,
}

struct ActiveTurnGuard {
    session_id: String,
    armed: bool,
}

impl ActiveTurnGuard {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ActiveTurnGuard {
    fn drop(&mut self) {
        if self.armed {
            session_state::global().clear_cancel_token(&self.session_id);
        }
    }
}

fn resolve_client_request_id(input: &AgentTurnStartInput) -> String {
    input
        .client_request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("client_req_{}", uuid::Uuid::new_v4()))
}

fn is_session_not_found_error(err: &AppError) -> bool {
    err.message == "session stream not found"
        && matches!(
            err.details
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(|value| value.as_str()),
            Some("E_AGENT_SESSION_NOT_FOUND")
        )
}

fn extract_pending_todo_count(state: &ConversationState) -> Option<usize> {
    // Find the most recent successful todowrite tool result and count pending + in_progress.
    for msg in state.messages.iter().rev() {
        if !matches!(msg.role, Role::Tool) {
            continue;
        }

        for block in &msg.blocks {
            let ContentBlock::ToolResult {
                tool_name,
                content,
                is_error,
                ..
            } = block
            else {
                continue;
            };

            if *is_error {
                continue;
            }

            let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
                continue;
            };

            let items = value
                .get("todo_state")
                .and_then(|v| v.get("items"))
                .and_then(|v| v.as_array());

            // Legacy compatibility: older snapshots may omit tool_name on tool_result blocks.
            // Treat it as todowrite only when the JSON shape matches.
            let is_todowrite = match tool_name.as_deref() {
                Some("todowrite") => true,
                Some(_) => false,
                None => items.is_some(),
            };
            if !is_todowrite {
                continue;
            }

            let Some(items) = items else { continue };

            let mut count = 0_usize;
            for item in items {
                match item.get("status").and_then(|v| v.as_str()) {
                    Some("pending") | Some("in_progress") => count = count.saturating_add(1),
                    _ => {}
                }
            }

            return Some(count);
        }
    }

    None
}

fn resolve_session_baseline(
    project_path: &std::path::Path,
    session_id: &str,
    current_canon_revision: Option<i64>,
    current_branch_id: &str,
) -> (Option<i64>, Option<String>) {
    let existing = crate::services::load_runtime_snapshot(project_path, session_id)
        .ok()
        .flatten();

    let session_canon_revision = existing
        .as_ref()
        .and_then(|s| s.session_canon_revision)
        .or(current_canon_revision);

    let session_branch_id = existing
        .as_ref()
        .and_then(|s| s.session_branch_id.clone())
        .or_else(|| Some(current_branch_id.to_string()));

    (session_canon_revision, session_branch_id)
}

fn stop_reason_to_runtime_state(reason: &StopReason) -> SessionRuntimeState {
    match reason {
        StopReason::Success => SessionRuntimeState::Completed,
        StopReason::Cancel => SessionRuntimeState::Cancelled,
        StopReason::Error => SessionRuntimeState::Failed,
        StopReason::Limit => SessionRuntimeState::Completed,
        StopReason::WaitingConfirmation => SessionRuntimeState::SuspendedConfirmation,
        StopReason::WaitingAskuser => SessionRuntimeState::SuspendedAskuser,
    }
}

fn is_suspension_stop_reason(reason: &StopReason) -> bool {
    matches!(
        reason,
        StopReason::WaitingConfirmation | StopReason::WaitingAskuser
    )
}

fn should_persist_explicit_resumed_state(resume_input: &ResumeInput) -> bool {
    matches!(resume_input, ResumeInput::Confirmation { .. })
}

fn resume_mode_label(resume_input: &ResumeInput) -> &'static str {
    match resume_input {
        ResumeInput::Confirmation { .. } => "confirmation",
        ResumeInput::AskUser { .. } => "askuser",
    }
}

fn build_confirmation_denied_trace(tool_name: &str, call_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 2,
        "stage": "result",
        "meta": {
            "tool": tool_name,
            "call_id": call_id,
            "duration_ms": 0,
        },
        "result": {
            "ok": false,
            "preview": {},
            "error": {
                "code": "E_TOOL_EXECUTION_DENIED",
                "message": "Tool execution denied by user.",
                "retryable": false,
                "fault_domain": "policy",
                "details": serde_json::Value::Null,
            },
        },
    })
}

fn persist_runtime_after_loop(
    snapshot_ctx: &SnapshotContext,
    session_id: &str,
    state: &ConversationState,
    stop_reason: &StopReason,
    active_skill: Option<String>,
) {
    if is_suspension_stop_reason(stop_reason) {
        tracing::info!(
            target: "agent_engine",
            session_id = %session_id,
            stop_reason = ?stop_reason,
            "loop suspended; preserving suspended runtime state"
        );
        return;
    }

    session_state::global().save_conversation(session_id, state.clone());
    persist_runtime_snapshot_for_conversation(
        snapshot_ctx,
        state,
        stop_reason_to_runtime_state(stop_reason),
        active_skill,
    );
}

fn persist_runtime_snapshot_for_conversation(
    ctx: &SnapshotContext,
    state: &ConversationState,
    runtime_state: SessionRuntimeState,
    active_skill: Option<String>,
) {
    let input = crate::services::RuntimeSnapshotUpsertInput::from_conversation(
        ctx.session_id.clone(),
        runtime_state,
        state.clone(),
        Some(state.current_turn),
        Some(ctx.provider_name.clone()),
        Some(ctx.model.clone()),
        Some(ctx.base_url.clone()),
        ctx.system_prompt.clone(),
        ctx.active_chapter_path.clone(),
        Some(ctx.loop_config.clone()),
    )
    .with_active_skill(active_skill.or_else(|| ctx.active_skill.clone()))
    .with_session_baseline(ctx.session_canon_revision, ctx.session_branch_id.clone());

    if let Err(err) =
        save_runtime_snapshot_from_input(std::path::Path::new(&ctx.project_path), input)
    {
        tracing::warn!(
            target: "agent_engine",
            session_id = %ctx.session_id,
            error = %err,
            "failed to persist runtime snapshot"
        );
    }
}

fn hydrate_existing_session_on_start(
    project_path: &str,
    session_id: &str,
) -> Result<HydratedSessionStart, AppError> {
    let path = std::path::Path::new(project_path);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::not_found("project_path not found"));
    }

    let runtime_available =
        crate::application::command_usecases::agent_session::hydrate_runtime_state(
            path,
            project_path,
            session_id,
        )?;

    if runtime_available.can_continue {
        let conversation = session_state::global()
            .take_conversation(session_id)
            .ok_or_else(|| AppError {
                code: ErrorCode::InvalidArgument,
                message: "hydrated session has no runtime conversation".to_string(),
                details: Some(serde_json::json!({
                    "code": "E_AGENT_SESSION_RUNTIME_UNAVAILABLE",
                    "session_id": session_id,
                    "hydrate_status": runtime_available.hydration_status,
                    "runtime_state": runtime_available.runtime_state,
                    "readonly_reason": runtime_available.readonly_reason,
                })),
                recoverable: Some(true),
            })?;

        let next_turn_id = runtime_available.next_turn_id.unwrap_or_else(|| {
            crate::agent_engine::session_state::derive_next_turn_id(
                runtime_available.last_turn,
                Some(&conversation),
                None,
            )
        });

        return Ok(HydratedSessionStart {
            conversation,
            hydration_status: runtime_available.hydration_status,
            next_turn_id,
            active_skill: runtime_available.active_skill,
        });
    }

    if runtime_available.can_resume {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "session is suspended and must be resumed, not started".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RESUME_NOT_SUPPORTED",
                "session_id": session_id,
                "hydrate_status": runtime_available.hydration_status,
                "runtime_state": runtime_available.runtime_state,
                "readonly_reason": runtime_available.readonly_reason,
            })),
            recoverable: Some(true),
        });
    }

    Err(AppError {
        code: ErrorCode::InvalidArgument,
        message: "session runtime is not available for continuation".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_UNAVAILABLE",
            "session_id": session_id,
            "hydrate_status": runtime_available.hydration_status,
            "runtime_state": runtime_available.runtime_state,
            "readonly_reason": runtime_available.readonly_reason,
        })),
        recoverable: Some(true),
    })
}

fn prepare_turn_start(
    project_path: &str,
    session_id: &str,
    client_request_id: &str,
) -> Result<PreparedStartTurn, AppError> {
    session_state::global().with_session_turn_lock(session_id, || {
        if session_state::global().has_active_turn(session_id) {
            return Err(AppError::invalid_argument(
                "A turn is already running for this session. Cancel it first or wait for completion.",
            ));
        }

        let hydrated = match hydrate_existing_session_on_start(project_path, session_id) {
            Ok(existing) => existing,
            Err(err) if is_session_not_found_error(&err) => HydratedSessionStart {
                conversation: ConversationState::new(session_id.to_string()),
                hydration_status: "event_rebuilt".to_string(),
                next_turn_id: 1,
                active_skill: None,
            },
            Err(err) => return Err(err),
        };

        session_state::global().seed_next_turn_id(session_id, hydrated.next_turn_id);
        let turn_id = session_state::global().next_turn_id(session_id);

        let cancel_token = CancellationToken::new();
        session_state::global().set_cancel_token(
            session_id,
            turn_id,
            cancel_token.clone(),
            Some(client_request_id.to_string()),
        );

        Ok(PreparedStartTurn {
            conversation: hydrated.conversation,
            hydration_status: hydrated.hydration_status,
            turn_id,
            cancel_token,
            active_skill: hydrated.active_skill,
        })
    })
}

fn prepare_resume_turn(session_id: &str) -> Result<PreparedResumeTurn, AppError> {
    session_state::global().with_session_turn_lock(session_id, || {
        if session_state::global().has_active_turn(session_id) {
            return Err(AppError::invalid_argument(
                "A turn is already running for this session. Cancel it first or wait for completion.",
            ));
        }

        let suspended = session_state::global().take_suspended(session_id).ok_or_else(|| {
            AppError::invalid_argument(format!(
                "no suspended turn for session '{}'",
                session_id
            ))
        })?;

        let turn_id = suspended.conversation_state.current_turn;
        let cancel_token = CancellationToken::new();
        session_state::global().set_cancel_token(session_id, turn_id, cancel_token.clone(), None);

        Ok(PreparedResumeTurn {
            suspended,
            turn_id,
            cancel_token,
        })
    })
}

/// Start a new agent turn.
///
/// Emits TURN_STARTED immediately and spawns the agent loop in background.
/// The loop emits streaming events via `magic:agent_event`.
#[command]
pub async fn agent_turn_start(
    app_handle: tauri::AppHandle,
    input: AgentTurnStartInput,
) -> Result<AgentTurnStartResult, AppError> {
    let session_id = input.session_id.clone();
    let client_request_id = resolve_client_request_id(&input);
    let prepared = prepare_turn_start(&input.project_path, &session_id, &client_request_id)?;
    let turn_id = prepared.turn_id;
    let cancel_token = prepared.cancel_token.clone();
    let hydration_status = prepared.hydration_status.clone();
    let active_skill = input
        .active_skill
        .clone()
        .or(prepared.active_skill.clone())
        .map(|skill| skill.trim().to_string())
        .filter(|skill| !skill.is_empty());
    let mut active_turn_guard = ActiveTurnGuard::new(session_id.clone());

    let (base_url, api_key, model) = resolve_turn_provider_config(&input)?;
    let provider = input.provider.as_deref().unwrap_or(DEFAULT_PROVIDER);

    let loop_config = build_loop_config(&input);

    let persistence = SessionPersistenceSink::new(input.project_path.clone(), session_id.clone())
        .with_client_request_id(Some(client_request_id.clone()))
        .with_hydration_source(Some(hydration_status.clone()));

    let emitter = AgentEventEmitter::new(app_handle.clone(), session_id.clone(), turn_id)
        .with_client_request_id(Some(client_request_id.clone()))
        .with_persistence(persistence);

    // Build conversation state (prefer saved history for multi-turn continuity)
    let mut state = prepared.conversation;

    let project_path = std::path::Path::new(&input.project_path);
    let pending_todo_count = extract_pending_todo_count(&state);
    let current_canon_revision = crate::gate_integration::read_canon_version(project_path)
        .ok()
        .flatten()
        .map(|cv| cv.revision);
    let current_branch_id =
        crate::agent_engine::reminder_builder::read_active_branch_id(project_path);
    let (session_canon_revision, session_branch_id) = resolve_session_baseline(
        project_path,
        &session_id,
        current_canon_revision,
        &current_branch_id,
    );

    // DevE: build dynamic reminder for Layer D injection
    let reminder = {
        use crate::agent_engine::prompt_assembler::mode_layer::PromptMode;
        use crate::agent_engine::reminder_builder::{build_reminder, ReminderInput, SessionKind};
        let mode = PromptMode::from_engine_modes(
            loop_config.capability_mode,
            loop_config.clarification_mode,
        );
        let session_kind = if state.messages.is_empty() {
            SessionKind::New
        } else {
            SessionKind::Resume
        };
        let mut reminder_input = ReminderInput::new(project_path, mode, session_kind);

        reminder_input.active_chapter_path = input
            .active_chapter_path
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        reminder_input.pending_todo_count = pending_todo_count;
        reminder_input.session_canon_revision = session_canon_revision;
        reminder_input.session_branch_id = session_branch_id.as_deref();

        // DevC: Provide pending-blocker signal (conservative: any mission pending review decision).
        reminder_input.has_pending_blocker =
            crate::gate_integration::has_pending_review_blocker(project_path);

        // DevC: Provide Active Rules summary (writing_rules EffectiveRules → compact text).
        let active_rules_summary = reminder_input.active_chapter_path.and_then(|p| {
            crate::writing_rules::resolve_effective_rules_if_available(project_path, p)
                .map(|rules| crate::gate_integration::render_active_rules_summary(&rules))
                .filter(|summary| !summary.trim().is_empty())
        });
        if let Some(ref summary) = active_rules_summary {
            reminder_input.active_rules_summary = Some(summary.as_str());
        }

        let r = build_reminder(&reminder_input);
        if r.is_empty() {
            None
        } else {
            Some(r)
        }
    };

    let prompt_role = input.prompt_role.as_deref().and_then(parse_prompt_role);

    apply_system_prompt_with_config(
        &mut state,
        input.system_prompt.as_deref(),
        &loop_config,
        Some(provider),
        Some(&model),
        prompt_role,
        reminder,
    );
    state.session_id = session_id.clone();

    // Add user message for this turn
    let user_message = AgentMessage::user(input.user_text.clone());
    state.messages.push(user_message.clone());
    state.current_turn = turn_id;

    emitter.persist_user_message(&user_message, turn_id)?;

    // Emit TURN_STARTED immediately
    let semantic_retrieval_enabled = load_openai_search_settings()
        .map(|settings| settings.openai_embedding_enabled)
        .unwrap_or(false);
    let turn_start_tool_meta = resolve_turn_tool_exposure(
        &state,
        &loop_config,
        input.active_chapter_path.as_deref(),
        semantic_retrieval_enabled,
    )
    .telemetry
    .to_payload();
    emitter.turn_started_with_meta(
        &input.user_text,
        provider,
        &model,
        Some(turn_start_tool_meta),
    )?;
    session_state::global().save_runtime_state(
        &session_id,
        state.clone(),
        turn_id.saturating_add(1),
        None,
    );

    // Build turn engine based on provider
    // Use StreamingTurnEngine for real-time delta events, fall back to direct for compatibility
    let use_streaming = input.provider.as_deref() != Some("direct");
    let snapshot_ctx = SnapshotContext {
        project_path: input.project_path.clone(),
        session_id: session_id.clone(),
        provider_name: provider.to_string(),
        model: model.clone(),
        base_url: base_url.clone(),
        system_prompt: input.system_prompt.clone(),
        active_chapter_path: input.active_chapter_path.clone(),
        active_skill: active_skill.clone(),
        loop_config: loop_config.clone(),
        session_canon_revision,
        session_branch_id: session_branch_id.clone(),
    };

    if use_streaming {
        // Spawn the agent loop in a background task
        let project_path = input.project_path.clone();
        let active_chapter_path = input.active_chapter_path.clone();
        let cancel = cancel_token.clone();
        let sid = session_id.clone();
        let prov = provider.to_string();
        let mdl = model.clone();
        let burl = base_url.clone();
        let akey = api_key.clone();
        let snapshot_ctx = snapshot_ctx.clone();
        tokio::spawn(async move {
            // Build streaming engine via LLM router
            let router = build_router(&prov, burl.clone(), akey.clone(), RetryConfig::default());
            let streaming_engine =
                StreamingTurnEngine::new(router, emitter.clone(), prov.clone(), mdl.clone())
                    .with_cancel_token(cancel.clone());

            let agent_loop =
                AgentLoop::new(emitter.clone(), loop_config.clone(), project_path, cancel)
                    .with_provider_info(prov, mdl, burl, akey)
                    .with_active_chapter_path(active_chapter_path)
                    .with_active_skill(active_skill.clone())
                    .with_editor_state(input.editor_state.clone());

            match agent_loop.run(&mut state, &streaming_engine).await {
                Ok(result) => {
                    persist_runtime_after_loop(
                        &snapshot_ctx,
                        &sid,
                        &state,
                        &result.stop_reason,
                        result.active_skill.clone(),
                    );
                    tracing::info!(
                        target: "agent_engine",
                        session_id = %sid,
                        engine = "streaming",
                        stop_reason = ?result.stop_reason,
                        rounds = result.rounds_executed,
                        tool_calls = result.total_tool_calls,
                        latency_ms = result.latency_ms,
                        "agent loop completed"
                    );
                }
                Err(e) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot_for_conversation(
                        &snapshot_ctx,
                        &state,
                        SessionRuntimeState::Failed,
                        snapshot_ctx.active_skill.clone(),
                    );
                    tracing::error!(
                        target: "agent_engine",
                        session_id = %sid,
                        error = %e,
                        "agent loop failed"
                    );
                }
            }
            session_state::global().clear_cancel_token(&sid);
        });
    } else {
        // Fallback: direct (non-streaming) engine
        let turn_engine = OpenAiDirectTurnEngine {
            base_url: base_url.clone(),
            api_key: api_key.clone(),
            model: model.clone(),
        };

        let project_path = input.project_path.clone();
        let active_chapter_path = input.active_chapter_path.clone();
        let cancel = cancel_token.clone();
        let sid = session_id.clone();
        let prov = provider.to_string();
        let mdl = model;
        let burl = base_url;
        let akey = api_key;
        let snapshot_ctx = snapshot_ctx.clone();
        tokio::spawn(async move {
            let agent_loop = AgentLoop::new(emitter.clone(), loop_config, project_path, cancel)
                .with_provider_info(prov, mdl, burl, akey)
                .with_active_chapter_path(active_chapter_path)
                .with_active_skill(active_skill.clone())
                .with_editor_state(input.editor_state);

            match agent_loop.run(&mut state, &turn_engine).await {
                Ok(result) => {
                    persist_runtime_after_loop(
                        &snapshot_ctx,
                        &sid,
                        &state,
                        &result.stop_reason,
                        result.active_skill.clone(),
                    );
                    tracing::info!(
                        target: "agent_engine",
                        session_id = %sid,
                        engine = "direct",
                        stop_reason = ?result.stop_reason,
                        rounds = result.rounds_executed,
                        tool_calls = result.total_tool_calls,
                        latency_ms = result.latency_ms,
                        "agent loop completed"
                    );
                }
                Err(e) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot_for_conversation(
                        &snapshot_ctx,
                        &state,
                        SessionRuntimeState::Failed,
                        snapshot_ctx.active_skill.clone(),
                    );
                    tracing::error!(
                        target: "agent_engine",
                        session_id = %sid,
                        error = %e,
                        "agent loop failed"
                    );
                }
            }
            session_state::global().clear_cancel_token(&sid);
        });
    }

    active_turn_guard.disarm();

    Ok(AgentTurnStartResult {
        session_id: input.session_id,
        turn_id,
        client_request_id,
        hydration_status: Some(hydration_status),
        session_revision: None,
    })
}

/// Cancel a running agent turn.
#[command]
pub async fn agent_turn_cancel(
    app_handle: tauri::AppHandle,
    input: AgentTurnCancelInput,
) -> Result<(), AppError> {
    // Signal the running loop to stop via cancellation token
    let cancelled = session_state::global().cancel_session(&input.session_id);
    let suspended = session_state::global().take_suspended(&input.session_id);
    let suspended_project_path = suspended.as_ref().map(|state| state.project_path.clone());
    let had_suspended = suspended.is_some();

    if let Some(ref suspended) = suspended {
        let mut cancelled_suspended = suspended.clone();
        cancelled_suspended.suspend_reason = StopReason::Cancel;
        let snapshot_input = RuntimeSnapshotUpsertInput::from_suspended(
            input.session_id.clone(),
            cancelled_suspended,
            Some(suspended.conversation_state.current_turn),
        );

        if let Err(err) = save_runtime_snapshot_from_input(
            std::path::Path::new(&suspended.project_path),
            snapshot_input,
        ) {
            tracing::warn!(
                target: "agent_engine",
                session_id = %input.session_id,
                error = %err,
                "failed to persist cancelled suspended snapshot"
            );
        }
    }

    // If we cancelled an active running turn, the loop will emit TURN_CANCELLED.
    // Only emit here for suspended turns (no running loop to emit).
    if had_suspended {
        let turn_id = suspended
            .as_ref()
            .map(|state| state.conversation_state.current_turn)
            .filter(|value| *value > 0)
            .unwrap_or(input.turn_id);

        let mut emitter = AgentEventEmitter::new(app_handle, input.session_id.clone(), turn_id);

        if let Some(project_path) = suspended_project_path {
            let persistence = SessionPersistenceSink::new(project_path, input.session_id.clone())
                .with_hydration_source(Some("cancel_command".to_string()));
            emitter = emitter.with_persistence(persistence);
        }

        emitter.turn_cancelled()?;
    }

    tracing::info!(
        target: "agent_engine",
        session_id = %input.session_id,
        token_cancelled = cancelled.is_some(),
        had_suspended,
        "agent_turn_cancel executed"
    );

    Ok(())
}

/// Resume a suspended turn (after confirmation or askuser).
#[command]
pub async fn agent_turn_resume(
    app_handle: tauri::AppHandle,
    input: AgentTurnResumeInput,
) -> Result<(), AppError> {
    let prepared = prepare_resume_turn(&input.session_id)?;
    let authoritative_turn_id = prepared.turn_id;
    let cancel_token = prepared.cancel_token.clone();
    let suspended = prepared.suspended;
    let mut active_turn_guard = ActiveTurnGuard::new(input.session_id.clone());

    if input.turn_id != authoritative_turn_id {
        tracing::warn!(
            target: "agent_engine",
            session_id = %input.session_id,
            requested_turn_id = input.turn_id,
            authoritative_turn_id,
            "agent_turn_resume ignoring mismatched turn_id from caller"
        );
    }

    let persistence =
        SessionPersistenceSink::new(suspended.project_path.clone(), input.session_id.clone())
            .with_hydration_source(Some("resume".to_string()));
    let emitter = AgentEventEmitter::new(
        app_handle.clone(),
        input.session_id.clone(),
        authoritative_turn_id,
    )
    .with_persistence(persistence);

    let project_path = std::path::Path::new(&suspended.project_path);
    let current_canon_revision = crate::gate_integration::read_canon_version(project_path)
        .ok()
        .flatten()
        .map(|cv| cv.revision);
    let current_branch_id =
        crate::agent_engine::reminder_builder::read_active_branch_id(project_path);
    let (session_canon_revision, session_branch_id) = resolve_session_baseline(
        project_path,
        &input.session_id,
        current_canon_revision,
        &current_branch_id,
    );

    let resume_snapshot_ctx = SnapshotContext {
        project_path: suspended.project_path.clone(),
        session_id: input.session_id.clone(),
        provider_name: if suspended.provider_name.trim().is_empty() {
            DEFAULT_PROVIDER.to_string()
        } else {
            suspended.provider_name.clone()
        },
        model: suspended.model.clone(),
        base_url: suspended.base_url.clone(),
        system_prompt: suspended.system_prompt.clone(),
        active_chapter_path: suspended.active_chapter_path.clone(),
        active_skill: suspended.active_skill.clone(),
        loop_config: suspended.loop_config.clone(),
        session_canon_revision,
        session_branch_id,
    };

    // Build the tool result message based on resume input
    let (tool_result_message, resumed_tool_calls_executed) = match &input.resume_input {
        ResumeInput::Confirmation { allowed } => {
            if *allowed {
                // Execute the pending tool call
                let tc = &suspended.pending_tool_call;
                let call_id = &suspended.pending_call_id;

                emitter.tool_call_started(tc, call_id)?;

                let tc_clone = tc.clone();
                let call_id_clone = call_id.clone();
                let project_path = suspended.project_path.clone();
                let active_chapter_path = suspended.active_chapter_path.clone();
                let active_skill = suspended.active_skill.clone();
                let tool_name_err = tc.tool_name.clone();
                let call_id_err = call_id.clone();

                let result = tokio::task::spawn_blocking(move || {
                    crate::agent_engine::tool_dispatch::execute_tool_call(
                        &tc_clone,
                        &project_path,
                        &call_id_clone,
                        active_chapter_path.as_deref(),
                        active_skill.as_deref(),
                    )
                })
                .await
                .unwrap_or_else(|e| {
                    use crate::agent_tools::contracts::{
                        FaultDomain, ToolError, ToolMeta, ToolResult,
                    };
                    ToolResult {
                        ok: false,
                        data: None,
                        error: Some(ToolError {
                            code: "E_TOOL_TASK_JOIN_FAILED".to_string(),
                            message: e.to_string(),
                            retryable: true,
                            fault_domain: FaultDomain::Tool,
                            details: None,
                        }),
                        meta: ToolMeta {
                            tool: tool_name_err,
                            call_id: call_id_err,
                            duration_ms: 0,
                            revision_before: None,
                            revision_after: None,
                            tx_id: None,
                            read_set: None,
                            write_set: None,
                        },
                    }
                });

                let status = if result.ok { "ok" } else { "error" };
                let progress = if result.ok { "done" } else { "error" };
                emitter.tool_call_progress(tc, call_id, progress)?;
                let trace = Some(build_tool_trace(&tc.tool_name, &result));
                emitter.tool_call_finished(tc, call_id, status, trace)?;

                (build_tool_message(tc, &result), 1_u32)
            } else {
                // User denied — generate a denied tool result
                let tc = &suspended.pending_tool_call;
                let call_id = &suspended.pending_call_id;
                emitter.tool_call_progress(tc, call_id, "denied")?;
                emitter.tool_call_finished(
                    tc,
                    call_id,
                    "error",
                    Some(build_confirmation_denied_trace(&tc.tool_name, call_id)),
                )?;

                (
                    AgentMessage::tool_result(
                        suspended.pending_tool_call.llm_call_id.clone(),
                        Some(suspended.pending_tool_call.tool_name.clone()),
                        "Tool execution denied by user.".to_string(),
                        true,
                    ),
                    0_u32,
                )
            }
        }
        ResumeInput::AskUser { answers } => {
            let tc = &suspended.pending_tool_call;
            let call_id = &suspended.pending_call_id;
            emitter.askuser_answered(tc, call_id, answers)?;
            let content = serde_json::to_string(answers).unwrap_or_else(|_| "{}".to_string());
            (
                AgentMessage::tool_result(
                    suspended.pending_tool_call.llm_call_id.clone(),
                    Some(suspended.pending_tool_call.tool_name.clone()),
                    content,
                    false,
                ),
                0_u32,
            )
        }
    };

    // Restore conversation state and append the tool result
    let mut state = suspended.conversation_state;
    state.messages.push(tool_result_message);

    if should_persist_explicit_resumed_state(&input.resume_input) {
        if let Err(e) = emitter.persist_turn_state(
            authoritative_turn_id,
            "resumed",
            serde_json::json!({
                "pending_call_id": suspended.pending_call_id.clone(),
                "call_id": suspended.pending_call_id.clone(),
                "resume_mode": resume_mode_label(&input.resume_input),
            }),
        ) {
            tracing::warn!(
                target: "agent_engine",
                error = %e,
                resume_mode = resume_mode_label(&input.resume_input),
                "failed to persist resumed state"
            );
        }
    }

    // Also execute any remaining tool calls
    let mut resumed_total_tool_calls = suspended
        .total_tool_calls
        .saturating_add(resumed_tool_calls_executed);

    if !suspended.remaining_tool_calls.is_empty() {
        let scheduler = crate::agent_engine::tool_scheduler::ToolScheduler::new(
            emitter.clone(),
            suspended.project_path.clone(),
            suspended.loop_config.approval_mode,
            suspended.loop_config.clarification_mode,
            cancel_token.clone(),
        )
        .with_active_chapter_path(suspended.active_chapter_path.clone())
        .with_active_skill(suspended.active_skill.clone());

        let remaining_result = scheduler
            .execute_batch(suspended.remaining_tool_calls)
            .await?;

        resumed_total_tool_calls =
            resumed_total_tool_calls.saturating_add(remaining_result.executed_count);

        for msg in remaining_result.tool_messages {
            state.messages.push(msg);
        }

        // Remaining batch triggered a new suspension: persist and return immediately.
        if let Some(suspend_info) = remaining_result.suspend_reason {
            let re_suspended = session_state::SuspendedTurnState {
                conversation_state: state.clone(),
                pending_tool_call: suspend_info.pending_tool_call.clone(),
                pending_call_id: suspend_info.pending_call_id.clone(),
                remaining_tool_calls: suspend_info.remaining_tool_calls.clone(),
                completed_messages: suspend_info.completed_messages.clone(),
                loop_config: suspended.loop_config.clone(),
                project_path: suspended.project_path.clone(),
                provider_name: suspended.provider_name.clone(),
                model: suspended.model.clone(),
                base_url: suspended.base_url.clone(),
                api_key: suspended.api_key.clone(),
                active_chapter_path: suspended.active_chapter_path.clone(),
                active_skill: suspended.active_skill.clone(),
                system_prompt: suspended.system_prompt.clone(),
                suspend_reason: suspend_info.reason.clone(),
                rounds_executed: suspended.rounds_executed,
                total_tool_calls: resumed_total_tool_calls,
            };

            let snapshot_input = RuntimeSnapshotUpsertInput::from_suspended(
                input.session_id.clone(),
                re_suspended.clone(),
                Some(re_suspended.conversation_state.current_turn),
            );
            if let Err(err) = save_runtime_snapshot_from_input(
                std::path::Path::new(&suspended.project_path),
                snapshot_input,
            ) {
                tracing::warn!(
                    target: "agent_engine",
                    session_id = %input.session_id,
                    error = %err,
                    "failed to persist re-suspended snapshot"
                );
            }
            session_state::global().suspend_turn(&input.session_id, re_suspended);

            tracing::info!(
                target: "agent_engine",
                session_id = %input.session_id,
                turn_id = authoritative_turn_id,
                "agent_turn_resume: remaining tools triggered re-suspension"
            );
            return Ok(());
        }
    }

    // Continue the agent loop from where it left off
    let sid = input.session_id.clone();
    let resume_snapshot_ctx = resume_snapshot_ctx.clone();
    tokio::spawn(async move {
        let provider = if suspended.provider_name.trim().is_empty() {
            DEFAULT_PROVIDER.to_string()
        } else {
            suspended.provider_name.clone()
        };

        let agent_loop = AgentLoop::new(
            emitter.clone(),
            suspended.loop_config,
            suspended.project_path,
            cancel_token.clone(),
        )
        .with_provider_info(
            provider.clone(),
            suspended.model.clone(),
            suspended.base_url.clone(),
            suspended.api_key.clone(),
        )
        .with_active_chapter_path(suspended.active_chapter_path.clone())
        .with_active_skill(suspended.active_skill.clone());

        let use_streaming = provider != "direct";
        let run_result = if use_streaming {
            let router = build_router(
                &provider,
                suspended.base_url.clone(),
                suspended.api_key.clone(),
                RetryConfig::default(),
            );
            let streaming_engine = StreamingTurnEngine::new(
                router,
                emitter.clone(),
                provider.clone(),
                suspended.model.clone(),
            )
            .with_cancel_token(cancel_token.clone());

            agent_loop.run(&mut state, &streaming_engine).await
        } else {
            let direct_engine = OpenAiDirectTurnEngine {
                base_url: suspended.base_url.clone(),
                api_key: suspended.api_key.clone(),
                model: suspended.model.clone(),
            };

            agent_loop.run(&mut state, &direct_engine).await
        };

        match run_result {
            Ok(result) => {
                persist_runtime_after_loop(
                    &resume_snapshot_ctx,
                    &sid,
                    &state,
                    &result.stop_reason,
                    result.active_skill.clone(),
                );
                tracing::info!(
                    target: "agent_engine",
                    session_id = %sid,
                    engine = if use_streaming { "resume_streaming" } else { "resume_direct" },
                    stop_reason = ?result.stop_reason,
                    rounds = result.rounds_executed,
                    tool_calls = result.total_tool_calls,
                    latency_ms = result.latency_ms,
                    "resumed agent loop completed"
                );
            }
            Err(e) => {
                session_state::global().save_conversation(&sid, state.clone());
                persist_runtime_snapshot_for_conversation(
                    &resume_snapshot_ctx,
                    &state,
                    SessionRuntimeState::Failed,
                    resume_snapshot_ctx.active_skill.clone(),
                );
                tracing::error!(
                    target: "agent_engine",
                    session_id = %sid,
                    error = %e,
                    "resumed agent loop failed"
                );
            }
        }
        session_state::global().clear_cancel_token(&sid);
    });

    active_turn_guard.disarm();

    tracing::info!(
        target: "agent_engine",
        session_id = %input.session_id,
        turn_id = authoritative_turn_id,
        "agent_turn_resume: loop restarted"
    );

    Ok(())
}

#[cfg(test)]
mod tests;
