use serde::{Deserialize, Serialize};

pub use crate::services::agent_session::session_event_types;
pub type InspirationSessionEvent = crate::services::agent_session::AgentSessionEvent;

use super::paths::INSPIRATION_SESSION_SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionMeta {
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
    pub compaction_count: Option<i64>,
}

impl InspirationSessionMeta {
    pub fn new(session_id: String, now: i64, title: Option<String>) -> Self {
        Self {
            schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
            session_id,
            created_at: now,
            updated_at: now,
            title,
            last_turn: None,
            last_stop_reason: None,
            compaction_count: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationSessionIndex {
    pub schema_version: i32,
    pub sessions: Vec<InspirationSessionMeta>,
}

impl Default for InspirationSessionIndex {
    fn default() -> Self {
        Self {
            schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
            sessions: Vec::new(),
        }
    }
}
