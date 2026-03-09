use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent_engine::messages::ConversationState;
use crate::agent_engine::session_state::SuspendedTurnState;
use crate::agent_engine::types::{LoopConfig, StopReason, ToolCallInfo};
use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::paths::sessions_root;

pub const RUNTIME_SNAPSHOT_SCHEMA_VERSION: i32 = 1;
pub const RUNTIME_SUFFIX: &str = ".runtime.json";

fn normalize_active_skill(active_skill: Option<String>) -> Option<String> {
    active_skill
        .map(|skill| skill.trim().to_string())
        .filter(|skill| !skill.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionRuntimeState {
    Ready,
    Running,
    SuspendedConfirmation,
    SuspendedAskuser,
    Completed,
    Failed,
    Cancelled,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionSuspendedSnapshot {
    pub kind: SuspendedKind,
    pub pending_call_id: String,
    pub pending_tool_call: ToolCallInfo,
    pub remaining_tool_calls: Vec<ToolCallInfo>,
    pub completed_messages: Vec<crate::agent_engine::messages::AgentMessage>,
    pub rounds_executed: u32,
    pub total_tool_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuspendedKind {
    Confirmation,
    Askuser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRuntimeSnapshotV1 {
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
    pub active_chapter_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_config: Option<LoopConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended: Option<AgentSessionSuspendedSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_revision: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshotUpsertInput {
    pub session_id: String,
    pub runtime_state: SessionRuntimeState,
    pub hydration_source: Option<String>,
    pub last_turn: Option<u32>,
    pub next_turn_id: Option<u32>,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub system_prompt: Option<String>,
    pub active_chapter_path: Option<String>,
    pub active_skill: Option<String>,
    pub loop_config: Option<LoopConfig>,
    pub conversation: Option<ConversationState>,
    pub suspended: Option<SuspendedTurnState>,
    pub readonly_reason: Option<String>,
    pub session_revision: Option<u64>,
}

impl RuntimeSnapshotUpsertInput {
    pub fn from_conversation(
        session_id: String,
        runtime_state: SessionRuntimeState,
        conversation: ConversationState,
        last_turn: Option<u32>,
        provider_name: Option<String>,
        model: Option<String>,
        base_url: Option<String>,
        system_prompt: Option<String>,
        active_chapter_path: Option<String>,
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
            active_chapter_path,
            active_skill: None,
            loop_config,
            conversation: Some(conversation),
            suspended: None,
            readonly_reason: None,
            session_revision: None,
        }
    }

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
            active_chapter_path: None,
            active_skill: None,
            loop_config: None,
            conversation: None,
            suspended: None,
            readonly_reason: None,
            session_revision: None,
        }
    }

    pub fn from_suspended(
        session_id: String,
        mut suspended: SuspendedTurnState,
        last_turn: Option<u32>,
    ) -> Self {
        let runtime_state = match suspended.suspend_reason {
            StopReason::WaitingConfirmation => SessionRuntimeState::SuspendedConfirmation,
            StopReason::WaitingAskuser => SessionRuntimeState::SuspendedAskuser,
            StopReason::Cancel => SessionRuntimeState::Cancelled,
            StopReason::Error => SessionRuntimeState::Failed,
            _ => SessionRuntimeState::Degraded,
        };

        if !matches!(
            suspended.suspend_reason,
            StopReason::WaitingConfirmation | StopReason::WaitingAskuser
        ) {
            suspended.pending_call_id.clear();
            suspended.remaining_tool_calls.clear();
            suspended.completed_messages.clear();
        }

        let next_turn_id = crate::agent_engine::session_state::derive_next_turn_id(
            last_turn,
            Some(&suspended.conversation_state),
            None,
        );

        Self {
            session_id,
            runtime_state,
            hydration_source: None,
            last_turn,
            next_turn_id: Some(next_turn_id),
            provider_name: Some(suspended.provider_name.clone()),
            model: Some(suspended.model.clone()),
            base_url: Some(suspended.base_url.clone()),
            system_prompt: suspended.system_prompt.clone(),
            active_chapter_path: suspended.active_chapter_path.clone(),
            active_skill: suspended.active_skill.clone(),
            loop_config: Some(suspended.loop_config.clone()),
            conversation: Some(suspended.conversation_state.clone()),
            suspended: if matches!(
                suspended.suspend_reason,
                StopReason::WaitingConfirmation | StopReason::WaitingAskuser
            ) {
                Some(suspended)
            } else {
                None
            },
            readonly_reason: None,
            session_revision: None,
        }
    }

    pub fn readonly(session_id: String, reason: String, last_turn: Option<u32>) -> Self {
        let next_turn_id =
            crate::agent_engine::session_state::derive_next_turn_id(last_turn, None, None);

        Self {
            session_id,
            runtime_state: SessionRuntimeState::Degraded,
            hydration_source: Some("readonly_fallback".to_string()),
            last_turn,
            next_turn_id: Some(next_turn_id),
            provider_name: None,
            model: None,
            base_url: None,
            system_prompt: None,
            active_chapter_path: None,
            active_skill: None,
            loop_config: None,
            conversation: None,
            suspended: None,
            readonly_reason: Some(reason),
            session_revision: None,
        }
    }

    pub fn with_active_skill(mut self, active_skill: Option<String>) -> Self {
        self.active_skill = normalize_active_skill(active_skill);
        self
    }
}

impl SessionRuntimeState {
    pub fn can_continue(&self) -> bool {
        matches!(
            self,
            SessionRuntimeState::Ready
                | SessionRuntimeState::Completed
                | SessionRuntimeState::Failed
                | SessionRuntimeState::Cancelled
        )
    }

    pub fn can_resume(&self) -> bool {
        matches!(
            self,
            SessionRuntimeState::SuspendedConfirmation | SessionRuntimeState::SuspendedAskuser
        )
    }
}

impl AgentSessionRuntimeSnapshotV1 {
    pub fn to_suspended_state(
        &self,
        project_path: String,
        base_url_override: Option<String>,
        api_key: String,
    ) -> Result<Option<SuspendedTurnState>, AppError> {
        let suspended = match self.suspended.as_ref() {
            Some(s) => s,
            None => return Ok(None),
        };

        let conversation_state = self.conversation.clone().ok_or_else(|| AppError {
            code: ErrorCode::SchemaValidationError,
            message: "runtime snapshot missing suspended conversation state".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_INVALID",
                "session_id": self.session_id,
                "field": "conversation",
            })),
            recoverable: Some(true),
        })?;

        let loop_config = self.loop_config.clone().ok_or_else(|| AppError {
            code: ErrorCode::SchemaValidationError,
            message: "runtime snapshot missing suspended loop config".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_INVALID",
                "session_id": self.session_id,
                "field": "loop_config",
            })),
            recoverable: Some(true),
        })?;

        let provider_name = self.provider_name.clone().unwrap_or_default();
        let model = self.model.clone().unwrap_or_default();
        let base_url = base_url_override
            .or_else(|| self.base_url.clone())
            .unwrap_or_default();

        let suspend_reason = match suspended.kind {
            SuspendedKind::Confirmation => StopReason::WaitingConfirmation,
            SuspendedKind::Askuser => StopReason::WaitingAskuser,
        };

        Ok(Some(SuspendedTurnState {
            conversation_state,
            pending_tool_call: suspended.pending_tool_call.clone(),
            pending_call_id: suspended.pending_call_id.clone(),
            remaining_tool_calls: suspended.remaining_tool_calls.clone(),
            completed_messages: suspended.completed_messages.clone(),
            loop_config,
            project_path,
            provider_name,
            model,
            base_url,
            api_key,
            active_chapter_path: self.active_chapter_path.clone(),
            active_skill: self.active_skill.clone(),
            system_prompt: self.system_prompt.clone(),
            suspend_reason,
            rounds_executed: suspended.rounds_executed,
            total_tool_calls: suspended.total_tool_calls,
        }))
    }

    pub fn from_upsert(input: RuntimeSnapshotUpsertInput) -> Self {
        let suspended_snapshot =
            input
                .suspended
                .as_ref()
                .map(|state| AgentSessionSuspendedSnapshot {
                    kind: match state.suspend_reason {
                        StopReason::WaitingConfirmation => SuspendedKind::Confirmation,
                        _ => SuspendedKind::Askuser,
                    },
                    pending_call_id: state.pending_call_id.clone(),
                    pending_tool_call: state.pending_tool_call.clone(),
                    remaining_tool_calls: state.remaining_tool_calls.clone(),
                    completed_messages: state.completed_messages.clone(),
                    rounds_executed: state.rounds_executed,
                    total_tool_calls: state.total_tool_calls,
                });

        let suspended_active_skill = input
            .suspended
            .as_ref()
            .and_then(|state| normalize_active_skill(state.active_skill.clone()));
        let active_skill = normalize_active_skill(input.active_skill).or(suspended_active_skill);

        let fallback_conversation = input
            .suspended
            .as_ref()
            .map(|state| state.conversation_state.clone());

        let conversation = input.conversation.or(fallback_conversation);
        let next_turn_id = crate::agent_engine::session_state::derive_next_turn_id(
            input.last_turn,
            conversation.as_ref(),
            input.next_turn_id,
        );

        Self {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
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
            active_chapter_path: input.active_chapter_path,
            active_skill,
            loop_config: input.loop_config,
            conversation,
            suspended: suspended_snapshot,
            readonly_reason: input.readonly_reason,
            session_revision: input.session_revision,
        }
    }
}

