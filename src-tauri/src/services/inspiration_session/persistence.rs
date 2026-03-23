use std::sync::Arc;

use serde_json::{json, Value};

use crate::agent_engine::events::event_types;
use crate::agent_engine::events::EventEnvelope;
use crate::agent_engine::messages::AgentMessage;
use crate::models::AppError;

use super::stream::append_session_events;
use super::types::{
    session_event_types, InspirationSessionEvent, InspirationSessionEvent as Event,
};
use super::INSPIRATION_SESSION_SCHEMA_VERSION;

#[derive(Clone)]
pub struct InspirationSessionPersistenceSink {
    session_id: Arc<String>,
    client_request_id: Option<String>,
    hydration_source: Option<String>,
}

impl InspirationSessionPersistenceSink {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Arc::new(session_id.into()),
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

    fn build_event(&self, event_type: &str, turn: Option<i64>, payload: Option<Value>) -> Event {
        Event {
            schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
            event_type: event_type.to_string(),
            session_id: self.session_id.as_ref().clone(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: None,
            dedupe_key: build_runtime_dedupe_key(event_type, turn, payload.as_ref()),
            turn,
            payload,
        }
    }

    fn append(&self, events: &[InspirationSessionEvent]) -> Result<(), AppError> {
        append_session_events(self.session_id.as_ref(), events)?;
        Ok(())
    }

    pub fn persist_event(&self, envelope: &EventEnvelope) -> Result<bool, AppError> {
        let (session_type, payload) = match map_event(envelope, self.hydration_source.as_deref()) {
            Some(mapped) => mapped,
            None => return Ok(false),
        };

        let event = self.build_event(session_type, Some(envelope.turn_id as i64), Some(payload));
        self.append(&[event])?;
        Ok(true)
    }

    pub fn persist_assistant_message(&self, msg: &AgentMessage, turn: u32) -> Result<(), AppError> {
        let tool_calls = msg
            .tool_calls()
            .iter()
            .map(|tc| {
                json!({
                    "llm_call_id": tc.llm_call_id,
                    "tool_name": tc.tool_name,
                })
            })
            .collect::<Vec<_>>();
        let payload = append_event_diagnostics(
            json!({
                "role": "assistant",
                "content": msg.text_content(),
                "message_id": msg.id,
                "tool_calls": tool_calls,
            }),
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

    pub fn persist_user_message(&self, text: &str, turn: u32) -> Result<(), AppError> {
        let payload = append_event_diagnostics(
            json!({
                "role": "user",
                "content": text,
                "message_id": format!("msg_{}", uuid::Uuid::new_v4()),
            }),
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

    pub fn persist_turn_state(&self, turn: u32, state: &str, extra: Value) -> Result<(), AppError> {
        let mut payload = json!({ "state": state });
        if let (Value::Object(target), Value::Object(map)) = (&mut payload, extra) {
            target.extend(map);
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

fn map_event(
    envelope: &EventEnvelope,
    hydration_source: Option<&str>,
) -> Option<(&'static str, Value)> {
    let mapped = match envelope.event_type.as_str() {
        event_types::ASSISTANT_TEXT_DELTA
        | event_types::THINKING_TEXT_DELTA
        | event_types::STREAMING_STARTED
        | event_types::TOOL_CALL_PROGRESS => None,
        event_types::TURN_STARTED | event_types::PLAN_STARTED => {
            Some((session_event_types::TURN_STARTED, envelope.payload.clone()))
        }
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
        event_types::TOOL_CALL_STARTED => Some((
            session_event_types::TOOL_EXECUTION,
            envelope.payload.clone(),
        )),
        event_types::TOOL_CALL_FINISHED => {
            Some((session_event_types::TOOL_RESULT, envelope.payload.clone()))
        }
        event_types::WAITING_FOR_CONFIRMATION => Some((
            session_event_types::TURN_STATE,
            json!({
                "state": "waiting_confirmation",
                "call_id": envelope.payload.get("call_id"),
                "tool_name": envelope.payload.get("tool_name"),
                "reason": envelope.payload.get("reason"),
            }),
        )),
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
        event_types::USAGE_UPDATE => {
            Some((session_event_types::TOKEN_USAGE, envelope.payload.clone()))
        }
        _ => None,
    }?;

    Some((
        mapped.0,
        append_event_diagnostics(
            mapped.1,
            envelope.turn_id,
            envelope.client_request_id.as_deref(),
            hydration_source,
        ),
    ))
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
    }

    if parts.len() == 1 {
        None
    } else {
        Some(parts.join(":"))
    }
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
