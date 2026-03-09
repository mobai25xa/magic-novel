//! Mission system - Worker NDJSON protocol types
//!
//! Defines the stdio protocol between Orchestrator and Worker processes.
//! Transport: NDJSON (one JSON object per line, terminated by \n).
//!
//! Based on docs/magic_plan/plan_agent/13-mission-worker-protocol.md

use serde::{Deserialize, Serialize};

use super::types::{Feature, HandoffEntry};

pub const PROTOCOL_SCHEMA_VERSION: i32 = 1;

// ── Orchestrator → Worker (stdin instructions) ──────────────────

/// Instructions sent from Orchestrator to Worker via stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInstruction {
    pub schema_version: i32,
    pub id: String,
    #[serde(rename = "type")]
    pub instruction_type: InstructionType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstructionType {
    Initialize,
    StartFeature,
    Cancel,
    Shutdown,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializePayload {
    pub worker_id: String,
    pub project_path: String,
    pub mission_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartFeaturePayload {
    pub feature: Feature,
    pub session_id: String,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<u32>,
}

// ── Worker → Orchestrator (stdout events) ───────────────────────

/// Events sent from Worker to Orchestrator via stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub schema_version: i32,
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: WorkerEventType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerEventType {
    Ack,
    AgentEvent,
    FeatureCompleted,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPayload {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCompletedPayload {
    pub feature_id: String,
    pub ok: bool,
    pub handoff: HandoffEntry,
}

// ── Builder helpers ─────────────────────────────────────────────

impl WorkerInstruction {
    /// Serialize to a single NDJSON line (without trailing newline).
    pub fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse a single NDJSON line.
    pub fn from_ndjson_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    pub fn initialize(id: &str, payload: InitializePayload) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: id.to_string(),
            instruction_type: InstructionType::Initialize,
            payload: serde_json::to_value(payload).unwrap_or_default(),
        }
    }

    pub fn start_feature(id: &str, payload: StartFeaturePayload) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: id.to_string(),
            instruction_type: InstructionType::StartFeature,
            payload: serde_json::to_value(payload).unwrap_or_default(),
        }
    }

    pub fn cancel(id: &str, turn_id: Option<u32>) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: id.to_string(),
            instruction_type: InstructionType::Cancel,
            payload: serde_json::to_value(CancelPayload { turn_id }).unwrap_or_default(),
        }
    }

    pub fn shutdown(id: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: id.to_string(),
            instruction_type: InstructionType::Shutdown,
            payload: serde_json::json!({}),
        }
    }

    pub fn ping(id: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: id.to_string(),
            instruction_type: InstructionType::Ping,
            payload: serde_json::json!({}),
        }
    }
}

impl WorkerEvent {
    /// Serialize to a single NDJSON line (without trailing newline).
    pub fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse a single NDJSON line.
    pub fn from_ndjson_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    pub fn ack(request_id: &str, ok: bool, error: Option<String>) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: format!("res_{request_id}"),
            event_type: WorkerEventType::Ack,
            payload: serde_json::to_value(AckPayload { ok, error }).unwrap_or_default(),
        }
    }

    pub fn agent_event(payload: serde_json::Value) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: format!("wevt_{}", uuid::Uuid::new_v4()),
            event_type: WorkerEventType::AgentEvent,
            payload,
        }
    }

    pub fn feature_completed(feature_id: &str, ok: bool, handoff: HandoffEntry) -> Self {
        let completed = FeatureCompletedPayload {
            feature_id: feature_id.to_string(),
            ok,
            handoff,
        };
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: format!("wevt_{}", uuid::Uuid::new_v4()),
            event_type: WorkerEventType::FeatureCompleted,
            payload: serde_json::to_value(completed).unwrap_or_default(),
        }
    }

    pub fn pong(request_id: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            id: format!("res_{request_id}"),
            event_type: WorkerEventType::Pong,
            payload: serde_json::json!({}),
        }
    }
}

