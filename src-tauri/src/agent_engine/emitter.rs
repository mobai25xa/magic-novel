//! Agent Engine - Event emitter (wraps Tauri app_handle.emit)
//!
//! Defines the `EventSink` trait so the agent loop can emit events without
//! depending on `AppHandle` (enabling use in the worker process via `StdoutEventSink`).

use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::mission::worker_protocol::WorkerEvent;

use crate::models::AppError;

use super::events::{EventEnvelope, AGENT_EVENT_CHANNEL};
use super::messages::AgentMessage;
use super::persistence::SessionPersistenceSink;
use super::text_utils::truncate_chars;
use super::types::{StopReason, ToolCallInfo, UsageInfo};

fn merge_payload_meta(
    mut payload: serde_json::Value,
    extra_meta: Option<serde_json::Value>,
) -> serde_json::Value {
    if let (Some(extra), Some(object)) = (extra_meta, payload.as_object_mut()) {
        if let Some(extra_object) = extra.as_object() {
            for (key, value) in extra_object {
                object.insert(key.clone(), value.clone());
            }
        }
    }
    payload
}

// ── EventSink trait ──────────────────────────────────────────────

/// Abstraction over event emission so `AgentLoop` does not depend on `AppHandle`.
///
/// Two implementations:
/// - `AgentEventEmitter` – Tauri process, emits via `AppHandle::emit`
/// - `StdoutEventSink`   – Worker process, writes `WorkerEvent::AgentEvent` to stdout
pub trait EventSink: Send + Sync + Clone + 'static {
    fn emit_raw(&self, event_type: &str, payload: serde_json::Value) -> Result<(), AppError>;

    fn source_kind(&self) -> &'static str {
        "agent"
    }

    fn persist_user_message(&self, _msg: &AgentMessage, _turn: u32) -> Result<(), AppError> {
        Ok(())
    }

    fn persist_assistant_message(&self, _msg: &AgentMessage, _turn: u32) -> Result<(), AppError> {
        Ok(())
    }

    fn persist_turn_state(
        &self,
        _turn: u32,
        _state: &str,
        _payload: serde_json::Value,
    ) -> Result<(), AppError> {
        Ok(())
    }

    // ── Turn lifecycle ───────────────────────────────────────────

    fn turn_started(
        &self,
        user_text_preview: &str,
        model_provider: &str,
        model: &str,
    ) -> Result<(), AppError> {
        self.turn_started_with_meta(user_text_preview, model_provider, model, None)
    }

    fn turn_started_with_meta(
        &self,
        user_text_preview: &str,
        model_provider: &str,
        model: &str,
        extra_meta: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        use super::events::event_types::TURN_STARTED;
        self.emit_raw(
            TURN_STARTED,
            merge_payload_meta(
                json!({
                    "user_text_preview": truncate_chars(user_text_preview, 200),
                    "model_provider": model_provider,
                    "model": model,
                }),
                extra_meta,
            ),
        )
    }

    fn turn_completed(
        &self,
        stop_reason: &StopReason,
        latency_ms: u64,
        compacted: bool,
    ) -> Result<(), AppError> {
        self.turn_completed_with_meta(stop_reason, latency_ms, compacted, None)
    }

    fn turn_completed_with_meta(
        &self,
        stop_reason: &StopReason,
        latency_ms: u64,
        compacted: bool,
        extra_meta: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        use super::events::event_types::TURN_COMPLETED;
        self.emit_raw(
            TURN_COMPLETED,
            merge_payload_meta(
                json!({
                    "stop_reason": stop_reason,
                    "latency_ms": latency_ms,
                    "compacted": compacted,
                }),
                extra_meta,
            ),
        )
    }

    fn turn_failed(
        &self,
        error_code: &str,
        error_message: &str,
        detail: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        use super::events::event_types::TURN_FAILED;
        let mut payload = json!({
            "error_code": error_code,
            "error_message": error_message,
        });
        if let Some(d) = detail {
            payload["error_detail"] = d;
        }
        self.emit_raw(TURN_FAILED, payload)
    }

    fn turn_cancelled(&self) -> Result<(), AppError> {
        use super::events::event_types::TURN_CANCELLED;
        self.emit_raw(TURN_CANCELLED, json!({}))
    }

    // ── Streaming ────────────────────────────────────────────────

    fn streaming_started(&self) -> Result<(), AppError> {
        use super::events::event_types::STREAMING_STARTED;
        self.emit_raw(STREAMING_STARTED, json!({}))
    }

    fn assistant_text_delta(&self, delta: &str) -> Result<(), AppError> {
        use super::events::event_types::ASSISTANT_TEXT_DELTA;
        self.emit_raw(ASSISTANT_TEXT_DELTA, json!({ "delta": delta }))
    }

    fn usage_update(&self, usage: &UsageInfo) -> Result<(), AppError> {
        use super::events::event_types::USAGE_UPDATE;
        self.emit_raw(
            USAGE_UPDATE,
            serde_json::to_value(usage).unwrap_or_default(),
        )
    }

    fn thinking_text_delta(&self, delta: &str) -> Result<(), AppError> {
        use super::events::event_types::THINKING_TEXT_DELTA;
        self.emit_raw(THINKING_TEXT_DELTA, json!({ "delta": delta }))
    }

    // ── Tool ─────────────────────────────────────────────────────

    fn tool_call_started(&self, tc: &ToolCallInfo, call_id: &str) -> Result<(), AppError> {
        use super::events::event_types::TOOL_CALL_STARTED;
        self.emit_raw(
            TOOL_CALL_STARTED,
            json!({
                "llm_call_id": tc.llm_call_id,
                "call_id": call_id,
                "tool_name": tc.tool_name,
                "args_preview": truncate_chars(&tc.args.to_string(), 500),
            }),
        )?;
        self.tool_call_progress(tc, call_id, "started")
    }

    fn tool_call_finished(
        &self,
        tc: &ToolCallInfo,
        call_id: &str,
        status: &str,
        trace: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        use super::events::event_types::TOOL_CALL_FINISHED;
        self.emit_raw(
            TOOL_CALL_FINISHED,
            json!({
                "llm_call_id": tc.llm_call_id,
                "call_id": call_id,
                "tool_name": tc.tool_name,
                "status": status,
                "trace": trace,
            }),
        )
    }

    fn tool_call_progress(
        &self,
        tc: &ToolCallInfo,
        call_id: &str,
        progress: &str,
    ) -> Result<(), AppError> {
        use super::events::event_types::TOOL_CALL_PROGRESS;
        self.emit_raw(
            TOOL_CALL_PROGRESS,
            json!({
                "llm_call_id": tc.llm_call_id,
                "call_id": call_id,
                "tool_name": tc.tool_name,
                "progress": progress,
            }),
        )
    }

    fn waiting_for_confirmation(
        &self,
        tc: &ToolCallInfo,
        call_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        use super::events::event_types::WAITING_FOR_CONFIRMATION;
        self.emit_raw(
            WAITING_FOR_CONFIRMATION,
            json!({
                "llm_call_id": tc.llm_call_id,
                "call_id": call_id,
                "tool_name": tc.tool_name,
                "reason": reason,
            }),
        )?;
        self.tool_call_progress(tc, call_id, "waiting_confirmation")
    }

    fn askuser_requested(
        &self,
        tc: &ToolCallInfo,
        call_id: &str,
        questions: Option<&serde_json::Value>,
        questionnaire: Option<&str>,
    ) -> Result<(), AppError> {
        use super::events::event_types::ASKUSER_REQUESTED;
        let mut payload = json!({
            "llm_call_id": tc.llm_call_id,
            "call_id": call_id,
            "tool_name": tc.tool_name,
        });
        if let Some(q) = questions {
            payload["questions"] = q.clone();
        }
        if let Some(qs) = questionnaire {
            payload["questionnaire"] = json!(qs);
        }
        self.emit_raw(ASKUSER_REQUESTED, payload)?;
        self.tool_call_progress(tc, call_id, "waiting_askuser")
    }

    fn askuser_answered(
        &self,
        tc: &ToolCallInfo,
        call_id: &str,
        answers: &serde_json::Value,
    ) -> Result<(), AppError> {
        use super::events::event_types::ASKUSER_ANSWERED;
        self.emit_raw(
            ASKUSER_ANSWERED,
            json!({
                "llm_call_id": tc.llm_call_id,
                "call_id": call_id,
                "tool_name": tc.tool_name,
                "answers": answers,
            }),
        )?;

        self.tool_call_progress(tc, call_id, "done")?;
        self.tool_call_finished(
            tc,
            call_id,
            "ok",
            Some(json!({
                "schema_version": 2,
                "stage": "result",
                "meta": {
                    "tool": tc.tool_name.as_str(),
                    "call_id": call_id,
                    "duration_ms": 0,
                },
                "result": {
                    "ok": true,
                    "preview": {
                        "askuser_response": {
                            "answers": answers,
                        },
                    },
                    "error": serde_json::Value::Null,
                },
            })),
        )
    }

    // ── Compaction ───────────────────────────────────────────────

    fn compaction_started(&self, reason: &str) -> Result<(), AppError> {
        use super::events::event_types::COMPACTION_STARTED;
        self.emit_raw(COMPACTION_STARTED, json!({ "reason": reason }))
    }

    fn compaction_finished(&self, meta: serde_json::Value) -> Result<(), AppError> {
        use super::events::event_types::COMPACTION_FINISHED;
        self.emit_raw(COMPACTION_FINISHED, json!({ "meta": meta }))
    }
}

