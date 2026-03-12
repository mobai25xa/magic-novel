//! Agent Engine - Session persistence sink
//!
//! Bridges agent_engine runtime events to agent_session JSONL persistence.
//!
//! Policy (per guide.md contract):
//! - Streaming deltas (ASSISTANT_TEXT_DELTA, THINKING_TEXT_DELTA, STREAMING_STARTED) are NOT persisted.
//! - Turn lifecycle, tool results, turn_state, compaction events ARE persisted.
//! - After a turn completes, the full assistant message is persisted (not deltas).

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::models::AppError;
#[cfg(test)]
use crate::services::agent_session::session_stream_path;
use crate::services::agent_session::types::session_event_types;
use crate::services::agent_session::{
    append_session_events, AgentSessionEvent, AGENT_SESSION_SCHEMA_VERSION,
};

use super::events::event_types;
use super::events::EventEnvelope;
use super::messages::AgentMessage;

/// Writes key agent events to the session JSONL stream.
///
/// Intended to be called alongside the UI emitter — the emitter sends events
/// to the Tauri event bus for live UI updates, while this sink persists them
/// for later replay/recovery.
pub struct SessionPersistenceSink {
    project_path: PathBuf,
    session_id: String,
    client_request_id: Option<String>,
    hydration_source: Option<String>,
}

impl SessionPersistenceSink {
    pub fn new(project_path: impl Into<PathBuf>, session_id: impl Into<String>) -> Self {
        Self {
            project_path: project_path.into(),
            session_id: session_id.into(),
            client_request_id: None,
            hydration_source: None,
        }
    }

    pub fn with_client_request_id(mut self, client_request_id: Option<String>) -> Self {
        self.client_request_id = client_request_id;
        self
    }

    pub fn with_hydration_source(mut self, hydration_source: Option<String>) -> Self {
        self.hydration_source = hydration_source;
        self
    }

    #[cfg(test)]
    fn stream_path(&self) -> PathBuf {
        session_stream_path(&self.project_path, &self.session_id)
    }

    fn build_event(
        &self,
        event_type: &str,
        turn: Option<i64>,
        payload: Option<Value>,
    ) -> AgentSessionEvent {
        let dedupe_key = build_runtime_dedupe_key(event_type, turn, payload.as_ref());

        AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: event_type.to_string(),
            session_id: self.session_id.clone(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: None,
            dedupe_key,
            turn,
            payload,
        }
    }

    fn append(&self, events: &[AgentSessionEvent]) -> Result<(), AppError> {
        let append_result = append_session_events(&self.project_path, &self.session_id, events)?;
        if append_result.deduped_count > 0 {
            tracing::debug!(
                target: "agent_engine",
                session_id = %self.session_id,
                deduped_count = append_result.deduped_count,
                appended_count = append_result.appended_count,
                last_event_seq = append_result.last_event_seq,
                "session persistence dropped duplicated dedupe_key events"
            );
        }
        Ok(())
    }

    /// Persist a runtime event envelope to the session JSONL.
    ///
    /// Returns `Ok(true)` if the event was persisted, `Ok(false)` if it was
    /// intentionally skipped (e.g. streaming deltas).
    pub fn persist_event(&self, envelope: &EventEnvelope) -> Result<bool, AppError> {
        let (session_type, payload) = match map_event(envelope, self.hydration_source.as_deref()) {
            Some(mapped) => mapped,
            None => return Ok(false),
        };

        let event = self.build_event(session_type, Some(envelope.turn_id as i64), Some(payload));

        self.append(&[event])?;
        Ok(true)
    }

