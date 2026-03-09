use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

use crate::agent_engine::messages::ConversationState;
use crate::application::command_usecases::agent_session_support::{
    create_session_files, create_session_meta, create_session_start_event, delete_session,
    ensure_meta_exists, ensure_session_exists, list_session_metas, load_session_events,
    load_session_meta, normalize_active_chapter, normalize_title, recover_sessions,
    resolve_project_path, save_session_meta, update_session_meta,
};
use crate::models::{AppError, ErrorCode};
use crate::services::{
    append_session_events, load_openai_search_settings, load_runtime_snapshot,
    save_runtime_snapshot_from_input, AgentSessionEvent, AgentSessionMeta,
    RuntimeSnapshotUpsertInput, SessionRuntimeState, AGENT_SESSION_SCHEMA_VERSION,
    RUNTIME_SNAPSHOT_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionCreateInput {
    pub project_path: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub active_chapter_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionAppendEventsInput {
    pub project_path: String,
    pub session_id: String,
    pub events: Vec<AgentSessionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionLoadInput {
    pub project_path: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionListInput {
    pub project_path: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionUpdateMetaInput {
    pub project_path: String,
    pub session_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub active_chapter_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRecoverInput {
    pub project_path: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionDeleteInput {
    pub project_path: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionCreateOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionLoadOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub events: Vec<AgentSessionEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<AgentSessionMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRecoverOutput {
    pub schema_version: i32,
    pub repaired_files: i64,
    pub truncated_bytes: i64,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quarantined_sessions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_repair_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionHydrateInput {
    pub project_path: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionReadonlyReason {
    RuntimeStateUnavailable,
    HistoricalSuspendedSessionWithoutRuntimeSnapshot,
    ProviderCredentialsUnavailableForResume,
}

impl AgentSessionReadonlyReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RuntimeStateUnavailable => "runtime_state_unavailable",
            Self::HistoricalSuspendedSessionWithoutRuntimeSnapshot => {
                "historical_suspended_session_without_runtime_snapshot"
            }
            Self::ProviderCredentialsUnavailableForResume => {
                "provider_credentials_unavailable_for_resume"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionHydrateOutput {
    pub schema_version: i32,
    pub session_id: String,
    pub hydration_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hydration_source: Option<String>,
    pub runtime_state: String,
    pub can_continue: bool,
    pub can_resume: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason: Option<String>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_turn_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_revision: Option<u64>,
}

const HYDRATION_STATUS_MEMORY_HIT: &str = "memory_hit";
const HYDRATION_STATUS_SNAPSHOT_LOADED: &str = "snapshot_loaded";
const HYDRATION_STATUS_EVENT_REBUILT: &str = "event_rebuilt";
const HYDRATION_STATUS_READONLY_FALLBACK: &str = "readonly_fallback";

fn validate_events(input: &AgentSessionAppendEventsInput) -> Result<(), AppError> {
    if input.events.iter().any(|event| !event.validate_v1()) {
        return Err(AppError {
            code: ErrorCode::SchemaValidationError,
            message: "invalid session event schema".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_EVENT_INVALID",
                "schema_version": AGENT_SESSION_SCHEMA_VERSION,
            })),
            recoverable: Some(true),
        });
    }

    if input
        .events
        .iter()
        .any(|event| event.session_id != input.session_id)
    {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "event session_id does not match input session_id".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_EVENT_SESSION_MISMATCH",
                "session_id": input.session_id,
            })),
            recoverable: Some(true),
        });
    }

    Ok(())
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

fn normalize_readonly_reason(reason: Option<&str>) -> Option<String> {
    let value = reason?.trim();
    if value.is_empty() {
        return None;
    }

    Some(match value {
        "historical_suspended_session_without_runtime_snapshot" => {
            AgentSessionReadonlyReason::HistoricalSuspendedSessionWithoutRuntimeSnapshot
                .as_str()
                .to_string()
        }
        "provider_credentials_unavailable_for_resume" => {
            AgentSessionReadonlyReason::ProviderCredentialsUnavailableForResume
                .as_str()
                .to_string()
        }
        "runtime_state_unavailable" => AgentSessionReadonlyReason::RuntimeStateUnavailable
            .as_str()
            .to_string(),
        _ => AgentSessionReadonlyReason::RuntimeStateUnavailable
            .as_str()
            .to_string(),
    })
}

fn derive_runtime_next_turn_id(
    last_turn: Option<u32>,
    conversation: Option<&ConversationState>,
    persisted_next_turn_id: Option<u32>,
) -> u32 {
    crate::agent_engine::session_state::derive_next_turn_id(
        last_turn,
        conversation,
        persisted_next_turn_id,
    )
}

fn runtime_unavailable_error(
    session_id: &str,
    provider_name: Option<&str>,
    hydration_status: Option<&str>,
    runtime_state: Option<&str>,
    readonly_reason: Option<&str>,
    message: &str,
) -> AppError {
    AppError {
        code: ErrorCode::InvalidArgument,
        message: message.to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_UNAVAILABLE",
            "session_id": session_id,
            "provider_name": provider_name,
            "hydration_status": hydration_status,
            "runtime_state": runtime_state,
            "readonly_reason": readonly_reason,
        })),
        recoverable: Some(true),
    }
}

fn hydrate_provider_credentials(
    session_id: &str,
    provider_name: Option<&str>,
    snapshot_base_url: Option<&str>,
) -> Result<(Option<String>, Option<String>, Vec<String>), AppError> {
    let mut warnings = Vec::new();
    let provider = provider_name.unwrap_or_default();

    if provider.is_empty() {
        return Ok((None, None, warnings));
    }

    let settings = load_openai_search_settings()?;
    let mut base_url = settings.openai_base_url.trim().to_string();
    let api_key = settings.openai_api_key.trim().to_string();

    if let Some(snapshot_url) = snapshot_base_url {
        let trimmed = snapshot_url.trim();
        if !trimmed.is_empty() {
            if base_url.is_empty() {
                base_url = trimmed.to_string();
                warnings.push("provider_base_url_loaded_from_snapshot".to_string());
            } else if trimmed != base_url {
                warnings.push("provider_base_url_changed_since_snapshot".to_string());
            }
        }
    }

    if base_url.is_empty() || api_key.is_empty() {
        tracing::warn!(
            target: "agent_session",
            session_id,
            provider_name = provider,
            "missing provider credentials for hydrate"
        );
        return Err(runtime_unavailable_error(
            session_id,
            Some(provider),
            Some(HYDRATION_STATUS_SNAPSHOT_LOADED),
            Some(runtime_state_label(&SessionRuntimeState::Degraded)),
            Some(AgentSessionReadonlyReason::ProviderCredentialsUnavailableForResume.as_str()),
            "missing provider credentials for hydrate",
        ));
    }

    Ok((Some(base_url), Some(api_key), warnings))
}

fn load_or_rebuild_conversation(
    project_path: &std::path::Path,
    session_id: &str,
) -> Result<Option<ConversationState>, AppError> {
    use crate::agent_engine::messages::{AgentMessage, ContentBlock};

    let events = load_session_events(project_path, session_id)?;
    if events.is_empty() {
        return Ok(None);
    }

    let mut state = ConversationState::new(session_id.to_string());
    let mut has_message = false;
    let mut max_turn: u32 = 0;

    for event in events {
        if let Some(turn) = event.turn {
            if turn > 0 {
                max_turn = max_turn.max(turn as u32);
            }
        }

        if event.event_type != "message" {
            continue;
        }

        let payload = match event.payload {
            Some(v) => v,
            None => continue,
        };

        let role = payload
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_lowercase();

        let content = payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let id = payload
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("msg_evt_{}_{}", event.ts, state.messages.len()));

        let msg_role = match role.as_str() {
            "system" => crate::agent_engine::messages::Role::System,
            "user" => crate::agent_engine::messages::Role::User,
            "assistant" => crate::agent_engine::messages::Role::Assistant,
            "tool" => crate::agent_engine::messages::Role::Tool,
            _ => continue,
        };

        state.messages.push(AgentMessage {
            id,
            role: msg_role,
            blocks: vec![ContentBlock::Text { text: content }],
            ts: event.ts,
        });
        has_message = true;
    }

    if !has_message {
        return Ok(None);
    }

    state.current_turn = max_turn;
    Ok(Some(state))
}

