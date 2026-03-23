//! Agent Engine - Event protocol (Rust -> UI)
//!
//! Aligned with docs/magic_plan/plan_agent/07-agent-event-protocol.md

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: i32 = 1;
pub const AGENT_EVENT_CHANNEL: &str = "magic:agent_event";

/// Envelope wrapping every event sent to the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub schema_version: i32,
    pub event_id: String,
    pub ts: i64,
    pub session_id: String,
    pub turn_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<String>,
    pub source: EventSource,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
}

impl EventSource {
    pub fn agent() -> Self {
        Self {
            kind: "agent".to_string(),
            worker_id: None,
            mission_id: None,
        }
    }
}

impl EventEnvelope {
    pub fn new(
        session_id: &str,
        turn_id: u32,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self::new_with_client_request_id(session_id, turn_id, event_type, payload, None)
    }

    pub fn new_with_client_request_id(
        session_id: &str,
        turn_id: u32,
        event_type: &str,
        payload: serde_json::Value,
        client_request_id: Option<&str>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            event_id: format!("evt_{}", uuid::Uuid::new_v4()),
            ts: chrono::Utc::now().timestamp_millis(),
            session_id: session_id.to_string(),
            turn_id,
            client_request_id: client_request_id.map(|value| value.to_string()),
            source: EventSource::agent(),
            event_type: event_type.to_string(),
            payload,
        }
    }
}

// ── Event type constants ─────────────────────────────────────────

pub mod event_types {
    // Turn lifecycle
    pub const TURN_STARTED: &str = "TURN_STARTED";
    pub const PLAN_STARTED: &str = "PLAN_STARTED";
    pub const TURN_COMPLETED: &str = "TURN_COMPLETED";
    pub const TURN_FAILED: &str = "TURN_FAILED";
    pub const TURN_CANCELLED: &str = "TURN_CANCELLED";

    // Worker identity (UI-P3 phase timeline)
    pub const WORKER_STARTED: &str = "WORKER_STARTED";
    pub const WORKER_COMPLETED: &str = "WORKER_COMPLETED";

    // Streaming
    pub const STREAMING_STARTED: &str = "STREAMING_STARTED";
    pub const ASSISTANT_TEXT_DELTA: &str = "ASSISTANT_TEXT_DELTA";
    pub const THINKING_TEXT_DELTA: &str = "THINKING_TEXT_DELTA";
    pub const USAGE_UPDATE: &str = "USAGE_UPDATE";

    // Tool
    pub const TOOL_CALL_STARTED: &str = "TOOL_CALL_STARTED";
    pub const TOOL_CALL_PROGRESS: &str = "TOOL_CALL_PROGRESS";
    pub const TOOL_CALL_FINISHED: &str = "TOOL_CALL_FINISHED";
    pub const WAITING_FOR_CONFIRMATION: &str = "WAITING_FOR_CONFIRMATION";

    // AskUser
    pub const ASKUSER_REQUESTED: &str = "ASKUSER_REQUESTED";
    pub const ASKUSER_ANSWERED: &str = "ASKUSER_ANSWERED";

    // Compaction
    pub const COMPACTION_STARTED: &str = "COMPACTION_STARTED";
    pub const COMPACTION_FINISHED: &str = "COMPACTION_FINISHED";
    pub const COMPACTION_FALLBACK: &str = "COMPACTION_FALLBACK";

    // Review (P1)
    pub const REVIEW_RECORDED: &str = "REVIEW_RECORDED";
}