    /// Persist a complete assistant message after a turn finishes.
    ///
    /// This is the primary persistence point for assistant content — streaming
    /// deltas are intentionally not persisted to avoid write amplification.
    pub fn persist_assistant_message(&self, msg: &AgentMessage, turn: u32) -> Result<(), AppError> {
        let text = msg.text_content();
        let tool_calls: Vec<Value> = msg
            .tool_calls()
            .iter()
            .map(|tc| {
                json!({
                    "llm_call_id": tc.llm_call_id,
                    "tool_name": tc.tool_name,
                })
            })
            .collect();

        let payload = json!({
            "role": "assistant",
            "content": text,
            "message_id": msg.id,
            "tool_calls": tool_calls,
        });
        let payload = append_event_diagnostics(
            payload,
            turn,
            self.client_request_id.as_deref(),
            self.hydration_source.as_deref(),
        );

        let event = self.build_event(
            session_event_types::MESSAGE,
            Some(turn as i64),
            Some(payload),
        );

        self.append(&[event])
    }

    /// Persist a user message.
    pub fn persist_user_message(&self, text: &str, turn: u32) -> Result<(), AppError> {
        let payload = json!({
            "role": "user",
            "content": text,
            "message_id": format!("msg_{}", uuid::Uuid::new_v4()),
        });
        let payload = append_event_diagnostics(
            payload,
            turn,
            self.client_request_id.as_deref(),
            self.hydration_source.as_deref(),
        );

        let event = self.build_event(
            session_event_types::MESSAGE,
            Some(turn as i64),
            Some(payload),
        );

        self.append(&[event])
    }

    /// Persist a turn state change (pause/resume).
    pub fn persist_turn_state(&self, turn: u32, state: &str, extra: Value) -> Result<(), AppError> {
        let mut payload = json!({ "state": state });
        if let Value::Object(map) = extra {
            if let Value::Object(ref mut target) = payload {
                target.extend(map);
            }
        }
        let payload = append_event_diagnostics(
            payload,
            turn,
            self.client_request_id.as_deref(),
            self.hydration_source.as_deref(),
        );

        let event = self.build_event(
            session_event_types::TURN_STATE,
            Some(turn as i64),
            Some(payload),
        );

        self.append(&[event])
    }
}