#[command]
pub async fn agent_session_create(
    input: AgentSessionCreateInput,
) -> Result<AgentSessionCreateOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    let now = Utc::now().timestamp_millis();
    let session_id = format!("chat_{}_{}", now, Uuid::new_v4().simple());

    let title = normalize_title(input.title);
    let active_chapter_path = normalize_active_chapter(input.active_chapter_path);

    let start_event = create_session_start_event(
        &session_id,
        now,
        &input.project_path,
        active_chapter_path.as_deref(),
    );

    create_session_files(&project_path, &session_id, &start_event)?;

    let meta = create_session_meta(session_id.clone(), now, title, active_chapter_path);
    save_session_meta(&project_path, meta)?;

    let snapshot_input = RuntimeSnapshotUpsertInput::ready(session_id.clone());
    save_runtime_snapshot_from_input(&project_path, snapshot_input)?;

    Ok(AgentSessionCreateOutput {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id,
        created_at: now,
    })
}

#[command]
pub async fn agent_session_append_events(
    input: AgentSessionAppendEventsInput,
) -> Result<(), AppError> {
    validate_events(&input)?;

    let project_path = resolve_project_path(&input.project_path)?;
    ensure_session_exists(&project_path, &input.session_id)?;

    ensure_meta_exists(&project_path, &input.session_id)?;

    let append_result = match append_session_events(&project_path, &input.session_id, &input.events)
    {
        Ok(result) => {
            tracing::info!(
                target: "agent_session",
                session_id = %input.session_id,
                metric = "agent_session_append_events_success_count",
                value = 1_u8,
                appended_count = result.appended_count,
                deduped_count = result.deduped_count,
                last_event_seq = result.last_event_seq,
                "agent_session_append_events succeeded"
            );
            result
        }
        Err(error) => {
            tracing::warn!(
                target: "agent_session",
                session_id = %input.session_id,
                metric = "agent_session_append_events_error_count",
                value = 1_u8,
                error = %error,
                "agent_session_append_events failed"
            );
            return Err(error);
        }
    };

    if append_result.deduped_count > 0 {
        tracing::info!(
            target: "agent_session",
            session_id = %input.session_id,
            deduped_count = append_result.deduped_count,
            appended_count = append_result.appended_count,
            last_event_seq = append_result.last_event_seq,
            "agent_session_append_events dropped duplicated dedupe_key events"
        );
    }

    Ok(())
}

