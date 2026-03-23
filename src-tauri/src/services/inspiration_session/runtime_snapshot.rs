use serde::{Deserialize, Serialize};

use crate::agent_engine::messages::ConversationState;
use crate::agent_engine::types::LoopConfig;
use crate::application::command_usecases::inspiration::{
    CreateProjectHandoffDraft, InspirationConsensusState, OpenQuestion,
};
use crate::models::{AppError, ErrorCode};
use crate::services::agent_session::SessionRuntimeState;
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::paths::{
    session_runtime_path, sessions_root, INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionRuntimeSnapshotV1 {
    pub schema_version: i32,
    pub session_id: String,
    pub updated_at: i64,
    pub runtime_state: SessionRuntimeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hydration_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_turn_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_config: Option<LoopConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consensus: Option<InspirationConsensusState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<OpenQuestion>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InspirationRuntimeSnapshotUpsertInput {
    pub session_id: String,
    pub runtime_state: SessionRuntimeState,
    pub hydration_source: Option<String>,
    pub last_turn: Option<u32>,
    pub next_turn_id: Option<u32>,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub system_prompt: Option<String>,
    pub loop_config: Option<LoopConfig>,
    pub conversation: Option<ConversationState>,
    pub consensus: Option<InspirationConsensusState>,
    pub open_questions: Option<Vec<OpenQuestion>>,
    pub final_create_handoff_draft: Option<CreateProjectHandoffDraft>,
    pub readonly_reason: Option<String>,
}

impl InspirationRuntimeSnapshotUpsertInput {
    pub fn ready(session_id: String) -> Self {
        Self {
            session_id,
            runtime_state: SessionRuntimeState::Ready,
            hydration_source: None,
            last_turn: Some(0),
            next_turn_id: Some(1),
            provider_name: None,
            model: None,
            base_url: None,
            system_prompt: None,
            loop_config: None,
            conversation: None,
            consensus: None,
            open_questions: None,
            final_create_handoff_draft: None,
            readonly_reason: None,
        }
    }

    pub fn from_conversation(
        session_id: String,
        runtime_state: SessionRuntimeState,
        conversation: ConversationState,
        last_turn: Option<u32>,
        provider_name: Option<String>,
        model: Option<String>,
        base_url: Option<String>,
        system_prompt: Option<String>,
        loop_config: Option<LoopConfig>,
    ) -> Self {
        let next_turn_id = crate::agent_engine::session_state::derive_next_turn_id(
            last_turn,
            Some(&conversation),
            None,
        );

        Self {
            session_id,
            runtime_state,
            hydration_source: None,
            last_turn,
            next_turn_id: Some(next_turn_id),
            provider_name,
            model,
            base_url,
            system_prompt,
            loop_config,
            conversation: Some(conversation),
            consensus: None,
            open_questions: None,
            final_create_handoff_draft: None,
            readonly_reason: None,
        }
    }

    pub fn with_hydration_source(mut self, hydration_source: Option<String>) -> Self {
        self.hydration_source = hydration_source;
        self
    }
}

impl InspirationSessionRuntimeSnapshotV1 {
    pub fn from_upsert(input: InspirationRuntimeSnapshotUpsertInput) -> Self {
        let next_turn_id = crate::agent_engine::session_state::derive_next_turn_id(
            input.last_turn,
            input.conversation.as_ref(),
            input.next_turn_id,
        );

        Self {
            schema_version: INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: input.session_id,
            updated_at: chrono::Utc::now().timestamp_millis(),
            runtime_state: input.runtime_state,
            hydration_source: input.hydration_source,
            last_turn: input.last_turn,
            next_turn_id: Some(next_turn_id),
            provider_name: input.provider_name,
            model: input.model,
            base_url: input.base_url,
            system_prompt: input.system_prompt,
            loop_config: input.loop_config,
            conversation: input.conversation,
            consensus: input.consensus,
            open_questions: input.open_questions,
            final_create_handoff_draft: input.final_create_handoff_draft,
            readonly_reason: input.readonly_reason,
        }
    }
}

pub fn load_runtime_snapshot(
    session_id: &str,
) -> Result<Option<InspirationSessionRuntimeSnapshotV1>, AppError> {
    let path = session_runtime_path(session_id)?;
    if !path.exists() {
        return Ok(None);
    }

    let mut snapshot: InspirationSessionRuntimeSnapshotV1 =
        read_json(&path).map_err(|err| AppError {
            code: ErrorCode::JsonParseError,
            message: format!("failed to parse inspiration runtime snapshot: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_RUNTIME_SNAPSHOT_PARSE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    if snapshot.schema_version != INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_VERSION {
        return Err(AppError {
            code: ErrorCode::SchemaVersionUnsupported,
            message: format!(
                "unsupported inspiration runtime snapshot schema version: {}",
                snapshot.schema_version
            ),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_UNSUPPORTED",
                "session_id": snapshot.session_id,
                "schema_version": snapshot.schema_version,
                "supported": INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            })),
            recoverable: Some(true),
        });
    }
    snapshot.next_turn_id = Some(crate::agent_engine::session_state::derive_next_turn_id(
        snapshot.last_turn,
        snapshot.conversation.as_ref(),
        snapshot.next_turn_id,
    ));

    Ok(Some(snapshot))
}

pub fn save_runtime_snapshot_from_input(
    input: InspirationRuntimeSnapshotUpsertInput,
) -> Result<InspirationSessionRuntimeSnapshotV1, AppError> {
    let root = sessions_root()?;
    ensure_dir(&root)?;
    let path = session_runtime_path(&input.session_id)?;
    let snapshot = InspirationSessionRuntimeSnapshotV1::from_upsert(input);

    atomic_write_json(&path, &snapshot).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write inspiration runtime snapshot: {err}"),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_RUNTIME_SNAPSHOT_WRITE_FAILED",
            "path": path.to_string_lossy(),
            "session_id": snapshot.session_id,
        })),
        recoverable: Some(true),
    })?;

    Ok(snapshot)
}
