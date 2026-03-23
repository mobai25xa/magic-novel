use chrono::Utc;
use serde::de::DeserializeOwned;

use crate::agent_engine::messages::{AgentMessage, ContentBlock, ConversationState, Role};
use crate::models::{AppError, ErrorCode};
use crate::services::agent_session::SessionRuntimeState;
use crate::services::inspiration_session::{
    append_session_events, ensure_meta_exists, load_runtime_snapshot, load_session_events,
    load_session_meta, save_runtime_snapshot_from_input, save_session_meta,
    InspirationRuntimeSnapshotUpsertInput, InspirationSessionEvent, InspirationSessionMeta,
    INSPIRATION_SESSION_SCHEMA_VERSION,
};

use super::{
    ApplyConsensusPatchOutput, ApplyOpenQuestionsPatchOutput, CreateProjectHandoffDraft,
    InspirationConsensusState, OpenQuestion,
};

pub const HYDRATION_STATUS_SNAPSHOT_LOADED: &str = "snapshot_loaded";
pub const HYDRATION_STATUS_EVENT_REBUILT: &str = "event_rebuilt";
pub const HYDRATION_STATUS_NEW_SESSION: &str = "new_session";

fn normalize_title(title: Option<String>) -> Option<String> {
    title.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn create_session_start_event(session_id: &str, now: i64) -> InspirationSessionEvent {
    InspirationSessionEvent {
        schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
        event_type: "session_start".to_string(),
        session_id: session_id.to_string(),
        ts: now,
        event_id: Some(format!("evt_start_{}_{}", now, session_id)),
        event_seq: Some(1),
        dedupe_key: Some("session_start".to_string()),
        turn: None,
        payload: Some(serde_json::json!({
            "scope": "inspiration",
        })),
    }
}

fn rebuild_conversation_from_events(
    session_id: &str,
    events: &[InspirationSessionEvent],
) -> ConversationState {
    let mut state = ConversationState::new(session_id.to_string());
    let mut max_turn = 0_u32;

    for event in events {
        if let Some(turn) = event.turn.filter(|turn| *turn > 0) {
            max_turn = max_turn.max(turn as u32);
        }

        if event.event_type != "message" {
            continue;
        }

        let payload = match event.payload.as_ref() {
            Some(value) => value,
            None => continue,
        };

        let role = match payload
            .get("role")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
        {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => continue,
        };

        let content = payload
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();

        let id = payload
            .get("message_id")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("msg_evt_{}_{}", event.ts, state.messages.len()));

        state.messages.push(AgentMessage {
            id,
            role,
            blocks: vec![ContentBlock::Text { text: content }],
            ts: event.ts,
        });
    }

    state.current_turn = max_turn;
    state
}

fn parse_successful_tool_result<T>(message: &AgentMessage, expected_tool_name: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    message.blocks.iter().find_map(|block| match block {
        ContentBlock::ToolResult {
            tool_name: Some(tool_name),
            content,
            is_error,
            ..
        } if !*is_error && tool_name == expected_tool_name => serde_json::from_str(content).ok(),
        _ => None,
    })
}

pub fn derive_inspiration_domain_state(
    conversation: &ConversationState,
) -> DerivedInspirationDomainState {
    let mut consensus = InspirationConsensusState::default();
    let mut open_questions = Vec::new();

    for message in &conversation.messages {
        if let Some(output) = parse_successful_tool_result::<ApplyConsensusPatchOutput>(
            message,
            "inspiration_consensus_patch",
        ) {
            *consensus.field_mut(output.field_id) = output.updated_field;
        }

        if let Some(output) = parse_successful_tool_result::<ApplyOpenQuestionsPatchOutput>(
            message,
            "inspiration_open_questions_patch",
        ) {
            open_questions = output.questions;
        }
    }

    DerivedInspirationDomainState {
        consensus,
        open_questions,
    }
}

pub fn create_inspiration_session(title: Option<String>) -> Result<(String, i64), AppError> {
    let now = Utc::now().timestamp_millis();
    let session_id = format!("insp_{}_{}", now, uuid::Uuid::new_v4().simple());
    let meta = InspirationSessionMeta::new(session_id.clone(), now, normalize_title(title));

    save_session_meta(meta)?;
    let append_result =
        append_session_events(&session_id, &[create_session_start_event(&session_id, now)])?;
    tracing::debug!(
        target: "inspiration",
        session_id = %session_id,
        appended_count = append_result.appended_count,
        deduped_count = append_result.deduped_count,
        last_event_seq = append_result.last_event_seq,
        "created inspiration session stream seed event"
    );
    save_runtime_snapshot_from_input(InspirationRuntimeSnapshotUpsertInput::ready(
        session_id.clone(),
    ))?;

    Ok((session_id, now))
}

pub fn ensure_inspiration_session_exists(session_id: &str) -> Result<(), AppError> {
    ensure_meta_exists(session_id)
}

