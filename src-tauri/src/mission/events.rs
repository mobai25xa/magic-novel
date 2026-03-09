//! Mission system - Event protocol (Rust → UI)
//!
//! Emits mission lifecycle events on the `magic:mission_event` Tauri channel.
//! Mirrors the pattern from agent_engine/emitter.rs.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::models::AppError;

pub const MISSION_EVENT_CHANNEL: &str = "magic:mission_event";

pub mod mission_event_types {
    pub const MISSION_STATE_CHANGED: &str = "MISSION_STATE_CHANGED";
    pub const MISSION_FEATURES_CHANGED: &str = "MISSION_FEATURES_CHANGED";
    pub const MISSION_PROGRESS_ENTRY: &str = "MISSION_PROGRESS_ENTRY";
    pub const WORKER_STARTED: &str = "WORKER_STARTED";
    pub const WORKER_COMPLETED: &str = "WORKER_COMPLETED";
    pub const MISSION_HEARTBEAT: &str = "MISSION_HEARTBEAT";
}

/// Envelope wrapping every mission event sent to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEventEnvelope {
    pub schema_version: i32,
    pub event_id: String,
    pub ts: i64,
    pub mission_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

impl MissionEventEnvelope {
    pub fn new(mission_id: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            schema_version: super::types::MISSION_SCHEMA_VERSION,
            event_id: format!("mevt_{}", uuid::Uuid::new_v4()),
            ts: chrono::Utc::now().timestamp_millis(),
            mission_id: mission_id.to_string(),
            event_type: event_type.to_string(),
            payload,
        }
    }
}

/// Emitter for mission events through the Tauri event bus.
#[derive(Clone)]
pub struct MissionEventEmitter {
    app_handle: AppHandle,
    mission_id: String,
}

impl MissionEventEmitter {
    pub fn new(app_handle: AppHandle, mission_id: String) -> Self {
        Self {
            app_handle,
            mission_id,
        }
    }

    fn emit(&self, event_type: &str, payload: serde_json::Value) -> Result<(), AppError> {
        let envelope = MissionEventEnvelope::new(&self.mission_id, event_type, payload);

        self.app_handle
            .emit(MISSION_EVENT_CHANNEL, &envelope)
            .map_err(|e| AppError::internal(format!("failed to emit mission event: {e}")))?;

        tracing::debug!(
            target: "mission",
            event_type = event_type,
            mission_id = %self.mission_id,
            "emitted mission event"
        );

        Ok(())
    }

    // ── State ───────────────────────────────────────────────────

    pub fn state_changed(&self, old_state: &str, new_state: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_STATE_CHANGED;
        self.emit(
            MISSION_STATE_CHANGED,
            json!({
                "old_state": old_state,
                "new_state": new_state,
            }),
        )
    }

    // ── Features ────────────────────────────────────────────────

    pub fn features_changed(&self, feature_id: &str, new_status: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_FEATURES_CHANGED;
        self.emit(
            MISSION_FEATURES_CHANGED,
            json!({
                "feature_id": feature_id,
                "new_status": new_status,
            }),
        )
    }

    // ── Progress ────────────────────────────────────────────────

    pub fn progress_entry(&self, message: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_PROGRESS_ENTRY;
        self.emit(
            MISSION_PROGRESS_ENTRY,
            json!({
                "message": message,
            }),
        )
    }

    // ── Worker ──────────────────────────────────────────────────

    pub fn worker_started(&self, worker_id: &str, feature_id: &str) -> Result<(), AppError> {
        use mission_event_types::WORKER_STARTED;
        self.emit(
            WORKER_STARTED,
            json!({
                "worker_id": worker_id,
                "feature_id": feature_id,
            }),
        )
    }

    pub fn worker_completed(
        &self,
        worker_id: &str,
        feature_id: &str,
        ok: bool,
        summary: &str,
    ) -> Result<(), AppError> {
        use mission_event_types::WORKER_COMPLETED;
        self.emit(
            WORKER_COMPLETED,
            json!({
                "worker_id": worker_id,
                "feature_id": feature_id,
                "ok": ok,
                "summary": summary,
            }),
        )
    }

    // ── Heartbeat ───────────────────────────────────────────────

    pub fn heartbeat(&self, worker_id: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_HEARTBEAT;
        self.emit(
            MISSION_HEARTBEAT,
            json!({
                "worker_id": worker_id,
            }),
        )
    }
}