/// Map an EventEnvelope to a session event type + payload.
/// Returns `None` for events that should not be persisted (streaming deltas).
fn map_event(
    envelope: &EventEnvelope,
    hydration_source: Option<&str>,
) -> Option<(&'static str, Value)> {
    let mapped = match envelope.event_type.as_str() {
        // ── Skip streaming deltas ──────────────────────────────
        event_types::ASSISTANT_TEXT_DELTA
        | event_types::THINKING_TEXT_DELTA
        | event_types::STREAMING_STARTED => None,

        // ── Turn lifecycle ─────────────────────────────────────
        event_types::TURN_STARTED => {
            Some((session_event_types::TURN_STARTED, envelope.payload.clone()))
        }
        event_types::PLAN_STARTED => Some((
            session_event_types::TURN_STARTED,
            json!({
                "timeline_type": event_types::PLAN_STARTED,
                "tool_package": envelope.payload.get("tool_package"),
                "route_reason": envelope.payload.get("route_reason"),
                "fallback_from": envelope.payload.get("fallback_from"),
                "fallback_reason": envelope.payload.get("fallback_reason"),
                "rollout_mode": envelope.payload.get("rollout_mode"),
                "rollout_in_canary": envelope.payload.get("rollout_in_canary"),
                "canary_percent": envelope.payload.get("canary_percent"),
                "exposed_tools": envelope.payload.get("exposed_tools"),
                "skipped_tools": envelope.payload.get("skipped_tools"),
            }),
        )),
        event_types::TURN_COMPLETED => Some((
            session_event_types::TURN_COMPLETED,
            envelope.payload.clone(),
        )),
        event_types::TURN_FAILED => {
            Some((session_event_types::TURN_FAILED, envelope.payload.clone()))
        }
        event_types::TURN_CANCELLED => Some((
            session_event_types::TURN_CANCELLED,
            envelope.payload.clone(),
        )),

        // ── Tool events ────────────────────────────────────────
        event_types::TOOL_CALL_STARTED => Some((
            session_event_types::TOOL_EXECUTION,
            envelope.payload.clone(),
        )),
        event_types::TOOL_CALL_FINISHED => {
            Some((session_event_types::TOOL_RESULT, envelope.payload.clone()))
        }

        // ── Review (P1) ───────────────────────────────────────
        event_types::REVIEW_RECORDED => Some((
            session_event_types::TIMELINE_EVENT,
            json!({
                "timeline_type": event_types::REVIEW_RECORDED,
                // Keep call_id at the top-level so runtime dedupe_key includes it.
                // This prevents multiple reviews within the same turn from being deduped.
                "call_id": envelope.payload.get("call_id"),
                "llm_call_id": envelope.payload.get("llm_call_id"),
                "tool_name": envelope.payload.get("tool_name"),
                "target_ref": envelope.payload.get("target_ref"),
                "overall_status": envelope.payload.get("overall_status"),
                "issue_counts": envelope.payload.get("issue_counts"),
                "review": envelope.payload.clone(),
            }),
        )),

        // ── Confirmation / AskUser → turn_state ────────────────
        event_types::WAITING_FOR_CONFIRMATION => Some((
            session_event_types::TURN_STATE,
            json!({
                "state": "waiting_confirmation",
                "call_id": envelope.payload.get("call_id"),
                "tool_name": envelope.payload.get("tool_name"),
                "reason": envelope.payload.get("reason"),
            }),
        )),
        event_types::ASKUSER_REQUESTED => Some((
            session_event_types::TURN_STATE,
            json!({
                "state": "waiting_askuser",
                "call_id": envelope.payload.get("call_id"),
                "tool_name": envelope.payload.get("tool_name"),
                "llm_call_id": envelope.payload.get("llm_call_id"),
                "questions": envelope.payload.get("questions"),
                "questionnaire": envelope.payload.get("questionnaire"),
            }),
        )),

        // ── Compaction ─────────────────────────────────────────
        event_types::COMPACTION_STARTED => Some((
            session_event_types::COMPACTION_STARTED,
            envelope.payload.clone(),
        )),
        event_types::COMPACTION_FINISHED => Some((
            session_event_types::COMPACTION_FINISHED,
            envelope.payload.clone(),
        )),
        event_types::COMPACTION_FALLBACK => Some((
            session_event_types::COMPACTION_FALLBACK,
            envelope.payload.clone(),
        )),

        // ── Usage ──────────────────────────────────────────────
        event_types::USAGE_UPDATE => {
            Some((session_event_types::TOKEN_USAGE, envelope.payload.clone()))
        }

        // ── AskUser answered / resumed state ───────────────────
        event_types::ASKUSER_ANSWERED => Some((
            session_event_types::TURN_STATE,
            json!({
                "state": "resumed",
                "call_id": envelope.payload.get("call_id"),
                "llm_call_id": envelope.payload.get("llm_call_id"),
                "answers": envelope.payload.get("answers"),
            }),
        )),

        // ── Unknown / TOOL_CALL_PROGRESS ────────────────────────
        // TOOL_CALL_PROGRESS is intermediate, not persisted.
        _ => None,
    };

    mapped.map(|(session_type, payload)| {
        (
            session_type,
            append_event_diagnostics(
                payload,
                envelope.turn_id,
                envelope.client_request_id.as_deref(),
                hydration_source,
            ),
        )
    })
}

fn append_event_diagnostics(
    mut payload: Value,
    turn_id: u32,
    client_request_id: Option<&str>,
    hydration_source: Option<&str>,
) -> Value {
    let Value::Object(ref mut map) = payload else {
        return payload;
    };

    map.entry("bound_turn_id".to_string())
        .or_insert_with(|| json!(turn_id));

    if let Some(client_request_id) = client_request_id.filter(|value| !value.trim().is_empty()) {
        map.entry("client_request_id".to_string())
            .or_insert_with(|| Value::String(client_request_id.to_string()));
    }

    if let Some(hydration_source) = hydration_source.filter(|value| !value.trim().is_empty()) {
        map.entry("hydrate_source".to_string())
            .or_insert_with(|| Value::String(hydration_source.to_string()));
    }

    payload
}