pub fn runtime_snapshot_path(project_path: &Path, session_id: &str) -> PathBuf {
    sessions_root(project_path).join(format!("{session_id}{RUNTIME_SUFFIX}"))
}

pub fn load_runtime_snapshot(
    project_path: &Path,
    session_id: &str,
) -> Result<Option<AgentSessionRuntimeSnapshotV1>, AppError> {
    let path = runtime_snapshot_path(project_path, session_id);
    if !path.exists() {
        return Ok(None);
    }

    let snapshot: AgentSessionRuntimeSnapshotV1 = read_json(&path).map_err(|err| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("failed to parse runtime snapshot: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_PARSE_FAILED",
            "path": path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    let original_next_turn_id = snapshot.next_turn_id;
    let migrated = migrate_runtime_snapshot(snapshot)?;

    if let Some(ref snapshot) = migrated {
        if snapshot.next_turn_id != original_next_turn_id {
            save_runtime_snapshot(project_path, session_id, snapshot)?;
        }
    }

    Ok(migrated)
}

pub fn migrate_runtime_snapshot(
    mut snapshot: AgentSessionRuntimeSnapshotV1,
) -> Result<Option<AgentSessionRuntimeSnapshotV1>, AppError> {
    if snapshot.schema_version == RUNTIME_SNAPSHOT_SCHEMA_VERSION {
        snapshot.next_turn_id = Some(crate::agent_engine::session_state::derive_next_turn_id(
            snapshot.last_turn,
            snapshot.conversation.as_ref(),
            snapshot.next_turn_id,
        ));
        return Ok(Some(snapshot));
    }

    Err(AppError {
        code: ErrorCode::SchemaVersionUnsupported,
        message: format!(
            "unsupported runtime snapshot schema version: {}",
            snapshot.schema_version
        ),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_SCHEMA_UNSUPPORTED",
            "schema_version": snapshot.schema_version,
            "supported": RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            "session_id": snapshot.session_id,
        })),
        recoverable: Some(true),
    })
}