// ── AgentEventEmitter (Tauri process) ───────────────────────────

/// Emits agent runtime events through the Tauri event bus
#[derive(Clone)]
pub struct AgentEventEmitter {
    app_handle: AppHandle,
    session_id: String,
    turn_id: u32,
    client_request_id: Option<String>,
    persistence: Option<Arc<SessionPersistenceSink>>,
}

impl AgentEventEmitter {
    pub fn new(app_handle: AppHandle, session_id: String, turn_id: u32) -> Self {
        Self {
            app_handle,
            session_id,
            turn_id,
            client_request_id: None,
            persistence: None,
        }
    }

    pub fn with_client_request_id(mut self, client_request_id: Option<String>) -> Self {
        self.client_request_id = client_request_id;
        self
    }

    pub fn with_persistence(mut self, sink: SessionPersistenceSink) -> Self {
        self.persistence = Some(Arc::new(sink));
        self
    }

    pub fn set_turn_id(&mut self, turn_id: u32) {
        self.turn_id = turn_id;
    }
}

impl EventSink for AgentEventEmitter {
    fn emit_raw(&self, event_type: &str, payload: serde_json::Value) -> Result<(), AppError> {
        let envelope = EventEnvelope::new_with_client_request_id(
            &self.session_id,
            self.turn_id,
            event_type,
            payload,
            self.client_request_id.as_deref(),
        );

        self.app_handle
            .emit(AGENT_EVENT_CHANNEL, &envelope)
            .map_err(|e| AppError::internal(format!("failed to emit event: {e}")))?;

        if let Some(ref sink) = self.persistence {
            if let Err(err) = sink.persist_event(&envelope) {
                tracing::warn!(
                    target: "agent_engine",
                    error = %err,
                    event_type = event_type,
                    "failed to persist agent event"
                );
            }
        }

        tracing::debug!(
            target: "agent_engine",
            event_type = event_type,
            session_id = %self.session_id,
            turn_id = self.turn_id,
            "emitted agent event"
        );

        Ok(())
    }