fn build_runtime_dedupe_key(
    event_type: &str,
    turn: Option<i64>,
    payload: Option<&Value>,
) -> Option<String> {
    let event_type = event_type.trim();
    if event_type.is_empty() {
        return None;
    }

    let mut parts = vec![event_type.to_string()];

    if let Some(turn) = turn {
        parts.push(format!("turn:{turn}"));
    }

    if let Some(state) = payload_string(payload, "state") {
        parts.push(format!("state:{state}"));
    }

    if let Some(call_id) = payload_string(payload, "call_id")
        .or_else(|| payload_string(payload, "pending_call_id"))
        .or_else(|| payload_string(payload, "llm_call_id"))
    {
        parts.push(format!("call:{call_id}"));
    }

    if let Some(message_id) = payload_string(payload, "message_id") {
        parts.push(format!("message:{message_id}"));
    }

    if let Some(stop_reason) = payload_string(payload, "stop_reason") {
        parts.push(format!("stop:{stop_reason}"));
    }

    if event_type == session_event_types::TOKEN_USAGE {
        if let Some(input_tokens) = payload_u64(payload, "input_tokens") {
            parts.push(format!("in:{input_tokens}"));
        }
        if let Some(output_tokens) = payload_u64(payload, "output_tokens") {
            parts.push(format!("out:{output_tokens}"));
        }
        if let Some(cache_read_tokens) = payload_u64(payload, "cache_read_tokens") {
            parts.push(format!("cache:{cache_read_tokens}"));
        }
        if let Some(thinking_tokens) = payload_u64(payload, "thinking_tokens") {
            parts.push(format!("think:{thinking_tokens}"));
        }
    }

    if parts.len() == 1 {
        return None;
    }

    Some(parts.join(":"))
}

