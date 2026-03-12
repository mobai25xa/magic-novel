//! Mission system - Event protocol (Rust → UI)
//!
//! Emits mission lifecycle events on the `magic:mission_event` Tauri channel.
//! Mirrors the pattern from agent_engine/emitter.rs.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::models::AppError;
use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeDelta, KnowledgeProposalBundle,
};
use crate::review::types::{ReviewDecisionRequest, ReviewReport};

pub const MISSION_EVENT_CHANNEL: &str = "magic:mission_event";

pub mod mission_event_types {
    pub const MISSION_STATE_CHANGED: &str = "MISSION_STATE_CHANGED";
    pub const MISSION_FEATURES_CHANGED: &str = "MISSION_FEATURES_CHANGED";
    pub const MISSION_PROGRESS_ENTRY: &str = "MISSION_PROGRESS_ENTRY";
    pub const MISSION_LAYER1_UPDATED: &str = "MISSION_LAYER1_UPDATED";
    pub const MISSION_CONTEXTPACK_BUILT: &str = "MISSION_CONTEXTPACK_BUILT";
    pub const WORKER_STARTED: &str = "WORKER_STARTED";
    pub const WORKER_COMPLETED: &str = "WORKER_COMPLETED";
    pub const MISSION_HEARTBEAT: &str = "MISSION_HEARTBEAT";

    // M3: Review Gate
    pub const MISSION_REVIEW_RECORDED: &str = "MISSION_REVIEW_RECORDED";
    pub const MISSION_REVIEW_DECISION_REQUIRED: &str = "MISSION_REVIEW_DECISION_REQUIRED";
    pub const MISSION_FIXUP_PROGRESS: &str = "MISSION_FIXUP_PROGRESS";

    // M4: Knowledge Writeback
    pub const MISSION_KNOWLEDGE_PROPOSED: &str = "MISSION_KNOWLEDGE_PROPOSED";
    pub const MISSION_KNOWLEDGE_DECISION_REQUIRED: &str = "MISSION_KNOWLEDGE_DECISION_REQUIRED";
    pub const MISSION_KNOWLEDGE_APPLIED: &str = "MISSION_KNOWLEDGE_APPLIED";
    pub const MISSION_KNOWLEDGE_ROLLED_BACK: &str = "MISSION_KNOWLEDGE_ROLLED_BACK";
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

    // ── Layer1 / ContextPack (M2+) ─────────────────────────────

    pub fn layer1_updated(&self, kind: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_LAYER1_UPDATED;
        self.emit(
            MISSION_LAYER1_UPDATED,
            json!({
                "kind": kind,
            }),
        )
    }