    fn persist_user_message(&self, msg: &AgentMessage, turn: u32) -> Result<(), AppError> {
        if let Some(ref sink) = self.persistence {
            sink.persist_user_message(msg, turn)?;
        }
        Ok(())
    }

    fn persist_assistant_message(&self, msg: &AgentMessage, turn: u32) -> Result<(), AppError> {
        if let Some(ref sink) = self.persistence {
            sink.persist_assistant_message(msg, turn)?;
        }
        Ok(())
    }

    fn persist_turn_state(
        &self,
        turn: u32,
        state: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError> {
        if let Some(ref sink) = self.persistence {
            sink.persist_turn_state(turn, state, payload)?;
        }
        Ok(())
    }
}

// ── StdoutEventSink (Worker process) ────────────────────────────

/// EventSink implementation for the worker process.
///
/// Wraps each agent event as a `WorkerEvent::AgentEvent` NDJSON line
/// written to stdout, so the orchestrator (Tauri process) can forward
/// it as a `magic:agent_event` to the UI.
#[derive(Clone)]
pub struct StdoutEventSink {
    pub session_id: String,
    pub turn_id: u32,
    pub worker_id: String,
    pub mission_id: String,
}

impl StdoutEventSink {
    pub fn new(session_id: String, turn_id: u32, mission_id: String, worker_id: String) -> Self {
        Self {
            session_id,
            turn_id,
            worker_id,
            mission_id,
        }
    }
}

impl EventSink for StdoutEventSink {
    fn emit_raw(&self, event_type: &str, payload: serde_json::Value) -> Result<(), AppError> {
        use std::io::Write;

        let envelope = serde_json::json!({
            "schema_version": 1,
            "event_id": format!("evt_{}", uuid::Uuid::new_v4()),
            "session_id": self.session_id,
            "turn_id": self.turn_id,
            "source": {
                "kind": "worker",
                "worker_id": self.worker_id,
                "mission_id": self.mission_id,
            },
            "type": event_type,
            "payload": payload,
            "ts": chrono::Utc::now().timestamp_millis(),
        });

        // Wrap in WorkerEvent::AgentEvent wire format
        let worker_event = WorkerEvent::agent_event(envelope);

        let line = serde_json::to_string(&worker_event)
            .map_err(|e| AppError::internal(format!("failed to serialize agent event: {e}")))?;

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(line.as_bytes())
            .and_then(|_| handle.write_all(b"\n"))
            .and_then(|_| handle.flush())
            .map_err(|e| AppError::internal(format!("stdout write failed: {e}")))?;

        tracing::debug!(
            target: "agent_worker",
            event_type = event_type,
            session_id = %self.session_id,
            "emitted agent event to stdout"
        );

        Ok(())
    }

