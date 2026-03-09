use std::collections::HashSet;

use crate::models::{AppError, ErrorCode};

use super::types::{session_event_types, AgentSessionEvent};
use super::AGENT_SESSION_SCHEMA_VERSION;

#[derive(Debug, Clone, Default)]
pub struct EventContractState {
    pub last_event_seq: i64,
    dedupe_keys: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedEventBatch {
    pub events: Vec<AgentSessionEvent>,
    pub deduped_count: usize,
    pub last_event_seq: i64,
}

impl EventContractState {
    pub fn from_existing(events: &[AgentSessionEvent]) -> Self {
        let mut state = Self::default();

        for (index, event) in events.iter().enumerate() {
            let fallback_seq = (index as i64).saturating_add(1);
            let seq = event.event_seq.unwrap_or(fallback_seq);
            if seq > state.last_event_seq {
                state.last_event_seq = seq;
            }

            if let Some(dedupe_key) = resolve_dedupe_key(event) {
                state.dedupe_keys.insert(dedupe_key);
            }
        }

        state
    }

    fn contains_dedupe_key(&self, dedupe_key: &str) -> bool {
        self.dedupe_keys.contains(dedupe_key)
    }

    fn insert_dedupe_key(&mut self, dedupe_key: String) {
        self.dedupe_keys.insert(dedupe_key);
    }
}

pub fn prepare_events_for_append(
    session_id: &str,
    events: &[AgentSessionEvent],
    mut state: EventContractState,
) -> Result<PreparedEventBatch, AppError> {
    let mut prepared = Vec::with_capacity(events.len());
    let mut deduped_count = 0_usize;

    for (index, event) in events.iter().enumerate() {
        validate_event_schema(index, event)?;
        validate_event_session(index, session_id, event)?;

        let mut normalized = event.clone();
        let dedupe_key = resolve_dedupe_key(&normalized);

        if let Some(dedupe_key) = dedupe_key.clone() {
            if state.contains_dedupe_key(&dedupe_key) {
                deduped_count = deduped_count.saturating_add(1);
                continue;
            }
            state.insert_dedupe_key(dedupe_key.clone());
            normalized.dedupe_key = Some(dedupe_key);
        }

        let next_seq = normalized
            .event_seq
            .unwrap_or_else(|| state.last_event_seq.saturating_add(1));

        if next_seq <= 0 || next_seq <= state.last_event_seq {
            return Err(AppError {
                code: ErrorCode::Conflict,
                message: "session event_seq must be strictly monotonic".to_string(),
                details: Some(serde_json::json!({
                    "code": "E_AGENT_SESSION_EVENT_SEQ_NON_MONOTONIC",
                    "session_id": session_id,
                    "event_index": index,
                    "event_type": normalized.event_type,
                    "last_event_seq": state.last_event_seq,
                    "event_seq": next_seq,
                })),
                recoverable: Some(true),
            });
        }

        normalized.event_seq = Some(next_seq);
        state.last_event_seq = next_seq;
        prepared.push(normalized);
    }

    Ok(PreparedEventBatch {
        events: prepared,
        deduped_count,
        last_event_seq: state.last_event_seq,
    })
}

fn validate_event_schema(index: usize, event: &AgentSessionEvent) -> Result<(), AppError> {
    if event.validate_v1() {
        return Ok(());
    }

    Err(AppError {
        code: ErrorCode::SchemaValidationError,
        message: "invalid session event schema".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_EVENT_INVALID",
            "schema_version": AGENT_SESSION_SCHEMA_VERSION,
            "event_index": index,
        })),
        recoverable: Some(true),
    })
}

fn validate_event_session(
    index: usize,
    session_id: &str,
    event: &AgentSessionEvent,
) -> Result<(), AppError> {
    if event.session_id == session_id {
        return Ok(());
    }

    Err(AppError {
        code: ErrorCode::InvalidArgument,
        message: "event session_id does not match input session_id".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_EVENT_SESSION_MISMATCH",
            "session_id": session_id,
            "event_index": index,
            "event_session_id": event.session_id,
        })),
        recoverable: Some(true),
    })
}

fn resolve_dedupe_key(event: &AgentSessionEvent) -> Option<String> {
    normalize_value(event.dedupe_key.as_deref())
        .or_else(|| {
            normalize_value(event.event_id.as_deref())
                .map(|event_id| format!("event_id:{event_id}"))
        })
        .or_else(|| payload_key(event, "call_id", "call"))
        .or_else(|| payload_key(event, "message_id", "message"))
        .or_else(|| {
            event
                .turn
                .map(|turn| format!("{}:turn:{turn}", event.event_type.trim()))
        })
        .or_else(|| {
            if event.event_type.trim() == session_event_types::SESSION_START {
                Some("session_start".to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            let event_type = event.event_type.trim();
            if event_type.is_empty() {
                None
            } else {
                Some(format!("{event_type}:ts:{}", event.ts))
            }
        })
}

fn payload_key(event: &AgentSessionEvent, field: &str, prefix: &str) -> Option<String> {
    let key = event
        .payload
        .as_ref()
        .and_then(|payload| payload.get(field))
        .and_then(|value| value.as_str())
        .and_then(|value| normalize_value(Some(value)))?;

    let event_type = normalize_value(Some(event.event_type.as_str()))
        .unwrap_or_else(|| "unknown_event".to_string());
    Some(format!("{event_type}:{prefix}:{key}"))
}

fn normalize_value(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
