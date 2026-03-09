use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::paths::AGENT_SESSION_SCHEMA_VERSION;
use super::types::{AgentSessionEvent, AgentSessionMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionIndex {
    pub schema_version: i32,
    pub sessions: Vec<AgentSessionMeta>,
}

impl Default for AgentSessionIndex {
    fn default() -> Self {
        Self {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            sessions: Vec::new(),
        }
    }
}

pub fn load_index(index_path: &Path) -> Result<AgentSessionIndex, AppError> {
    if !index_path.exists() {
        return Ok(AgentSessionIndex::default());
    }

    read_json(index_path).map_err(|err| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("failed to parse session index: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_INDEX_PARSE_FAILED",
            "path": index_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })
}

pub fn save_index(index_path: &Path, index: &AgentSessionIndex) -> Result<(), AppError> {
    if let Some(parent) = index_path.parent() {
        ensure_dir(parent)?;
    }

    atomic_write_json(index_path, index).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write session index: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_INDEX_WRITE_FAILED",
            "path": index_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })
}

pub fn upsert_meta(index: &mut AgentSessionIndex, meta: AgentSessionMeta) {
    if let Some(existing) = index
        .sessions
        .iter_mut()
        .find(|item| item.session_id == meta.session_id)
    {
        *existing = meta;
    } else {
        index.sessions.push(meta);
    }

    index.sessions.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
}

pub fn find_meta(index: &AgentSessionIndex, session_id: &str) -> Option<AgentSessionMeta> {
    index
        .sessions
        .iter()
        .find(|item| item.session_id == session_id)
        .cloned()
}

pub fn update_index_for_events(
    project_path: &Path,
    session_id: &str,
    events: &[AgentSessionEvent],
) -> Result<(), AppError> {
    let now = Utc::now().timestamp_millis();
    let index_path = super::session_index_path(project_path);
    let mut index = load_index(&index_path)?;

    let mut meta = find_meta(&index, session_id).unwrap_or_else(|| AgentSessionMeta {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        created_at: now,
        updated_at: now,
        title: None,
        last_turn: None,
        last_stop_reason: None,
        active_chapter_path: None,
        compaction_count: Some(0),
    });

    meta.updated_at = now;

    let mut last_turn = meta.last_turn;
    let mut last_stop_reason = meta.last_stop_reason.clone();
    let mut compaction_count = meta.compaction_count.unwrap_or(0);
    apply_events_to_meta(
        &mut meta,
        events,
        &mut last_turn,
        &mut last_stop_reason,
        &mut compaction_count,
    );

    meta.last_turn = last_turn;
    meta.last_stop_reason = last_stop_reason;
    meta.compaction_count = Some(compaction_count);

    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}

pub fn rebuild_index_for_session(
    project_path: &Path,
    session_id: &str,
    events: &[AgentSessionEvent],
) -> Result<(), AppError> {
    let now = Utc::now().timestamp_millis();
    let index_path = super::session_index_path(project_path);
    let mut index = load_index(&index_path)?;

    let existing = find_meta(&index, session_id);
    let created_at = existing.as_ref().map(|meta| meta.created_at).unwrap_or(now);
    let title = existing.as_ref().and_then(|meta| meta.title.clone());

    let mut meta = AgentSessionMeta {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        created_at,
        updated_at: now,
        title,
        last_turn: None,
        last_stop_reason: None,
        active_chapter_path: None,
        compaction_count: Some(0),
    };

    let mut last_turn = None;
    let mut last_stop_reason = None;
    let mut compaction_count = 0_i64;
    apply_events_to_meta(
        &mut meta,
        events,
        &mut last_turn,
        &mut last_stop_reason,
        &mut compaction_count,
    );
    meta.last_turn = last_turn;
    meta.last_stop_reason = last_stop_reason;
    meta.compaction_count = Some(compaction_count);

    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}

fn apply_events_to_meta(
    meta: &mut AgentSessionMeta,
    events: &[AgentSessionEvent],
    last_turn: &mut Option<i64>,
    last_stop_reason: &mut Option<String>,
    compaction_count: &mut i64,
) {
    for event in events {
        if let Some(turn) = event.turn {
            if last_turn.map_or(true, |current| turn > current) {
                *last_turn = Some(turn);
            }
        }

        match event.event_type.as_str() {
            "turn_completed" => {
                *last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("success".to_string()));
            }
            "turn_failed" => {
                *last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("error".to_string()));
            }
            "turn_cancelled" => {
                *last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("cancel".to_string()));
            }
            "compaction_summary" | "compaction_fallback" => {
                *compaction_count = (*compaction_count).saturating_add(1)
            }
            _ => {}
        }

        if let Some(payload) = &event.payload {
            if let Some(path) = payload
                .get("active_chapter_path")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            {
                meta.active_chapter_path = Some(path.to_string());
            }
        }
    }
}

fn stop_reason_from_payload(payload: Option<&serde_json::Value>) -> Option<String> {
    payload
        .and_then(|value| value.get("stop_reason"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .and_then(|value| match value {
            "success" | "cancel" | "error" | "limit" => Some(value.to_string()),
            _ => None,
        })
}