fn payload_string(payload: Option<&Value>, field: &str) -> Option<String> {
    payload
        .and_then(|value| value.get(field))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn payload_u64(payload: Option<&Value>, field: &str) -> Option<u64> {
    payload
        .and_then(|value| value.get(field))
        .and_then(|value| value.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp_project() -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("magic_persist_test_{}", uuid::Uuid::new_v4()));
        let sessions_dir = base.join("magic_novel").join("ai").join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        base
    }

    fn make_envelope(event_type: &str, turn_id: u32, payload: Value) -> EventEnvelope {
        EventEnvelope::new("test_session", turn_id, event_type, payload)
    }

    #[test]
    fn test_skip_streaming_deltas() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::ASSISTANT_TEXT_DELTA,
            1,
            json!({"delta": "hello"}),
        );
        let persisted = sink.persist_event(&envelope).unwrap();
        assert!(!persisted);

        let envelope = make_envelope(
            event_types::THINKING_TEXT_DELTA,
            1,
            json!({"delta": "thinking"}),
        );
        let persisted = sink.persist_event(&envelope).unwrap();
        assert!(!persisted);

        let envelope = make_envelope(event_types::STREAMING_STARTED, 1, json!({}));
        let persisted = sink.persist_event(&envelope).unwrap();
        assert!(!persisted);

        // JSONL file should not exist (nothing written)
        let path = sink.stream_path();
        assert!(!path.exists());
    }

    #[test]
    fn test_persist_turn_lifecycle() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::TURN_STARTED,
            1,
            json!({"user_text_preview": "hi", "model_provider": "openai", "model": "gpt-4"}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let envelope = make_envelope(
            event_types::TURN_COMPLETED,
            1,
            json!({"stop_reason": "success", "latency_ms": 500}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "turn_started");
        assert_eq!(events[1].event_type, "turn_completed");
    }

    #[test]
    fn test_persist_tool_events() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::TOOL_CALL_STARTED,
            1,
            json!({"call_id": "tool_1", "tool_name": "read", "args_preview": "{}"}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let envelope = make_envelope(
            event_types::TOOL_CALL_FINISHED,
            1,
            json!({"call_id": "tool_1", "tool_name": "read", "status": "ok"}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "tool_execution");
        assert_eq!(events[1].event_type, "tool_result");
    }

    #[test]
    fn test_persist_tool_finished_keeps_trace_payload_v2() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::TOOL_CALL_FINISHED,
            1,
            json!({
                "call_id": "tool_todo_1",
                "tool_name": "todowrite",
                "status": "ok",
                "trace": {
                    "schema_version": 2,
                    "stage": "result",
                    "meta": {
                        "tool": "todowrite",
                        "call_id": "tool_todo_1",
                        "duration_ms": 12
                    },
                    "result": {
                        "ok": true,
                        "preview": {
                            "todo_state": {
                                "items": [{"status": "in_progress", "text": "Plan"}],
                                "last_updated_at": 123,
                                "source_call_id": "tool_todo_1"
                            }
                        },
                        "error": null
                    }
                }
            }),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 1);
        let payload = events[0]
            .payload
            .as_ref()
            .expect("tool_result payload should exist");
        assert!(
            payload.get("result").is_none(),
            "payload.result mirror should not be present in v2"
        );
        assert_eq!(
            payload
                .get("trace")
                .and_then(|v| v.get("result"))
                .and_then(|v| v.get("preview"))
                .and_then(|v| v.get("todo_state"))
                .and_then(|v| v.get("items"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.len()),
            Some(1)
        );
    }

    #[test]
    fn test_persist_waiting_for_confirmation() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::WAITING_FOR_CONFIRMATION,
            1,
            json!({"call_id": "tool_1", "tool_name": "edit", "reason": "sensitive_write"}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "turn_state");

        let payload = events[0].payload.as_ref().unwrap();
        assert_eq!(payload["state"], "waiting_confirmation");
    }

    #[test]
    fn test_persist_waiting_for_askuser_and_answered() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let requested = make_envelope(
            event_types::ASKUSER_REQUESTED,
            2,
            json!({
                "call_id": "ask_call_1",
                "tool_name": "askuser",
                "llm_call_id": "call_ask_1",
                "questionnaire": "1. [question] Q\n[topic] T\n[option] A\n[option] B"
            }),
        );
        assert!(sink.persist_event(&requested).unwrap());

        let answered = make_envelope(
            event_types::ASKUSER_ANSWERED,
            2,
            json!({"call_id": "ask_call_1", "llm_call_id": "call_ask_1", "answers": [{"topic": "T", "value": "A"}]}),
        );
        assert!(sink.persist_event(&answered).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "turn_state");
        assert_eq!(events[1].event_type, "turn_state");

        let payload0 = events[0].payload.as_ref().unwrap();
        assert_eq!(payload0["state"], "waiting_askuser");
        assert_eq!(payload0["call_id"], "ask_call_1");
        assert_eq!(payload0["tool_name"], "askuser");
        assert_eq!(
            payload0["questionnaire"],
            "1. [question] Q\n[topic] T\n[option] A\n[option] B"
        );

        let payload1 = events[1].payload.as_ref().unwrap();
        assert_eq!(payload1["state"], "resumed");
        assert_eq!(payload1["call_id"], "ask_call_1");
    }

    #[test]
    fn test_persist_compaction_events() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let envelope = make_envelope(
            event_types::COMPACTION_STARTED,
            1,
            json!({"reason": "threshold"}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let envelope = make_envelope(
            event_types::COMPACTION_FINISHED,
            1,
            json!({"meta": {"removed_count": 10}}),
        );
        assert!(sink.persist_event(&envelope).unwrap());

        let fallback = make_envelope(
            event_types::COMPACTION_FALLBACK,
            1,
            json!({"reason": "missing_credentials", "message": "fallback"}),
        );
        assert!(sink.persist_event(&fallback).unwrap());

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "compaction_started");
        assert_eq!(events[1].event_type, "compaction_finished");
        assert_eq!(events[2].event_type, "compaction_fallback");
    }

    #[test]
    fn test_persist_assistant_message() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let msg = super::super::messages::AgentMessage {
            id: "msg_test_1".to_string(),
            role: super::super::messages::Role::Assistant,
            blocks: vec![super::super::messages::ContentBlock::Text {
                text: "Hello, I can help with that.".to_string(),
            }],
            ts: chrono::Utc::now().timestamp_millis(),
        };

        sink.persist_assistant_message(&msg, 1).unwrap();

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "message");

        let payload = events[0].payload.as_ref().unwrap();
        assert_eq!(payload["role"], "assistant");
        assert_eq!(payload["content"], "Hello, I can help with that.");
        assert_eq!(payload["message_id"], "msg_test_1");
    }

    #[test]
    fn test_persist_turn_state() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        sink.persist_turn_state(
            1,
            "waiting_confirmation",
            json!({"call_id": "tool_42", "tool_name": "edit"}),
        )
        .unwrap();

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "turn_state");

        let payload = events[0].payload.as_ref().unwrap();
        assert_eq!(payload["state"], "waiting_confirmation");
        assert_eq!(payload["call_id"], "tool_42");
        assert_eq!(payload["tool_name"], "edit");
    }

    #[test]
    fn test_event_seq_increments() {
        let project = setup_temp_project();
        let sink = SessionPersistenceSink::new(&project, "test_session");

        let e1 = make_envelope(event_types::TURN_STARTED, 1, json!({}));
        sink.persist_event(&e1).unwrap();
        let e2 = make_envelope(event_types::TURN_COMPLETED, 1, json!({}));
        sink.persist_event(&e2).unwrap();

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events[0].event_seq, Some(1));
        assert_eq!(events[1].event_seq, Some(2));
    }

    #[test]
    fn test_event_seq_continues_across_sink_instances() {
        let project = setup_temp_project();

        let sink = SessionPersistenceSink::new(&project, "test_session");
        sink.persist_event(&make_envelope(event_types::TURN_STARTED, 1, json!({})))
            .unwrap();

        let resumed_sink = SessionPersistenceSink::new(&project, "test_session");
        resumed_sink
            .persist_event(&make_envelope(event_types::TURN_COMPLETED, 1, json!({})))
            .unwrap();

        let events =
            crate::services::agent_session::read_events_jsonl(&sink.stream_path()).unwrap();
        assert_eq!(events[0].event_seq, Some(1));
        assert_eq!(events[1].event_seq, Some(2));
    }

    #[test]
    fn test_turn_state_dedupe_key_distinguishes_waiting_and_resumed() {
        let waiting_key = build_runtime_dedupe_key(
            session_event_types::TURN_STATE,
            Some(3),
            Some(&json!({
                "state": "waiting_confirmation",
                "call_id": "call_1"
            })),
        )
        .expect("waiting key");

        let resumed_key = build_runtime_dedupe_key(
            session_event_types::TURN_STATE,
            Some(3),
            Some(&json!({
                "state": "resumed",
                "call_id": "call_1"
            })),
        )
        .expect("resumed key");

        assert_ne!(waiting_key, resumed_key);
        assert!(waiting_key.contains("state:waiting_confirmation"));
        assert!(resumed_key.contains("state:resumed"));
    }

    #[test]
    fn test_token_usage_dedupe_key_includes_usage_counters() {
        let usage_key = build_runtime_dedupe_key(
            session_event_types::TOKEN_USAGE,
            Some(2),
            Some(&json!({
                "input_tokens": 10,
                "output_tokens": 20,
                "cache_read_tokens": 3,
                "thinking_tokens": 7
            })),
        )
        .expect("usage key");

        assert!(usage_key.contains("in:10"));
        assert!(usage_key.contains("out:20"));
        assert!(usage_key.contains("cache:3"));
        assert!(usage_key.contains("think:7"));
    }
}
