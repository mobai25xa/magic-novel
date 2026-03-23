use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use chrono::Utc;

use crate::models::{AppError, ErrorCode};
use crate::services::ensure_dir;

use super::paths::{session_stream_path, sessions_root, INSPIRATION_SESSION_SCHEMA_VERSION};
use super::store::update_index_for_events;
use super::types::InspirationSessionEvent;

#[derive(Debug, Clone)]
pub struct AppendSessionEventsResult {
    pub appended_count: usize,
    pub deduped_count: usize,
    pub last_event_seq: i64,
}

fn stream_len(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn rollback_stream_append(path: &std::path::Path, original_len: u64) -> Result<(), AppError> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open inspiration stream for rollback: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_ROLLBACK_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    file.set_len(original_len).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to rollback inspiration stream append: {err}"),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_ROLLBACK_TRUNCATE_FAILED",
            "path": path.to_string_lossy(),
            "length": original_len,
        })),
        recoverable: Some(true),
    })
}

fn append_events_jsonl(
    path: &std::path::Path,
    events: &[InspirationSessionEvent],
) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open inspiration session stream: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_APPEND_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    for event in events {
        let line = serde_json::to_string(event).map_err(|err| AppError {
            code: ErrorCode::JsonParseError,
            message: format!("failed to serialize inspiration session event: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_APPEND_SERIALIZE_FAILED",
            })),
            recoverable: Some(false),
        })?;

        file.write_all(line.as_bytes()).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to write inspiration session event: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_APPEND_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
        file.write_all(b"\n").map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to flush inspiration session newline: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_APPEND_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
    }

    Ok(())
}

pub fn load_session_events(session_id: &str) -> Result<Vec<InspirationSessionEvent>, AppError> {
    let path = session_stream_path(session_id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to open inspiration session stream: {err}"),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_LOAD_OPEN_FAILED",
            "path": path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to read inspiration session stream line: {err}"),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_LOAD_READ_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<InspirationSessionEvent>(&line) {
            Ok(event) => events.push(event),
            Err(err) => {
                if !line.trim_end().ends_with('}') {
                    continue;
                }

                return Err(AppError {
                    code: ErrorCode::JsonParseError,
                    message: format!("failed to parse inspiration session stream line: {err}"),
                    details: Some(serde_json::json!({
                        "code": "E_INSPIRATION_SESSION_LOAD_PARSE_FAILED",
                        "line": line,
                    })),
                    recoverable: Some(true),
                });
            }
        }
    }

    Ok(events)
}

pub fn append_session_events(
    session_id: &str,
    events: &[InspirationSessionEvent],
) -> Result<AppendSessionEventsResult, AppError> {
    let root = sessions_root()?;
    ensure_dir(&root)?;
    let stream_path = session_stream_path(session_id)?;
    let existing = load_session_events(session_id)?;
    let mut seen_dedupe_keys = existing
        .iter()
        .filter_map(|event| event.dedupe_key.clone())
        .collect::<HashSet<_>>();
    let mut last_event_seq = existing
        .iter()
        .filter_map(|event| event.event_seq)
        .max()
        .unwrap_or(0);
    let mut prepared = Vec::new();
    let mut deduped_count = 0_usize;

    for event in events {
        if !event.validate_v1() {
            return Err(AppError {
                code: ErrorCode::SchemaValidationError,
                message: "invalid inspiration session event schema".to_string(),
                details: Some(serde_json::json!({
                    "code": "E_INSPIRATION_SESSION_EVENT_INVALID",
                    "schema_version": INSPIRATION_SESSION_SCHEMA_VERSION,
                })),
                recoverable: Some(true),
            });
        }

        let mut prepared_event = event.clone();
        prepared_event.schema_version = INSPIRATION_SESSION_SCHEMA_VERSION;
        prepared_event.session_id = session_id.to_string();
        prepared_event.ts = prepared_event.ts.max(Utc::now().timestamp_millis());
        prepared_event.dedupe_key = prepared_event
            .dedupe_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        if let Some(key) = prepared_event.dedupe_key.clone() {
            if seen_dedupe_keys.contains(&key) {
                deduped_count = deduped_count.saturating_add(1);
                continue;
            }
            seen_dedupe_keys.insert(key);
        }

        last_event_seq = last_event_seq.saturating_add(1);
        prepared_event.event_seq = Some(last_event_seq);
        if prepared_event.event_id.is_none() {
            prepared_event.event_id = Some(format!("evt_{}", uuid::Uuid::new_v4()));
        }
        prepared.push(prepared_event);
    }

    if prepared.is_empty() {
        return Ok(AppendSessionEventsResult {
            appended_count: 0,
            deduped_count,
            last_event_seq,
        });
    }

    let original_stream_len = stream_len(&stream_path);
    append_events_jsonl(&stream_path, &prepared)?;

    if let Err(err) = update_index_for_events(session_id, &prepared) {
        let rollback_result = rollback_stream_append(&stream_path, original_stream_len);
        return Err(AppError {
            code: err.code,
            message: "failed to sync inspiration session index after append".to_string(),
            details: Some(serde_json::json!({
                "code": "E_INSPIRATION_SESSION_APPEND_INDEX_SYNC_FAILED",
                "session_id": session_id,
                "stream_path": stream_path.to_string_lossy(),
                "appended_count": prepared.len(),
                "deduped_count": deduped_count,
                "stream_rollback_succeeded": rollback_result.is_ok(),
                "stream_rollback_error": rollback_result.err(),
            })),
            recoverable: Some(true),
        });
    }

    Ok(AppendSessionEventsResult {
        appended_count: prepared.len(),
        deduped_count,
        last_event_seq,
    })
}