    pub fn contextpack_built(
        &self,
        scope_ref: &str,
        token_budget: &str,
        generated_at: i64,
    ) -> Result<(), AppError> {
        use mission_event_types::MISSION_CONTEXTPACK_BUILT;
        self.emit(
            MISSION_CONTEXTPACK_BUILT,
            json!({
                "scope_ref": scope_ref,
                "token_budget": token_budget,
                "generated_at": generated_at,
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

    // ── M3: Review Gate ────────────────────────────────────────

    pub fn review_recorded(&self, report: &ReviewReport) -> Result<(), AppError> {
        use mission_event_types::MISSION_REVIEW_RECORDED;

        let mut warn = 0_i32;
        let mut block = 0_i32;
        for i in &report.issues {
            match i.severity {
                crate::review::types::ReviewSeverity::Warn => warn += 1,
                crate::review::types::ReviewSeverity::Block => block += 1,
                _ => {}
            }
        }
        let total = report.issues.len() as i32;

        fn enum_str<T: serde::Serialize>(v: &T) -> String {
            serde_json::to_string(v)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        }

        self.emit(
            MISSION_REVIEW_RECORDED,
            json!({
                "review_id": report.review_id,
                "overall_status": enum_str(&report.overall_status),
                "recommended_action": enum_str(&report.recommended_action),
                "issue_counts": {
                    "total": total,
                    "warn": warn,
                    "block": block,
                },
                "generated_at": report.generated_at,
            }),
        )
    }

    pub fn review_decision_required(&self, req: &ReviewDecisionRequest) -> Result<(), AppError> {
        use mission_event_types::MISSION_REVIEW_DECISION_REQUIRED;
        self.emit(
            MISSION_REVIEW_DECISION_REQUIRED,
            json!({
                "review_id": req.review_id,
                "feature_id": req.feature_id,
                "scope_ref": req.scope_ref,
                "target_refs": req.target_refs,
                "question": req.question,
                "options": req.options,
                "created_at": req.created_at,
            }),
        )
    }

    pub fn fixup_progress(&self, attempt: i32, message: &str) -> Result<(), AppError> {
        use mission_event_types::MISSION_FIXUP_PROGRESS;
        self.emit(
            MISSION_FIXUP_PROGRESS,
            json!({
                "attempt": attempt,
                "message": message,
            }),
        )
    }

    // ── M4: Knowledge Writeback ───────────────────────────────

    pub fn knowledge_proposed(&self, bundle: &KnowledgeProposalBundle) -> Result<(), AppError> {
        use mission_event_types::MISSION_KNOWLEDGE_PROPOSED;

        let mut auto_if_pass = 0_i32;
        let mut manual = 0_i32;
        let mut orchestrator_only = 0_i32;
        for it in &bundle.proposal_items {
            match it.accept_policy {
                KnowledgeAcceptPolicy::AutoIfPass => auto_if_pass += 1,
                KnowledgeAcceptPolicy::Manual => manual += 1,
                KnowledgeAcceptPolicy::OrchestratorOnly => orchestrator_only += 1,
            }
        }
        let total = bundle.proposal_items.len() as i32;

        self.emit(
            MISSION_KNOWLEDGE_PROPOSED,
            json!({
                "bundle_id": bundle.bundle_id,
                "scope_ref": bundle.scope_ref,
                "generated_at": bundle.generated_at,
                "counts": {
                    "total": total,
                    "auto_if_pass": auto_if_pass,
                    "manual": manual,
                    "orchestrator_only": orchestrator_only,
                }
            }),
        )
    }

    pub fn knowledge_decision_required(&self, delta: &KnowledgeDelta) -> Result<(), AppError> {
        use mission_event_types::MISSION_KNOWLEDGE_DECISION_REQUIRED;

        let status_str = serde_json::to_string(&delta.status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let conflicts = delta
            .conflicts
            .iter()
            .map(|c| json!({"type": c.conflict_type, "message": c.message, "item_id": c.item_id, "target_ref": c.target_ref}))
            .collect::<Vec<_>>();

        self.emit(
            MISSION_KNOWLEDGE_DECISION_REQUIRED,
            json!({
                "delta_id": delta.knowledge_delta_id,
                "scope_ref": delta.scope_ref,
                "status": status_str,
                "generated_at": delta.generated_at,
                "conflict_count": delta.conflicts.len(),
                "conflicts": conflicts,
            }),
        )
    }

    pub fn knowledge_applied(&self, delta: &KnowledgeDelta) -> Result<(), AppError> {
        use mission_event_types::MISSION_KNOWLEDGE_APPLIED;
        let token = delta
            .rollback
            .as_ref()
            .and_then(|rb| rb.token.clone());

        self.emit(
            MISSION_KNOWLEDGE_APPLIED,
            json!({
                "delta_id": delta.knowledge_delta_id,
                "scope_ref": delta.scope_ref,
                "applied_at": delta.applied_at,
                "rollback_token": token,
            }),
        )
    }

    pub fn knowledge_rolled_back(
        &self,
        token: &str,
        restored: usize,
        deleted: usize,
    ) -> Result<(), AppError> {
        use mission_event_types::MISSION_KNOWLEDGE_ROLLED_BACK;
        self.emit(
            MISSION_KNOWLEDGE_ROLLED_BACK,
            json!({
                "rollback_token": token,
                "restored": restored,
                "deleted": deleted,
            }),
        )
    }
}