pub fn save_inspiration_session_state(
    session_id: &str,
    consensus: InspirationConsensusState,
    open_questions: Vec<OpenQuestion>,
    final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
) -> Result<LoadedInspirationSessionSnapshot, AppError> {
    ensure_inspiration_session_exists(session_id)?;

    let loaded = load_inspiration_session_snapshot(session_id)?;
    let runtime_snapshot = load_runtime_snapshot(session_id)?;
    let conversation = loaded.conversation.clone();
    let mut input = match runtime_snapshot {
        Some(snapshot) => InspirationRuntimeSnapshotUpsertInput {
            session_id: session_id.to_string(),
            runtime_state: if crate::agent_engine::session_state::global()
                .has_active_turn(session_id)
            {
                SessionRuntimeState::Running
            } else {
                snapshot.runtime_state
            },
            hydration_source: snapshot.hydration_source,
            last_turn: snapshot.last_turn.or(Some(conversation.current_turn)),
            next_turn_id: snapshot.next_turn_id,
            provider_name: snapshot.provider_name,
            model: snapshot.model,
            base_url: snapshot.base_url,
            system_prompt: snapshot.system_prompt,
            loop_config: snapshot.loop_config,
            conversation: Some(conversation),
            consensus: Some(consensus),
            open_questions: Some(open_questions),
            final_create_handoff_draft,
            readonly_reason: snapshot.readonly_reason,
        },
        None => {
            let mut input = InspirationRuntimeSnapshotUpsertInput::from_conversation(
                session_id.to_string(),
                loaded.runtime_state,
                conversation,
                loaded.last_turn,
                None,
                None,
                None,
                None,
                None,
            );
            input.consensus = Some(consensus);
            input.open_questions = Some(open_questions);
            input.final_create_handoff_draft = final_create_handoff_draft;
            input
        }
    };
    input.last_turn = input.last_turn.or(Some(loaded.conversation.current_turn));

    save_runtime_snapshot_from_input(input)?;
    load_inspiration_session_snapshot(session_id)
}

pub fn load_inspiration_session_snapshot(
    session_id: &str,
) -> Result<LoadedInspirationSessionSnapshot, AppError> {
    let meta = load_session_meta(session_id)?.ok_or_else(|| AppError {
        code: ErrorCode::NotFound,
        message: "inspiration session metadata not found".to_string(),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_NOT_FOUND",
            "session_id": session_id,
        })),
        recoverable: Some(false),
    })?;

    let events = load_session_events(session_id)?;
    let runtime_snapshot = load_runtime_snapshot(session_id)?;
    let (conversation, hydration_status, last_turn, next_turn_id, runtime_state) =
        if let Some(snapshot) = runtime_snapshot.clone() {
            let conversation = snapshot
                .conversation
                .unwrap_or_else(|| rebuild_conversation_from_events(session_id, &events));
            let last_turn = snapshot.last_turn.or(Some(conversation.current_turn));
            let next_turn_id = Some(crate::agent_engine::session_state::derive_next_turn_id(
                last_turn,
                Some(&conversation),
                snapshot.next_turn_id,
            ));
            (
                conversation,
                snapshot
                    .hydration_source
                    .unwrap_or_else(|| HYDRATION_STATUS_SNAPSHOT_LOADED.to_string()),
                last_turn,
                next_turn_id,
                snapshot.runtime_state,
            )
        } else if events.is_empty() {
            (
                ConversationState::new(session_id.to_string()),
                HYDRATION_STATUS_NEW_SESSION.to_string(),
                Some(0),
                Some(1),
                SessionRuntimeState::Ready,
            )
        } else {
            let conversation = rebuild_conversation_from_events(session_id, &events);
            let last_turn = Some(conversation.current_turn);
            let next_turn_id = Some(crate::agent_engine::session_state::derive_next_turn_id(
                last_turn,
                Some(&conversation),
                None,
            ));
            (
                conversation,
                HYDRATION_STATUS_EVENT_REBUILT.to_string(),
                last_turn,
                next_turn_id,
                SessionRuntimeState::Ready,
            )
        };

    let runtime_state = if crate::agent_engine::session_state::global().has_active_turn(session_id)
    {
        SessionRuntimeState::Running
    } else {
        runtime_state
    };

    let derived_domain = derive_inspiration_domain_state(&conversation);
    let consensus = runtime_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.consensus.clone())
        .unwrap_or(derived_domain.consensus);
    let open_questions = runtime_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.open_questions.clone())
        .unwrap_or(derived_domain.open_questions);
    let final_create_handoff_draft = runtime_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.final_create_handoff_draft.clone());

    Ok(LoadedInspirationSessionSnapshot {
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
    })
}

#[derive(Debug, Clone)]
pub struct LoadedInspirationSessionSnapshot {
    pub meta: InspirationSessionMeta,
    pub events: Vec<InspirationSessionEvent>,
    pub conversation: ConversationState,
    pub consensus: InspirationConsensusState,
    pub open_questions: Vec<OpenQuestion>,
    pub final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
    pub runtime_state: SessionRuntimeState,
    pub hydration_status: String,
    pub last_turn: Option<u32>,
    pub next_turn_id: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct DerivedInspirationDomainState {
    pub consensus: InspirationConsensusState,
    pub open_questions: Vec<OpenQuestion>,
}