#[command]
pub async fn agent_session_load(
    input: AgentSessionLoadInput,
) -> Result<AgentSessionLoadOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    ensure_session_exists(&project_path, &input.session_id)?;
    ensure_meta_exists(&project_path, &input.session_id)?;

    let events = load_session_events(&project_path, &input.session_id)?;
    let meta = load_session_meta(&project_path, &input.session_id)?;

    Ok(AgentSessionLoadOutput {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id: input.session_id,
        events,
        meta,
    })
}

#[command]
pub async fn agent_session_list(
    input: AgentSessionListInput,
) -> Result<Vec<AgentSessionMeta>, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    list_session_metas(&project_path, input.limit)
}

#[command]
pub async fn agent_session_update_meta(input: AgentSessionUpdateMetaInput) -> Result<(), AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    ensure_meta_exists(&project_path, &input.session_id)?;

    update_session_meta(
        &project_path,
        &input.session_id,
        input.title,
        input.active_chapter_path,
    )
}

#[command]
pub async fn agent_session_recover(
    input: AgentSessionRecoverInput,
) -> Result<AgentSessionRecoverOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;

    if let Some(session_id) = input.session_id.as_deref() {
        ensure_session_exists(&project_path, session_id)?;
        ensure_meta_exists(&project_path, session_id)?;
    }

    let recovered = recover_sessions(&project_path, input.session_id.as_deref())?;

    Ok(AgentSessionRecoverOutput {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        repaired_files: recovered.repaired_files,
        truncated_bytes: recovered.truncated_bytes,
        notes: recovered.notes,
        quarantined_sessions: recovered.quarantined_sessions,
        manual_repair_actions: recovered.manual_repair_actions,
    })
}

#[command]
pub async fn agent_session_delete(input: AgentSessionDeleteInput) -> Result<(), AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    delete_session(&project_path, &input.session_id)
}

fn suspended_reason_runtime_state(
    reason: &crate::agent_engine::types::StopReason,
) -> SessionRuntimeState {
    match reason {
        crate::agent_engine::types::StopReason::WaitingConfirmation => {
            SessionRuntimeState::SuspendedConfirmation
        }
        _ => SessionRuntimeState::SuspendedAskuser,
    }
}