    fn source_kind(&self) -> &'static str {
        "worker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdout_sink_envelope_includes_event_id_and_source() {
        let sink = StdoutEventSink::new(
            "session_test".to_string(),
            7,
            "mis_test".to_string(),
            "wk_test".to_string(),
        );
        let payload = serde_json::json!({ "delta": "hello" });

        let envelope = serde_json::json!({
            "schema_version": 1,
            "event_id": format!("evt_{}", uuid::Uuid::new_v4()),
            "session_id": sink.session_id,
            "turn_id": sink.turn_id,
            "source": {
                "kind": "worker",
                "worker_id": sink.worker_id,
                "mission_id": sink.mission_id,
            },
            "type": "ASSISTANT_TEXT_DELTA",
            "payload": payload,
            "ts": chrono::Utc::now().timestamp_millis(),
        });

        let worker_event = WorkerEvent::agent_event(envelope.clone());
        let wire = serde_json::to_value(worker_event).expect("worker event should serialize");
        let nested = wire
            .get("payload")
            .and_then(|v| v.as_object())
            .expect("nested payload must be object");

        assert_eq!(
            nested.get("session_id").and_then(|v| v.as_str()),
            Some("session_test")
        );
        assert_eq!(nested.get("turn_id").and_then(|v| v.as_u64()), Some(7));
        assert!(nested.get("event_id").and_then(|v| v.as_str()).is_some());

        let source = nested
            .get("source")
            .and_then(|v| v.as_object())
            .expect("source must be object");
        assert_eq!(source.get("kind").and_then(|v| v.as_str()), Some("worker"));
        assert_eq!(
            source.get("worker_id").and_then(|v| v.as_str()),
            Some("wk_test")
        );
        assert_eq!(
            source.get("mission_id").and_then(|v| v.as_str()),
            Some("mis_test")
        );
    }
}
