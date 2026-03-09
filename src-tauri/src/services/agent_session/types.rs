use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::paths::AGENT_SESSION_SCHEMA_VERSION;

/// Well-known session event type constants for agent_session JSONL persistence.
/// These map UI runtime events to persisted session events.
pub mod session_event_types {
    pub const SESSION_START: &str = "session_start";
    pub const SESSION_REMINDER_INJECTED: &str = "session_reminder_injected";
    pub const TURN_STARTED: &str = "turn_started";
    pub const MESSAGE: &str = "message";
    pub const TOOL_EXECUTION: &str = "tool_execution";
    pub const TOOL_RESULT: &str = "tool_result";
    pub const TURN_STATE: &str = "turn_state";
    pub const COMPACTION_STARTED: &str = "compaction_started";
    pub const COMPACTION_SUMMARY: &str = "compaction_summary";
    pub const COMPACTION_FINISHED: &str = "compaction_finished";
    pub const COMPACTION_FALLBACK: &str = "compaction_fallback";
    pub const TURN_COMPLETED: &str = "turn_completed";
    pub const TURN_FAILED: &str = "turn_failed";
    pub const TURN_CANCELLED: &str = "turn_cancelled";
    pub const TOKEN_USAGE: &str = "token_usage";
    pub const TIMELINE_EVENT: &str = "timeline_event";
    pub const SESSION_SETTINGS_UPDATED: &str = "session_settings_updated";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionEvent {
    pub schema_version: i32,
    #[serde(rename = "type")]
    pub event_type: String,
    pub session_id: String,
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_seq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

impl AgentSessionEvent {
    pub fn validate_v1(&self) -> bool {
        self.schema_version == AGENT_SESSION_SCHEMA_VERSION
            && !self.event_type.trim().is_empty()
            && !self.session_id.trim().is_empty()
            && self.ts > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionMeta {
    pub schema_version: i32,
    pub session_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_chapter_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionSettings {
    pub schema_version: i32,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl Default for AgentSessionSettings {
    fn default() -> Self {
        Self {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            session_id: String::new(),
            model: None,
            provider: None,
            token_budget: None,
            metadata: None,
        }
    }
}