pub fn save_runtime_snapshot(
    project_path: &Path,
    session_id: &str,
    snapshot: &AgentSessionRuntimeSnapshotV1,
) -> Result<(), AppError> {
    let root = sessions_root(project_path);
    ensure_dir(&root)?;
    let path = runtime_snapshot_path(project_path, session_id);

    atomic_write_json(&path, snapshot).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write runtime snapshot: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_WRITE_FAILED",
            "path": path.to_string_lossy(),
            "session_id": session_id,
        })),
        recoverable: Some(true),
    })
}

pub fn save_runtime_snapshot_from_input(
    project_path: &Path,
    input: RuntimeSnapshotUpsertInput,
) -> Result<AgentSessionRuntimeSnapshotV1, AppError> {
    let snapshot = AgentSessionRuntimeSnapshotV1::from_upsert(input);
    save_runtime_snapshot(project_path, &snapshot.session_id, &snapshot)?;
    Ok(snapshot)
}

pub fn delete_runtime_snapshot(project_path: &Path, session_id: &str) -> Result<(), AppError> {
    let path = runtime_snapshot_path(project_path, session_id);
    if !path.exists() {
        return Ok(());
    }

    std::fs::remove_file(&path).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to delete runtime snapshot: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RUNTIME_SNAPSHOT_DELETE_FAILED",
            "path": path.to_string_lossy(),
            "session_id": session_id,
        })),
        recoverable: Some(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::messages::AgentMessage;

    fn setup_temp_project() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "magic_runtime_snapshot_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(base.join("magic_novel").join("ai").join("sessions")).unwrap();
        base
    }

    #[test]
    fn runtime_snapshot_save_and_load_roundtrip() {
        let project = setup_temp_project();

        let mut conversation = ConversationState::new("s1".to_string());
        conversation
            .messages
            .push(AgentMessage::user("hello".to_string()));

        let saved = save_runtime_snapshot_from_input(
            &project,
            RuntimeSnapshotUpsertInput::from_conversation(
                "s1".to_string(),
                SessionRuntimeState::Completed,
                conversation.clone(),
                Some(3),
                Some("openai".to_string()),
                Some("gpt-4o-mini".to_string()),
                Some("https://example.com/v1".to_string()),
                Some("system".to_string()),
                Some("book/ch1.md".to_string()),
                Some(LoopConfig::default()),
            )
            .with_active_skill(Some("story-architect".to_string())),
        )
        .expect("save should succeed");

        assert_eq!(saved.schema_version, RUNTIME_SNAPSHOT_SCHEMA_VERSION);

        let loaded = load_runtime_snapshot(&project, "s1")
            .expect("load should succeed")
            .expect("snapshot should exist");

        assert_eq!(loaded.session_id, "s1");
        assert_eq!(loaded.runtime_state, SessionRuntimeState::Completed);
        assert_eq!(loaded.last_turn, Some(3));
        assert_eq!(loaded.next_turn_id, Some(4));
        assert_eq!(loaded.provider_name.as_deref(), Some("openai"));
        assert_eq!(loaded.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(loaded.base_url.as_deref(), Some("https://example.com/v1"));
        assert_eq!(loaded.system_prompt.as_deref(), Some("system"));
        assert_eq!(loaded.active_chapter_path.as_deref(), Some("book/ch1.md"));
        assert_eq!(loaded.active_skill.as_deref(), Some("story-architect"));
        assert_eq!(
            loaded.conversation.as_ref().map(|s| s.messages.len()),
            Some(conversation.messages.len())
        );
    }

    #[test]
    fn runtime_snapshot_load_migrates_missing_next_turn_id() {
        let project = setup_temp_project();
        let session_id = "legacy_snapshot";

        let mut conversation = ConversationState::new(session_id.to_string());
        conversation.current_turn = 7;
        conversation
            .messages
            .push(AgentMessage::user("hello legacy".to_string()));

        let legacy = AgentSessionRuntimeSnapshotV1 {
            schema_version: RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            runtime_state: SessionRuntimeState::Completed,
            hydration_source: Some("snapshot_loaded".to_string()),
            last_turn: Some(7),
            next_turn_id: None,
            provider_name: Some("openai".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            base_url: Some("https://example.com/v1".to_string()),
            system_prompt: None,
            active_chapter_path: None,
            active_skill: None,
            loop_config: Some(LoopConfig::default()),
            conversation: Some(conversation),
            suspended: None,
            readonly_reason: None,
            session_revision: None,
        };

        save_runtime_snapshot(project.as_path(), session_id, &legacy)
            .expect("save legacy snapshot");

        let loaded = load_runtime_snapshot(project.as_path(), session_id)
            .expect("load should succeed")
            .expect("snapshot should exist");

        assert_eq!(loaded.next_turn_id, Some(8));
    }
}