/// Generate a unique request ID for instructions.
pub fn new_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4())
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::types::FeatureStatus;

    #[test]
    fn test_instruction_initialize_roundtrip() {
        let instr = WorkerInstruction::initialize(
            "req_1",
            InitializePayload {
                worker_id: "wk_abc".to_string(),
                project_path: "/tmp/project".to_string(),
                mission_dir: "/tmp/project/magic_novel/missions/mis_1".to_string(),
            },
        );

        let line = instr.to_ndjson_line().unwrap();
        let parsed = WorkerInstruction::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.instruction_type, InstructionType::Initialize);
        assert_eq!(parsed.id, "req_1");

        let payload: InitializePayload = serde_json::from_value(parsed.payload).unwrap();
        assert_eq!(payload.worker_id, "wk_abc");
    }

    #[test]
    fn test_instruction_start_feature_roundtrip() {
        let feature = Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "Write chapter".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: vec!["chapters/ch1.md".to_string()],
        };

        let instr = WorkerInstruction::start_feature(
            "req_2",
            StartFeaturePayload {
                feature,
                session_id: "chat_123".to_string(),
                model: "gpt-4".to_string(),
                provider: "openai".to_string(),
                base_url: "https://api.openai.com".to_string(),
                api_key: "sk-test".to_string(),
            },
        );

        let line = instr.to_ndjson_line().unwrap();
        assert!(!line.contains('\n'));

        let parsed = WorkerInstruction::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.instruction_type, InstructionType::StartFeature);
    }

    #[test]
    fn test_instruction_cancel_shutdown_ping() {
        let cancel = WorkerInstruction::cancel("req_3", Some(5));
        let line = cancel.to_ndjson_line().unwrap();
        let parsed = WorkerInstruction::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.instruction_type, InstructionType::Cancel);

        let shutdown = WorkerInstruction::shutdown("req_4");
        let line = shutdown.to_ndjson_line().unwrap();
        let parsed = WorkerInstruction::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.instruction_type, InstructionType::Shutdown);

        let ping = WorkerInstruction::ping("req_5");
        let line = ping.to_ndjson_line().unwrap();
        let parsed = WorkerInstruction::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.instruction_type, InstructionType::Ping);
    }

    #[test]
    fn test_event_ack_roundtrip() {
        let evt = WorkerEvent::ack("req_1", true, None);
        let line = evt.to_ndjson_line().unwrap();
        let parsed = WorkerEvent::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.event_type, WorkerEventType::Ack);
        assert_eq!(parsed.id, "res_req_1");

        let payload: AckPayload = serde_json::from_value(parsed.payload).unwrap();
        assert!(payload.ok);
        assert!(payload.error.is_none());
    }

    #[test]
    fn test_event_ack_error() {
        let evt = WorkerEvent::ack("req_2", false, Some("not initialized".to_string()));
        let line = evt.to_ndjson_line().unwrap();
        let parsed = WorkerEvent::from_ndjson_line(&line).unwrap();

        let payload: AckPayload = serde_json::from_value(parsed.payload).unwrap();
        assert!(!payload.ok);
        assert_eq!(payload.error.unwrap(), "not initialized");
    }

    #[test]
    fn test_event_feature_completed_roundtrip() {
        use crate::mission::types::HandoffEntry;

        let handoff = HandoffEntry {
            feature_id: "f1".to_string(),
            worker_id: "wk_1".to_string(),
            ok: true,
            summary: "done".to_string(),
            commands_run: vec!["read".to_string()],
            artifacts: Vec::new(),
            issues: Vec::new(),
        };

        let evt = WorkerEvent::feature_completed("f1", true, handoff);
        let line = evt.to_ndjson_line().unwrap();
        let parsed = WorkerEvent::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.event_type, WorkerEventType::FeatureCompleted);

        let payload: FeatureCompletedPayload = serde_json::from_value(parsed.payload).unwrap();
        assert_eq!(payload.feature_id, "f1");
        assert!(payload.ok);
        assert_eq!(payload.handoff.summary, "done");
    }

    #[test]
    fn test_event_pong_roundtrip() {
        let evt = WorkerEvent::pong("req_5");
        let line = evt.to_ndjson_line().unwrap();
        let parsed = WorkerEvent::from_ndjson_line(&line).unwrap();
        assert_eq!(parsed.event_type, WorkerEventType::Pong);
        assert_eq!(parsed.id, "res_req_5");
    }

    #[test]
    fn test_invalid_ndjson_line() {
        let result = WorkerEvent::from_ndjson_line("not json at all");
        assert!(result.is_err());

        let result = WorkerInstruction::from_ndjson_line("{\"broken");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_newline_in_ndjson() {
        let instr = WorkerInstruction::ping("req_1");
        let line = instr.to_ndjson_line().unwrap();
        assert!(
            !line.contains('\n'),
            "NDJSON line must not contain newlines"
        );

        let evt = WorkerEvent::pong("req_1");
        let line = evt.to_ndjson_line().unwrap();
        assert!(
            !line.contains('\n'),
            "NDJSON line must not contain newlines"
        );
    }
}