pub fn hydrate_runtime_state(
    project_path: &std::path::Path,
    project_path_raw: &str,
    session_id: &str,
) -> Result<AgentSessionHydrateOutput, AppError> {
    ensure_session_exists(project_path, session_id)?;
    ensure_meta_exists(project_path, session_id)?;

    if let Some(suspended) = crate::agent_engine::session_state::global().take_suspended(session_id)
    {
        let next_turn_id = derive_runtime_next_turn_id(
            Some(suspended.conversation_state.current_turn),
            Some(&suspended.conversation_state),
            Some(crate::agent_engine::session_state::global().peek_next_turn_id(session_id)),
        );
        crate::agent_engine::session_state::global().save_suspended_runtime_state(
            session_id,
            suspended.clone(),
            next_turn_id,
        );
        let runtime_state = suspended_reason_runtime_state(&suspended.suspend_reason);

        return Ok(AgentSessionHydrateOutput {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            hydration_status: HYDRATION_STATUS_MEMORY_HIT.to_string(),
            hydration_source: Some(HYDRATION_STATUS_MEMORY_HIT.to_string()),
            runtime_state: runtime_state_label(&runtime_state).to_string(),
            can_continue: false,
            can_resume: true,
            readonly_reason: None,
            warnings: Vec::new(),
            last_turn: Some(suspended.conversation_state.current_turn),
            next_turn_id: Some(next_turn_id),
            active_skill: suspended.active_skill.clone(),
            session_revision: None,
        });
    }

    if let Some(conversation) =
        crate::agent_engine::session_state::global().take_conversation(session_id)
    {
        let turn = conversation.current_turn;
        let active_skill_from_snapshot = load_runtime_snapshot(project_path, session_id)
            .ok()
            .and_then(|snapshot| snapshot.and_then(|snapshot| snapshot.active_skill));
        let next_turn_id = derive_runtime_next_turn_id(
            Some(turn),
            Some(&conversation),
            Some(crate::agent_engine::session_state::global().peek_next_turn_id(session_id)),
        );
        crate::agent_engine::session_state::global().save_runtime_state(
            session_id,
            conversation,
            next_turn_id,
            None,
        );

        return Ok(AgentSessionHydrateOutput {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            hydration_status: HYDRATION_STATUS_MEMORY_HIT.to_string(),
            hydration_source: Some(HYDRATION_STATUS_MEMORY_HIT.to_string()),
            runtime_state: runtime_state_label(&SessionRuntimeState::Ready).to_string(),
            can_continue: true,
            can_resume: false,
            readonly_reason: None,
            warnings: Vec::new(),
            last_turn: Some(turn),
            next_turn_id: Some(next_turn_id),
            active_skill: active_skill_from_snapshot,
            session_revision: None,
        });
    }

    if let Some(meta) = load_session_meta(project_path, session_id)? {
        if meta.last_turn.is_none() {
            let empty_conversation = ConversationState::new(session_id.to_string());
            let next_turn_id =
                derive_runtime_next_turn_id(Some(0), Some(&empty_conversation), None);
            crate::agent_engine::session_state::global().save_runtime_state(
                session_id,
                empty_conversation,
                next_turn_id,
                None,
            );

            return Ok(AgentSessionHydrateOutput {
                schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
                session_id: session_id.to_string(),
                hydration_status: HYDRATION_STATUS_EVENT_REBUILT.to_string(),
                hydration_source: Some(HYDRATION_STATUS_EVENT_REBUILT.to_string()),
                runtime_state: runtime_state_label(&SessionRuntimeState::Ready).to_string(),
                can_continue: true,
                can_resume: false,
                readonly_reason: None,
                warnings: vec!["new_session_without_runtime_state".to_string()],
                last_turn: Some(0),
                next_turn_id: Some(next_turn_id),
                active_skill: None,
                session_revision: None,
            });
        }
    }

    if let Some(snapshot) = load_runtime_snapshot(project_path, session_id)? {
        let mut warnings = Vec::new();
        let next_turn_id = derive_runtime_next_turn_id(
            snapshot.last_turn,
            snapshot.conversation.as_ref(),
            snapshot.next_turn_id,
        );
        let mut runtime_state = snapshot.runtime_state.clone();
        let mut can_continue = runtime_state.can_continue() && snapshot.conversation.is_some();
        let mut can_resume = runtime_state.can_resume();
        let mut readonly_reason = snapshot.readonly_reason.clone();

        if snapshot.runtime_state.can_continue() && snapshot.conversation.is_none() {
            warnings.push("runtime_snapshot_missing_conversation_state".to_string());
            runtime_state = SessionRuntimeState::Degraded;
            can_continue = false;
            readonly_reason = Some(
                AgentSessionReadonlyReason::RuntimeStateUnavailable
                    .as_str()
                    .to_string(),
            );
            tracing::warn!(
                target: "agent_session",
                session_id,
                runtime_state = ?snapshot.runtime_state,
                "runtime snapshot missing conversation state"
            );
        }

        if can_continue {
            if let Some(conversation) = snapshot.conversation.clone() {
                crate::agent_engine::session_state::global().save_runtime_state(
                    session_id,
                    conversation,
                    next_turn_id,
                    None,
                );
            }
        }

        if snapshot.runtime_state.can_resume() {
            match hydrate_provider_credentials(
                session_id,
                snapshot.provider_name.as_deref(),
                snapshot.base_url.as_deref(),
            ) {
                Ok((resolved_base_url, resolved_api_key, provider_warnings)) => {
                    warnings.extend(provider_warnings);

                    let api_key = resolved_api_key.ok_or_else(|| {
                        runtime_unavailable_error(
                            session_id,
                            snapshot.provider_name.as_deref(),
                            Some(HYDRATION_STATUS_SNAPSHOT_LOADED),
                            Some(runtime_state_label(&SessionRuntimeState::Degraded)),
                            Some(
                                AgentSessionReadonlyReason::ProviderCredentialsUnavailableForResume
                                    .as_str(),
                            ),
                            "missing provider credentials for suspended hydrate",
                        )
                    })?;

                    if let Some(suspended) = snapshot.to_suspended_state(
                        project_path_raw.to_string(),
                        resolved_base_url,
                        api_key,
                    )? {
                        crate::agent_engine::session_state::global().save_suspended_runtime_state(
                            session_id,
                            suspended,
                            next_turn_id,
                        );
                    } else {
                        warnings.push("runtime_snapshot_missing_suspended_state".to_string());
                        runtime_state = SessionRuntimeState::Degraded;
                        can_resume = false;
                        readonly_reason = Some(
                            AgentSessionReadonlyReason::RuntimeStateUnavailable
                                .as_str()
                                .to_string(),
                        );
                        tracing::warn!(
                            target: "agent_session",
                            session_id,
                            "runtime snapshot missing suspended state"
                        );
                    }
                }
                Err(_) => {
                    warnings.push("provider_credentials_unavailable_for_resume".to_string());
                    runtime_state = SessionRuntimeState::Degraded;
                    can_continue = false;
                    can_resume = false;
                    readonly_reason = Some(
                        AgentSessionReadonlyReason::ProviderCredentialsUnavailableForResume
                            .as_str()
                            .to_string(),
                    );
                    tracing::warn!(
                        target: "agent_session",
                        session_id,
                        "provider credentials unavailable for suspended resume"
                    );
                }
            }
        }

        return Ok(AgentSessionHydrateOutput {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            hydration_status: HYDRATION_STATUS_SNAPSHOT_LOADED.to_string(),
            hydration_source: snapshot
                .hydration_source
                .clone()
                .or_else(|| Some(HYDRATION_STATUS_SNAPSHOT_LOADED.to_string())),
            runtime_state: runtime_state_label(&runtime_state).to_string(),
            can_continue,
            can_resume,
            readonly_reason: normalize_readonly_reason(readonly_reason.as_deref()),
            warnings,
            last_turn: snapshot.last_turn,
            next_turn_id: Some(next_turn_id),
            active_skill: snapshot.active_skill.clone(),
            session_revision: snapshot.session_revision,
        });
    }

    let events = load_session_events(project_path, session_id)?;
    let last_state = events
        .iter()
        .rev()
        .find(|evt| evt.event_type == "turn_state")
        .and_then(|evt| evt.payload.as_ref())
        .and_then(|payload| payload.get("state"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    if matches!(
        last_state.as_deref(),
        Some("waiting_confirmation" | "waiting_askuser")
    ) {
        tracing::warn!(
            target: "agent_session",
            session_id,
            "historical suspended session missing runtime snapshot; entering readonly fallback"
        );
        let readonly_snapshot = save_runtime_snapshot_from_input(
            project_path,
            RuntimeSnapshotUpsertInput::readonly(
                session_id.to_string(),
                AgentSessionReadonlyReason::HistoricalSuspendedSessionWithoutRuntimeSnapshot
                    .as_str()
                    .to_string(),
                None,
            ),
        )?;

        return Ok(AgentSessionHydrateOutput {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            hydration_status: HYDRATION_STATUS_READONLY_FALLBACK.to_string(),
            hydration_source: readonly_snapshot
                .hydration_source
                .clone()
                .or_else(|| Some(HYDRATION_STATUS_READONLY_FALLBACK.to_string())),
            runtime_state: runtime_state_label(&SessionRuntimeState::Degraded).to_string(),
            can_continue: false,
            can_resume: false,
            readonly_reason: normalize_readonly_reason(
                readonly_snapshot.readonly_reason.as_deref(),
            ),
            warnings: vec![
                "snapshot_missing_for_suspended_session".to_string(),
                "resume_requires_runtime_snapshot".to_string(),
            ],
            last_turn: readonly_snapshot.last_turn,
            next_turn_id: readonly_snapshot.next_turn_id,
            active_skill: readonly_snapshot.active_skill.clone(),
            session_revision: readonly_snapshot.session_revision,
        });
    }

    if let Some(conversation) = load_or_rebuild_conversation(project_path, session_id)? {
        let last_turn = conversation.current_turn;
        let inferred_system_prompt = conversation
            .messages
            .iter()
            .find_map(|msg| {
                if matches!(msg.role, crate::agent_engine::messages::Role::System) {
                    Some(msg.text_content())
                } else {
                    None
                }
            })
            .filter(|text| !text.trim().is_empty());

        let next_turn_id = derive_runtime_next_turn_id(Some(last_turn), Some(&conversation), None);

        crate::agent_engine::session_state::global().save_runtime_state(
            session_id,
            conversation.clone(),
            next_turn_id,
            None,
        );

        save_runtime_snapshot_from_input(
            project_path,
            RuntimeSnapshotUpsertInput::from_conversation(
                session_id.to_string(),
                SessionRuntimeState::Ready,
                conversation,
                Some(last_turn),
                None,
                None,
                None,
                inferred_system_prompt,
                None,
                None,
            ),
        )?;

        return Ok(AgentSessionHydrateOutput {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            hydration_status: HYDRATION_STATUS_EVENT_REBUILT.to_string(),
            hydration_source: Some(HYDRATION_STATUS_EVENT_REBUILT.to_string()),
            runtime_state: runtime_state_label(&SessionRuntimeState::Ready).to_string(),
            can_continue: true,
            can_resume: false,
            readonly_reason: None,
            warnings: vec!["runtime_snapshot_rebuilt_from_event_log".to_string()],
            last_turn: Some(last_turn),
            next_turn_id: Some(next_turn_id),
            active_skill: None,
            session_revision: None,
        });
    }

    let readonly_snapshot = save_runtime_snapshot_from_input(
        project_path,
        RuntimeSnapshotUpsertInput::readonly(
            session_id.to_string(),
            AgentSessionReadonlyReason::RuntimeStateUnavailable
                .as_str()
                .to_string(),
            None,
        ),
    )?;

    tracing::warn!(
        target: "agent_session",
        session_id,
        "runtime snapshot missing and event rebuild unavailable; entering readonly fallback"
    );

    Ok(AgentSessionHydrateOutput {
        schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        hydration_status: HYDRATION_STATUS_READONLY_FALLBACK.to_string(),
        hydration_source: readonly_snapshot
            .hydration_source
            .clone()
            .or_else(|| Some(HYDRATION_STATUS_READONLY_FALLBACK.to_string())),
        runtime_state: runtime_state_label(&SessionRuntimeState::Degraded).to_string(),
        can_continue: false,
        can_resume: false,
        readonly_reason: normalize_readonly_reason(readonly_snapshot.readonly_reason.as_deref()),
        warnings: vec!["runtime_snapshot_missing_and_event_rebuild_unavailable".to_string()],
        last_turn: readonly_snapshot.last_turn,
        next_turn_id: readonly_snapshot.next_turn_id,
        active_skill: readonly_snapshot.active_skill.clone(),
        session_revision: readonly_snapshot.session_revision,
    })
}

#[command]
pub async fn agent_session_hydrate(
    input: AgentSessionHydrateInput,
) -> Result<AgentSessionHydrateOutput, AppError> {
    let project_path = resolve_project_path(&input.project_path)?;
    hydrate_runtime_state(&project_path, &input.project_path, &input.session_id)
}
