use std::sync::{Mutex, MutexGuard, OnceLock};

use chrono::Utc;

use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::paths::{session_index_path, INSPIRATION_SESSION_SCHEMA_VERSION};
use super::runtime_snapshot::load_runtime_snapshot;
use super::stream::load_session_events;
use super::types::{
    session_event_types, InspirationSessionEvent, InspirationSessionIndex, InspirationSessionMeta,
};

const SESSION_LIST_HARD_LIMIT: usize = 200;

fn index_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_index() -> MutexGuard<'static, ()> {
    index_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn normalize_title(title: Option<String>) -> Option<String> {
    title.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn load_index() -> Result<(std::path::PathBuf, InspirationSessionIndex), AppError> {
    let index_path = session_index_path()?;
    if !index_path.exists() {
        return Ok((index_path, InspirationSessionIndex::default()));
    }

    let index = read_json(&index_path).map_err(|err| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("failed to parse inspiration session index: {err}"),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_INDEX_PARSE_FAILED",
            "path": index_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    Ok((index_path, index))
}

fn save_index(
    index_path: &std::path::Path,
    index: &InspirationSessionIndex,
) -> Result<(), AppError> {
    if let Some(parent) = index_path.parent() {
        ensure_dir(parent)?;
    }

    atomic_write_json(index_path, index).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write inspiration session index: {err}"),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_INDEX_WRITE_FAILED",
            "path": index_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })
}

pub fn upsert_meta(index: &mut InspirationSessionIndex, meta: InspirationSessionMeta) {
    if let Some(existing) = index
        .sessions
        .iter_mut()
        .find(|item| item.session_id == meta.session_id)
    {
        *existing = meta;
    } else {
        index.sessions.push(meta);
    }

    index.sessions.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
}

fn apply_events_to_meta(meta: &mut InspirationSessionMeta, events: &[InspirationSessionEvent]) {
    let mut last_turn = meta.last_turn;
    let mut last_stop_reason = meta.last_stop_reason.clone();
    let mut compaction_count = meta.compaction_count.unwrap_or(0);

    for event in events {
        if let Some(turn) = event.turn {
            if last_turn.map_or(true, |value| turn > value) {
                last_turn = Some(turn);
            }
        }

        meta.updated_at = meta.updated_at.max(event.ts);

        match event.event_type.as_str() {
            session_event_types::TURN_COMPLETED => {
                last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("success".to_string()));
            }
            session_event_types::TURN_FAILED => {
                last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("error".to_string()));
            }
            session_event_types::TURN_CANCELLED => {
                last_stop_reason = stop_reason_from_payload(event.payload.as_ref())
                    .or_else(|| Some("cancel".to_string()));
            }
            session_event_types::COMPACTION_STARTED
            | session_event_types::COMPACTION_FINISHED
            | session_event_types::COMPACTION_FALLBACK
            | session_event_types::COMPACTION_SUMMARY => {
                compaction_count = compaction_count.saturating_add(1);
            }
            _ => {}
        }
    }

    meta.last_turn = last_turn;
    meta.last_stop_reason = last_stop_reason;
    meta.compaction_count = Some(compaction_count);
}

fn build_recovered_meta(session_id: &str) -> Result<Option<InspirationSessionMeta>, AppError> {
    let runtime_snapshot = load_runtime_snapshot(session_id)?;
    let events = load_session_events(session_id)?;

    if runtime_snapshot.is_none() && events.is_empty() {
        return Ok(None);
    }

    let now = Utc::now().timestamp_millis();
    let first_event_ts = events.iter().map(|event| event.ts).min();
    let last_event_ts = events.iter().map(|event| event.ts).max();
    let snapshot_updated_at = runtime_snapshot
        .as_ref()
        .map(|snapshot| snapshot.updated_at);

    let created_at = first_event_ts.or(snapshot_updated_at).unwrap_or(now);
    let updated_at = last_event_ts
        .or(snapshot_updated_at)
        .unwrap_or(created_at)
        .max(created_at);

    let mut meta = InspirationSessionMeta {
        schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        created_at,
        updated_at,
        title: None,
        last_turn: runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.last_turn.map(i64::from)),
        last_stop_reason: None,
        compaction_count: Some(0),
    };

    apply_events_to_meta(&mut meta, &events);
    Ok(Some(meta))
}

