use crate::agent_engine::exposure_policy::{
    CapabilityPolicy, CapabilityPreset, ExposureContext, SessionSource,
};
use serde::{Deserialize, Serialize};
use tauri::command;
use tokio_util::sync::CancellationToken;

use crate::agent_engine::emitter::EventSink;
use crate::agent_engine::loop_engine::AgentLoop;
use crate::agent_engine::messages::AgentMessage;
use crate::agent_engine::session_state;
use crate::agent_engine::tool_routing::resolve_turn_tool_exposure_with_context;
use crate::agent_engine::turn::OpenAiDirectTurnEngine;
use crate::agent_engine::types::{
    AgentMode, AgentTurnStartResult, ApprovalMode, ClarificationMode,
};
use crate::application::command_usecases::inspiration::{
    build_create_handoff, create_inspiration_session, ensure_inspiration_session_exists,
    generate_metadata_variants, load_inspiration_session_snapshot, save_inspiration_session_state,
    CreateProjectHandoffDraft, GenerateMetadataVariantsInput, InspirationConsensusState,
    LoadedInspirationSessionSnapshot, MetadataVariant, OpenQuestion,
};
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::streaming_turn::StreamingTurnEngine;
use crate::models::{AppError, ErrorCode};
use crate::services::agent_session::SessionRuntimeState;
use crate::services::inspiration_session::{
    inspiration_root, list_session_meta, load_runtime_snapshot, remove_session_meta,
    save_runtime_snapshot_from_input, session_runtime_path, session_stream_path,
    update_session_meta_title, InspirationRuntimeSnapshotUpsertInput, InspirationSessionMeta,
    InspirationSessionPersistenceSink,
};
use crate::services::load_openai_search_settings;

