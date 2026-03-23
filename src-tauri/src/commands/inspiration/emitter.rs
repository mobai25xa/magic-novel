use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::agent_engine::emitter::EventSink;
use crate::agent_engine::events::{EventEnvelope, AGENT_EVENT_CHANNEL};
use crate::agent_engine::messages::AgentMessage;
use crate::models::AppError;
use crate::services::inspiration_session::InspirationSessionPersistenceSink;

#[derive(Clone)]
pub struct InspirationEventEmitter {
    app_handle: AppHandle,
    session_id: String,
    turn_id: u32,
    client_request_id: Option<String>,
    persistence: Option<Arc<InspirationSessionPersistenceSink>>,
}

impl InspirationEventEmitter {
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

    pub fn with_persistence(mut self, sink: InspirationSessionPersistenceSink) -> Self {
        self.persistence = Some(Arc::new(sink));
        self
    }
}

impl EventSink for InspirationEventEmitter {
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
            .map_err(|err| AppError::internal(format!("failed to emit event: {err}")))?;

        if let Some(ref sink) = self.persistence {
            if let Err(err) = sink.persist_event(&envelope) {
                tracing::warn!(
                    target: "inspiration",
                    error = %err,
                    event_type = event_type,
                    "failed to persist inspiration event"
                );
            }
        }

        Ok(())
    }

    fn persist_user_message(&self, text: &str, turn: u32) -> Result<(), AppError> {
        if let Some(ref sink) = self.persistence {
            sink.persist_user_message(text, turn)?;
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