fn load_or_repair_meta_locked(
    index_path: &std::path::Path,
    index: &mut InspirationSessionIndex,
    session_id: &str,
) -> Result<Option<InspirationSessionMeta>, AppError> {
    if let Some(meta) = find_meta(index, session_id) {
        return Ok(Some(meta));
    }

    let Some(recovered) = build_recovered_meta(session_id)? else {
        return Ok(None);
    };

    tracing::warn!(
        target: "inspiration",
        session_id = %session_id,
        "recovered inspiration session metadata from persisted files"
    );
    upsert_meta(index, recovered.clone());
    save_index(index_path, index)?;

    Ok(Some(recovered))
}

pub fn find_meta(
    index: &InspirationSessionIndex,
    session_id: &str,
) -> Option<InspirationSessionMeta> {
    index
        .sessions
        .iter()
        .find(|item| item.session_id == session_id)
        .cloned()
}

pub fn save_session_meta(meta: InspirationSessionMeta) -> Result<(), AppError> {
    let _guard = lock_index();
    let (index_path, mut index) = load_index()?;
    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}

pub fn list_session_meta(limit: Option<i64>) -> Result<Vec<InspirationSessionMeta>, AppError> {
    let _guard = lock_index();
    let (_, mut index) = load_index()?;
    index.sessions.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });

    let capped_limit = match limit {
        Some(value) if value <= 0 => return Ok(Vec::new()),
        Some(value) => usize::try_from(value)
            .unwrap_or(usize::MAX)
            .min(SESSION_LIST_HARD_LIMIT),
        None => SESSION_LIST_HARD_LIMIT,
    };

    if index.sessions.len() > capped_limit {
        index.sessions.truncate(capped_limit);
    }

    Ok(index.sessions)
}

pub fn update_session_meta_title(
    session_id: &str,
    title: Option<String>,
) -> Result<InspirationSessionMeta, AppError> {
    let now = Utc::now().timestamp_millis();
    let _guard = lock_index();
    let (index_path, mut index) = load_index()?;
    let mut updated =
        load_or_repair_meta_locked(&index_path, &mut index, session_id)?.ok_or_else(|| {
            AppError {
                code: ErrorCode::NotFound,
                message: "inspiration session metadata not found".to_string(),
                details: Some(serde_json::json!({
                    "code": "E_INSPIRATION_SESSION_NOT_FOUND",
                    "session_id": session_id,
                })),
                recoverable: Some(false),
            }
        })?;

    if let Some(title_value) = title {
        updated.title = normalize_title(Some(title_value));
    }
    updated.updated_at = now;
    upsert_meta(&mut index, updated.clone());
    save_index(&index_path, &index)?;

    Ok(updated)
}

pub fn remove_session_meta(session_id: &str) -> Result<(), AppError> {
    let _guard = lock_index();
    let (index_path, mut index) = load_index()?;
    let before_count = index.sessions.len();
    index.sessions.retain(|item| item.session_id != session_id);

    if index.sessions.len() == before_count {
        return Ok(());
    }

    save_index(&index_path, &index)
}

pub fn load_session_meta(session_id: &str) -> Result<Option<InspirationSessionMeta>, AppError> {
    let _guard = lock_index();
    let (index_path, mut index) = load_index()?;
    load_or_repair_meta_locked(&index_path, &mut index, session_id)
}

pub fn ensure_meta_exists(session_id: &str) -> Result<(), AppError> {
    if load_session_meta(session_id)?.is_some() {
        return Ok(());
    }

    Err(AppError {
        code: ErrorCode::NotFound,
        message: "inspiration session metadata not found".to_string(),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_NOT_FOUND",
            "session_id": session_id,
        })),
        recoverable: Some(false),
    })
}

pub fn update_index_for_events(
    session_id: &str,
    events: &[InspirationSessionEvent],
) -> Result<(), AppError> {
    let now = Utc::now().timestamp_millis();
    let _guard = lock_index();
    let (index_path, mut index) = load_index()?;
    let mut meta =
        load_or_repair_meta_locked(&index_path, &mut index, session_id)?.unwrap_or_else(|| {
            InspirationSessionMeta {
                schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
                session_id: session_id.to_string(),
                created_at: now,
                updated_at: now,
                title: None,
                last_turn: None,
                last_stop_reason: None,
                compaction_count: Some(0),
            }
        });

    meta.updated_at = now;
    apply_events_to_meta(&mut meta, events);

    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}

fn stop_reason_from_payload(payload: Option<&serde_json::Value>) -> Option<String> {
    payload
        .and_then(|value| value.get("stop_reason"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| matches!(*value, "success" | "cancel" | "error" | "limit"))
        .map(|value| value.to_string())
}