use super::common::{build_loop_config, resolve_turn_provider_config};
use super::emitter::InspirationEventEmitter;
use super::prompt::apply_inspiration_system_prompt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionCreateInput {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionCreateOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionLoadInput {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionSnapshot {
    pub meta: InspirationSessionMeta,
    pub messages: Vec<crate::agent_engine::messages::AgentMessage>,
    pub consensus: InspirationConsensusState,
    pub open_questions: Vec<OpenQuestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
    pub runtime_state: String,
    pub hydration_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_turn_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionLoadOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub snapshot: InspirationSessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionSaveStateInput {
    pub session_id: String,
    pub consensus: InspirationConsensusState,
    #[serde(default)]
    pub open_questions: Vec<OpenQuestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionSaveStateOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub snapshot: InspirationSessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InspirationSessionListInput {
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionUpdateMetaInput {
    pub session_id: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionDeleteInput {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationTurnStartInput {
    pub session_id: String,
    #[serde(default)]
    pub client_request_id: Option<String>,
    pub user_text: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub capability_mode: Option<AgentMode>,
    #[serde(default)]
    pub approval_mode: Option<ApprovalMode>,
    #[serde(default)]
    pub clarification_mode: Option<ClarificationMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationTurnCancelInput {
    pub session_id: String,
    pub turn_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InspirationGenerateMetadataVariantsInput {
    pub consensus: InspirationConsensusState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationMetadataVariantCandidate {
    pub variant: MetadataVariant,
    pub create_handoff: CreateProjectHandoffDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationGenerateMetadataVariantsOutput {
    pub schema_version: i32,
    pub shared_story_core: String,
    pub variants: Vec<InspirationMetadataVariantCandidate>,
}

#[derive(Debug)]
struct PreparedStartTurn {
    conversation: crate::agent_engine::messages::ConversationState,
    hydration_status: String,
    turn_id: u32,
    cancel_token: CancellationToken,
}

#[derive(Clone)]
struct SnapshotContext {
    session_id: String,
    provider_name: String,
    model: String,
    base_url: String,
    system_prompt: Option<String>,
    loop_config: crate::agent_engine::types::LoopConfig,
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

fn inspiration_capability_policy(_clarification_mode: ClarificationMode) -> CapabilityPolicy {
    let mut policy = CapabilityPolicy::new(CapabilityPreset::MainPlanning);
    policy.allow_delegate = false;
    // The inspiration session profile is the hard boundary; this policy only
    // exposes the two inspiration patch tools inside that profile.
    policy.forced_tools = vec![
        "inspiration_consensus_patch".to_string(),
        "inspiration_open_questions_patch".to_string(),
    ];
    policy.normalized()
}

fn resolve_client_request_id(input: &InspirationTurnStartInput) -> String {
    input
        .client_request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("client_req_{}", uuid::Uuid::new_v4()))
}

fn runtime_state_label(state: &SessionRuntimeState) -> &'static str {
    match state {
        SessionRuntimeState::Ready => "ready",
        SessionRuntimeState::Running => "running",
        SessionRuntimeState::SuspendedConfirmation => "suspended_confirmation",
        SessionRuntimeState::SuspendedAskuser => "suspended_askuser",
        SessionRuntimeState::Completed => "completed",
        SessionRuntimeState::Failed => "failed",
        SessionRuntimeState::Cancelled => "cancelled",
        SessionRuntimeState::Degraded => "degraded",
    }
}

fn stop_reason_to_runtime_state(
    reason: &crate::agent_engine::types::StopReason,
) -> SessionRuntimeState {
    match reason {
        crate::agent_engine::types::StopReason::Success => SessionRuntimeState::Completed,
        crate::agent_engine::types::StopReason::Cancel => SessionRuntimeState::Cancelled,
        crate::agent_engine::types::StopReason::Error => SessionRuntimeState::Failed,
        crate::agent_engine::types::StopReason::Limit => SessionRuntimeState::Completed,
        crate::agent_engine::types::StopReason::WaitingConfirmation => {
            SessionRuntimeState::SuspendedConfirmation
        }
        crate::agent_engine::types::StopReason::WaitingAskuser => {
            SessionRuntimeState::SuspendedAskuser
        }
    }
}

fn persist_runtime_snapshot(
    ctx: &SnapshotContext,
    state: crate::agent_engine::messages::ConversationState,
    runtime_state: SessionRuntimeState,
    hydration_source: Option<String>,
) {
    let existing_domain_state =
        load_runtime_snapshot(&ctx.session_id)
            .ok()
            .flatten()
            .map(|snapshot| {
                (
                    snapshot.consensus,
                    snapshot.open_questions,
                    snapshot.final_create_handoff_draft,
                )
            });
    let current_turn = state.current_turn;
    let mut input = InspirationRuntimeSnapshotUpsertInput::from_conversation(
        ctx.session_id.clone(),
        runtime_state,
        state,
        Some(current_turn),
        Some(ctx.provider_name.clone()),
        Some(ctx.model.clone()),
        Some(ctx.base_url.clone()),
        ctx.system_prompt.clone(),
        Some(ctx.loop_config.clone()),
    )
    .with_hydration_source(hydration_source);

    if let Some((consensus, open_questions, final_create_handoff_draft)) = existing_domain_state {
        input.consensus = consensus;
        input.open_questions = open_questions;
        input.final_create_handoff_draft = final_create_handoff_draft;
    }

    if let Err(err) = save_runtime_snapshot_from_input(input) {
        tracing::warn!(
            target: "inspiration",
            session_id = %ctx.session_id,
            error = %err,
            "failed to persist inspiration runtime snapshot"
        );
    }
}

fn build_session_snapshot(loaded: LoadedInspirationSessionSnapshot) -> InspirationSessionSnapshot {
    let LoadedInspirationSessionSnapshot {
        meta,
        events,
        conversation,
        consensus,
        open_questions,
        final_create_handoff_draft,
        runtime_state,
        hydration_status,
        last_turn,
        next_turn_id,
    } = loaded;
    let _event_count = events.len();

    InspirationSessionSnapshot {
        meta,
        messages: conversation.messages,
        consensus,
        open_questions,
        final_create_handoff_draft,
        runtime_state: runtime_state_label(&runtime_state).to_string(),
        hydration_status,
        last_turn,
        next_turn_id,
    }
}

fn active_turn_delete_conflict(session_id: &str) -> AppError {
    AppError {
        code: ErrorCode::Conflict,
        message: "cannot delete inspiration session while a turn is active".to_string(),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_DELETE_CONFLICT_ACTIVE_TURN",
            "session_id": session_id,
        })),
        recoverable: Some(true),
    }
}

fn remove_file_if_exists(path: &std::path::Path) -> Result<(), AppError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError {
            code: ErrorCode::IoError,
            message: format!("failed to delete inspiration session file: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_DELETE_FILE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        }),
    }
}

fn prepare_turn_start(
    session_id: &str,
    client_request_id: &str,
) -> Result<PreparedStartTurn, AppError> {
    ensure_inspiration_session_exists(session_id)?;

    session_state::global().with_session_turn_lock(session_id, || {
        if session_state::global().has_active_turn(session_id) {
            return Err(AppError::invalid_argument(
                "A turn is already running for this inspiration session.",
            ));
        }

        let snapshot = load_inspiration_session_snapshot(session_id)?;
        let next_turn_id = snapshot.next_turn_id.unwrap_or_else(|| {
            crate::agent_engine::session_state::derive_next_turn_id(
                snapshot.last_turn,
                Some(&snapshot.conversation),
                None,
            )
        });
        session_state::global().seed_next_turn_id(session_id, next_turn_id);

        let turn_id = session_state::global().next_turn_id(session_id);
        let cancel_token = CancellationToken::new();
        session_state::global().set_cancel_token(
            session_id,
            turn_id,
            cancel_token.clone(),
            Some(client_request_id.to_string()),
        );

        Ok(PreparedStartTurn {
            conversation: snapshot.conversation,
            hydration_status: snapshot.hydration_status,
            turn_id,
            cancel_token,
        })
    })
}

fn prepare_turn_start_guarded(
    session_id: &str,
    client_request_id: &str,
) -> Result<(PreparedStartTurn, ActiveTurnGuard), AppError> {
    let prepared = prepare_turn_start(session_id, client_request_id)?;
    let guard = ActiveTurnGuard::new(session_id.to_string());
    Ok((prepared, guard))
}

#[command]
pub async fn inspiration_session_create(
    input: InspirationSessionCreateInput,
) -> Result<InspirationSessionCreateOutput, AppError> {
    let (session_id, created_at) = create_inspiration_session(input.title)?;
    Ok(InspirationSessionCreateOutput {
        schema_version: 1,
        session_id,
        created_at,
    })
}

#[command]
pub async fn inspiration_session_load(
    input: InspirationSessionLoadInput,
) -> Result<InspirationSessionLoadOutput, AppError> {
    let loaded = load_inspiration_session_snapshot(&input.session_id)?;
    Ok(InspirationSessionLoadOutput {
        schema_version: 1,
        session_id: input.session_id,
        snapshot: build_session_snapshot(loaded),
    })
}

#[command]
pub async fn inspiration_session_save_state(
    input: InspirationSessionSaveStateInput,
) -> Result<InspirationSessionSaveStateOutput, AppError> {
    let loaded = save_inspiration_session_state(
        &input.session_id,
        input.consensus,
        input.open_questions,
        input.final_create_handoff_draft,
    )?;

    Ok(InspirationSessionSaveStateOutput {
        schema_version: 1,
        session_id: input.session_id,
        snapshot: build_session_snapshot(loaded),
    })
}

#[command]
pub async fn inspiration_session_list(
    input: InspirationSessionListInput,
) -> Result<Vec<InspirationSessionMeta>, AppError> {
    list_session_meta(input.limit)
}

#[command]
pub async fn inspiration_session_update_meta(
    input: InspirationSessionUpdateMetaInput,
) -> Result<InspirationSessionMeta, AppError> {
    update_session_meta_title(&input.session_id, input.title)
}

#[command]
pub async fn inspiration_session_delete(
    input: InspirationSessionDeleteInput,
) -> Result<(), AppError> {
    if session_state::global().has_active_turn(&input.session_id) {
        return Err(active_turn_delete_conflict(&input.session_id));
    }

    remove_session_meta(&input.session_id)?;
    let stream_path = session_stream_path(&input.session_id)?;
    let runtime_path = session_runtime_path(&input.session_id)?;
    remove_file_if_exists(&stream_path)?;
    remove_file_if_exists(&runtime_path)?;
    session_state::global().remove_session(&input.session_id);

    Ok(())
}

#[command]
pub async fn inspiration_turn_start(
    app_handle: tauri::AppHandle,
    input: InspirationTurnStartInput,
) -> Result<AgentTurnStartResult, AppError> {
    let client_request_id = resolve_client_request_id(&input);
    let (prepared, mut active_turn_guard) =
        prepare_turn_start_guarded(&input.session_id, &client_request_id)?;
    let (provider, base_url, api_key, model) = resolve_turn_provider_config(&input)?;
    let loop_config = build_loop_config(&input);
    let capability_policy = inspiration_capability_policy(loop_config.clarification_mode);
    let mut state = prepared.conversation;
    let turn_id = prepared.turn_id;
    let cancel_token = prepared.cancel_token.clone();

    apply_inspiration_system_prompt(&mut state, input.system_prompt.as_deref());
    state.session_id = input.session_id.clone();
    state
        .messages
        .push(AgentMessage::user(input.user_text.clone()));
    state.current_turn = turn_id;

    let persistence = InspirationSessionPersistenceSink::new(input.session_id.clone())
        .with_client_request_id(Some(client_request_id.clone()))
        .with_hydration_source(Some(prepared.hydration_status.clone()));
    let emitter =
        InspirationEventEmitter::new(app_handle.clone(), input.session_id.clone(), turn_id)
            .with_client_request_id(Some(client_request_id.clone()))
            .with_persistence(persistence);

    emitter.persist_user_message(&input.user_text, turn_id)?;

    let semantic_retrieval_enabled = load_openai_search_settings()
        .map(|settings| settings.openai_embedding_enabled)
        .unwrap_or(false);
    let resolved_tool_exposure = resolve_turn_tool_exposure_with_context(
        &loop_config,
        None,
        ExposureContext::new(
            loop_config.capability_mode,
            loop_config.approval_mode,
            loop_config.clarification_mode,
            SessionSource::UserInteractive,
            0,
            semantic_retrieval_enabled,
            None,
            Some("inspiration_session".to_string()),
            capability_policy.clone(),
        ),
    );
    emitter.turn_started_with_meta(
        &input.user_text,
        &provider,
        &model,
        Some(resolved_tool_exposure.telemetry.to_payload()),
    )?;

    session_state::global().save_runtime_state(
        &input.session_id,
        state.clone(),
        turn_id.saturating_add(1),
        None,
    );

    let snapshot_ctx = SnapshotContext {
        session_id: input.session_id.clone(),
        provider_name: provider.clone(),
        model: model.clone(),
        base_url: base_url.clone(),
        system_prompt: input.system_prompt.clone(),
        loop_config: loop_config.clone(),
    };
    persist_runtime_snapshot(
        &snapshot_ctx,
        state.clone(),
        SessionRuntimeState::Running,
        Some(prepared.hydration_status.clone()),
    );

    let runtime_root = inspiration_root()?.to_string_lossy().to_string();
    let use_streaming = provider != "direct";

    if use_streaming {
        let sid = input.session_id.clone();
        let emitter_clone = emitter.clone();
        let snapshot_ctx = snapshot_ctx.clone();
        let loop_config = loop_config.clone();
        let runtime_root = runtime_root.clone();
        let capability_policy = capability_policy.clone();

        tokio::spawn(async move {
            let router = build_router(
                &provider,
                base_url.clone(),
                api_key.clone(),
                RetryConfig::interactive(),
            );
            let turn_engine = StreamingTurnEngine::new(
                router,
                emitter_clone.clone(),
                provider.clone(),
                model.clone(),
            )
            .with_cancel_token(cancel_token.clone());
            let agent_loop = AgentLoop::new(
                emitter_clone.clone(),
                loop_config.clone(),
                runtime_root,
                cancel_token.clone(),
            )
            .with_provider_info(
                provider.clone(),
                model.clone(),
                base_url.clone(),
                api_key.clone(),
            )
            .with_capability_policy(capability_policy);

            match agent_loop.run(&mut state, &turn_engine).await {
                Ok(result) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot(
                        &snapshot_ctx,
                        state.clone(),
                        stop_reason_to_runtime_state(&result.stop_reason),
                        Some("loop_finished".to_string()),
                    );
                }
                Err(err) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot(
                        &snapshot_ctx,
                        state.clone(),
                        SessionRuntimeState::Failed,
                        Some("loop_failed".to_string()),
                    );
                    tracing::error!(
                        target: "inspiration",
                        session_id = %sid,
                        error = %err,
                        "inspiration turn failed"
                    );
                }
            }
            session_state::global().clear_cancel_token(&sid);
        });
    } else {
        let sid = input.session_id.clone();
        let emitter_clone = emitter.clone();
        let snapshot_ctx = snapshot_ctx.clone();
        let loop_config = loop_config.clone();
        let runtime_root = runtime_root.clone();
        let capability_policy = capability_policy.clone();

        tokio::spawn(async move {
            let turn_engine = OpenAiDirectTurnEngine {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: model.clone(),
            };
            let agent_loop = AgentLoop::new(
                emitter_clone.clone(),
                loop_config.clone(),
                runtime_root,
                cancel_token.clone(),
            )
            .with_provider_info(
                provider.clone(),
                model.clone(),
                base_url.clone(),
                api_key.clone(),
            )
            .with_capability_policy(capability_policy);

            match agent_loop.run(&mut state, &turn_engine).await {
                Ok(result) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot(
                        &snapshot_ctx,
                        state.clone(),
                        stop_reason_to_runtime_state(&result.stop_reason),
                        Some("loop_finished".to_string()),
                    );
                }
                Err(err) => {
                    session_state::global().save_conversation(&sid, state.clone());
                    persist_runtime_snapshot(
                        &snapshot_ctx,
                        state.clone(),
                        SessionRuntimeState::Failed,
                        Some("loop_failed".to_string()),
                    );
                    tracing::error!(
                        target: "inspiration",
                        session_id = %sid,
                        error = %err,
                        "inspiration turn failed"
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
        hydration_status: Some(prepared.hydration_status),
        session_revision: None,
    })
}

#[command]
pub async fn inspiration_turn_cancel(input: InspirationTurnCancelInput) -> Result<(), AppError> {
    ensure_inspiration_session_exists(&input.session_id)?;
    let cancelled = session_state::global().cancel_session(&input.session_id);

    tracing::info!(
        target: "inspiration",
        session_id = %input.session_id,
        turn_id = input.turn_id,
        token_cancelled = cancelled.is_some(),
        "inspiration_turn_cancel executed"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        inspiration_generate_metadata_variants, inspiration_session_delete,
        inspiration_session_list, inspiration_session_update_meta, inspiration_turn_cancel,
        prepare_turn_start, prepare_turn_start_guarded, InspirationGenerateMetadataVariantsInput,
        InspirationSessionDeleteInput, InspirationSessionListInput,
        InspirationSessionUpdateMetaInput, InspirationTurnCancelInput,
    };
    use crate::agent_engine::session_state;
    use crate::application::command_usecases::inspiration::{
        create_inspiration_session, ConsensusValue, InspirationConsensusState, MetadataVariantId,
    };
    use crate::models::AppError;
    use crate::services::inspiration_session::{
        load_session_meta, save_session_meta, session_runtime_path, session_stream_path,
        InspirationSessionMeta,
    };
    use crate::test_support::inspiration_env::{enter_temp_root, with_temp_root};

    #[test]
    fn prepare_turn_start_allocates_first_turn_without_project_path() {
        with_temp_root(|| {
            let (session_id, _) = create_inspiration_session(None).expect("create session");

            let prepared =
                prepare_turn_start(&session_id, "req_prepare").expect("prepare start turn");

            assert_eq!(prepared.turn_id, 1);
            assert_eq!(prepared.hydration_status, "snapshot_loaded");
            assert!(prepared.conversation.messages.is_empty());
            assert!(session_state::global().has_active_turn(&session_id));

            session_state::global().clear_cancel_token(&session_id);
            session_state::global().remove_session(&session_id);
        });
    }

    #[tokio::test]
    async fn inspiration_turn_cancel_cancels_prepared_turn() {
        let _temp_root = enter_temp_root();

        let (session_id, _) = create_inspiration_session(None).expect("create session");
        let prepared = prepare_turn_start(&session_id, "req_cancel").expect("prepare start turn");

        assert!(!prepared.cancel_token.is_cancelled());
        inspiration_turn_cancel(InspirationTurnCancelInput {
            session_id: session_id.clone(),
            turn_id: prepared.turn_id,
        })
        .await
        .expect("cancel turn");

        assert!(prepared.cancel_token.is_cancelled());
        assert!(!session_state::global().has_active_turn(&session_id));
        session_state::global().remove_session(&session_id);
    }

    #[test]
    fn prepare_turn_start_guarded_releases_active_turn_on_early_error() {
        with_temp_root(|| {
            let (session_id, _) = create_inspiration_session(None).expect("create session");
            assert!(!session_state::global().has_active_turn(&session_id));

            let result = (|| -> Result<(), AppError> {
                let (_prepared, _guard) =
                    prepare_turn_start_guarded(&session_id, "req_guarded_error")?;
                Err(AppError::invalid_argument("forced early error"))
            })();

            assert!(result.is_err());
            assert!(
                !session_state::global().has_active_turn(&session_id),
                "active turn lock should be released on early error",
            );

            session_state::global().remove_session(&session_id);
        });
    }

    #[tokio::test]
    async fn list_returns_sorted_and_limited() {
        let _temp_root = enter_temp_root();

        let mut low = InspirationSessionMeta::new("insp_low".to_string(), 10, None);
        low.updated_at = 100;
        let mut mid = InspirationSessionMeta::new("insp_mid".to_string(), 20, None);
        mid.updated_at = 300;
        let mut top = InspirationSessionMeta::new("insp_top".to_string(), 30, None);
        top.updated_at = 300;

        save_session_meta(low).expect("save low");
        save_session_meta(mid).expect("save mid");
        save_session_meta(top).expect("save top");

        let listed = inspiration_session_list(InspirationSessionListInput { limit: Some(2) })
            .await
            .expect("list sessions");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].session_id, "insp_top");
        assert_eq!(listed[1].session_id, "insp_mid");

        let empty = inspiration_session_list(InspirationSessionListInput { limit: Some(0) })
            .await
            .expect("list with zero limit");
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn update_meta_updates_title_and_updated_at() {
        let _temp_root = enter_temp_root();

        let mut meta = InspirationSessionMeta::new(
            "insp_update_meta".to_string(),
            7,
            Some("旧标题".to_string()),
        );
        meta.updated_at = 8;
        save_session_meta(meta).expect("save baseline meta");

        let updated = inspiration_session_update_meta(InspirationSessionUpdateMetaInput {
            session_id: "insp_update_meta".to_string(),
            title: Some("  新标题  ".to_string()),
        })
        .await
        .expect("update title");
        assert_eq!(updated.title.as_deref(), Some("新标题"));
        assert!(updated.updated_at > 8);

        let cleared = inspiration_session_update_meta(InspirationSessionUpdateMetaInput {
            session_id: "insp_update_meta".to_string(),
            title: Some("   ".to_string()),
        })
        .await
        .expect("clear title");
        assert!(cleared.title.is_none());
    }

    #[tokio::test]
    async fn delete_removes_index_and_files() {
        let _temp_root = enter_temp_root();

        let (session_id, _) = create_inspiration_session(Some("待删除".to_string()))
            .expect("create inspiration session");
        let stream_path = session_stream_path(&session_id).expect("stream path");
        let runtime_path = session_runtime_path(&session_id).expect("runtime path");
        assert!(
            stream_path.exists(),
            "stream file should exist before delete"
        );
        assert!(
            runtime_path.exists(),
            "runtime file should exist before delete"
        );

        inspiration_session_delete(InspirationSessionDeleteInput {
            session_id: session_id.clone(),
        })
        .await
        .expect("delete inspiration session");

        let loaded = load_session_meta(&session_id).expect("load deleted meta");
        assert!(loaded.is_none(), "session meta should be removed");
        assert!(!stream_path.exists(), "stream file should be removed");
        assert!(!runtime_path.exists(), "runtime file should be removed");
    }

    #[tokio::test]
    async fn delete_conflicts_when_turn_is_active() {
        let _temp_root = enter_temp_root();
        let (session_id, _) = create_inspiration_session(None).expect("create session");
        let prepared = prepare_turn_start(&session_id, "req_delete_conflict")
            .expect("prepare active turn for conflict");
        assert!(session_state::global().has_active_turn(&session_id));

        let err = inspiration_session_delete(InspirationSessionDeleteInput {
            session_id: session_id.clone(),
        })
        .await
        .expect_err("delete should conflict while turn is active");

        assert!(matches!(err.code, crate::models::ErrorCode::Conflict));
        let detail_code = err
            .details
            .as_ref()
            .and_then(|value| value.get("code"))
            .and_then(|value| value.as_str());
        assert_eq!(
            detail_code,
            Some("E_INSPIRATION_SESSION_DELETE_CONFLICT_ACTIVE_TURN")
        );

        session_state::global().clear_cancel_token(&session_id);
        session_state::global().remove_session(&session_id);
        assert!(!prepared.cancel_token.is_cancelled());
    }

    fn sample_consensus() -> InspirationConsensusState {
        let mut consensus = InspirationConsensusState::default();
        consensus.story_core.draft_value = Some(ConsensusValue::Text(
            "一个被放逐的抄经师必须在禁书与秩序之间做选择".to_string(),
        ));
        consensus.premise.draft_value = Some(ConsensusValue::Text(
            "被放逐的抄经师发现禁书能改写现实，却会吞噬记忆。".to_string(),
        ));
        consensus.genre_tone.draft_value = Some(ConsensusValue::List(vec![
            "奇幻".to_string(),
            "悬疑".to_string(),
            "克制".to_string(),
        ]));
        consensus.protagonist.draft_value = Some(ConsensusValue::Text(
            "主角是谨慎、自律、擅长辨伪的抄经师，但内心一直想证明自己不是废人。".to_string(),
        ));
        consensus.worldview.draft_value = Some(ConsensusValue::Text(
            "帝国以抄写院垄断知识，禁书会以代价换来现实改写。".to_string(),
        ));
        consensus.core_conflict.draft_value = Some(ConsensusValue::Text(
            "主角必须决定是交出禁书保全秩序，还是借禁书撕开真相。".to_string(),
        ));
        consensus.selling_points.draft_value = Some(ConsensusValue::List(vec![
            "禁书机制".to_string(),
            "记忆代价".to_string(),
        ]));
        consensus.audience.draft_value = Some(ConsensusValue::Text(
            "喜欢设定驱动悬疑奇幻的读者".to_string(),
        ));
        consensus.ending_direction.draft_value = Some(ConsensusValue::Text(
            "结局必须让主角为真相付出记忆代价，但保住最重要的底线。".to_string(),
        ));
        consensus
    }

    #[tokio::test]
    async fn generate_metadata_variants_command_returns_handoffs() {
        let output =
            inspiration_generate_metadata_variants(InspirationGenerateMetadataVariantsInput {
                consensus: sample_consensus(),
            })
            .await
            .expect("generate metadata variants");

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.variants.len(), 3);
        assert_eq!(
            output.variants[0].variant.variant_id,
            MetadataVariantId::Balanced
        );
        assert_eq!(
            output.variants[1].variant.variant_id,
            MetadataVariantId::Hook
        );
        assert_eq!(
            output.variants[2].variant.variant_id,
            MetadataVariantId::Setting
        );
        assert_eq!(
            output.variants[0].create_handoff.name,
            output.variants[0].variant.title
        );
        assert_eq!(
            output.variants[0]
                .create_handoff
                .protagonist_seed
                .as_deref(),
            Some(output.variants[0].variant.protagonist_seed.as_str())
        );
    }
}

#[command]
pub async fn inspiration_generate_metadata_variants(
    input: InspirationGenerateMetadataVariantsInput,
) -> Result<InspirationGenerateMetadataVariantsOutput, AppError> {
    let generated = generate_metadata_variants(GenerateMetadataVariantsInput {
        consensus: input.consensus,
    })
    .await?;

    Ok(InspirationGenerateMetadataVariantsOutput {
        schema_version: 1,
        shared_story_core: generated.shared_story_core,
        variants: generated
            .variants
            .into_iter()
            .map(|variant| InspirationMetadataVariantCandidate {
                create_handoff: build_create_handoff(&variant),
                variant,
            })
            .collect(),
    })
}
