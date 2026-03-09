use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::models::{AppError, ErrorCode};

use super::contract::{prepare_events_for_append, EventContractState};
use super::{read_and_migrate, session_stream_path, update_index_for_events, AgentSessionEvent};

#[derive(Debug, Clone)]
pub struct AppendSessionEventsResult {
    pub appended_count: usize,
    pub deduped_count: usize,
    pub last_event_seq: i64,
}

pub fn append_events_jsonl(path: &Path, events: &[AgentSessionEvent]) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open session stream: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    for event in events {
        let line = serde_json::to_string(event).map_err(|err| AppError {
            code: ErrorCode::JsonParseError,
            message: format!("failed to serialize session event: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_SERIALIZE_FAILED",
            })),
            recoverable: Some(false),
        })?;

        file.write_all(line.as_bytes()).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to write session event: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

        file.write_all(b"\n").map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to flush session newline: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
    }

    Ok(())
}

pub fn write_events_jsonl(path: &Path, events: &[AgentSessionEvent]) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open session stream for rewrite: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_REWRITE_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    for event in events {
        let line = serde_json::to_string(event).map_err(|err| AppError {
            code: ErrorCode::JsonParseError,
            message: format!("failed to serialize session event during rewrite: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_REWRITE_SERIALIZE_FAILED",
            })),
            recoverable: Some(false),
        })?;

        file.write_all(line.as_bytes()).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to write session event during rewrite: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_REWRITE_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

        file.write_all(b"\n").map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to write session newline during rewrite: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_REWRITE_WRITE_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
    }

    Ok(())
}

pub fn append_session_events(
    project_path: &Path,
    session_id: &str,
    events: &[AgentSessionEvent],
) -> Result<AppendSessionEventsResult, AppError> {
    if events.is_empty() {
        let stream_path = session_stream_path(project_path, session_id);
        let existing = read_and_migrate(&stream_path)?;
        let contract_state = EventContractState::from_existing(&existing);
        return Ok(AppendSessionEventsResult {
            appended_count: 0,
            deduped_count: 0,
            last_event_seq: contract_state.last_event_seq,
        });
    }

    let stream_path = session_stream_path(project_path, session_id);
    let existing = read_and_migrate(&stream_path)?;
    let contract_state = EventContractState::from_existing(&existing);
    let prepared = prepare_events_for_append(session_id, events, contract_state)?;

    if prepared.events.is_empty() {
        return Ok(AppendSessionEventsResult {
            appended_count: 0,
            deduped_count: prepared.deduped_count,
            last_event_seq: prepared.last_event_seq,
        });
    }

    let original_stream_len = stream_len(&stream_path);
    append_events_jsonl(&stream_path, &prepared.events)?;

    if let Err(index_err) = update_index_for_events(project_path, session_id, &prepared.events) {
        let rollback_result = rollback_stream_append(&stream_path, original_stream_len);
        return Err(AppError {
            code: index_err.code.clone(),
            message: "failed to sync session index after append".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_INDEX_SYNC_FAILED",
                "session_id": session_id,
                "stream_path": stream_path.to_string_lossy(),
                "appended_count": prepared.events.len(),
                "deduped_count": prepared.deduped_count,
                "index_error": index_err,
                "stream_rollback_succeeded": rollback_result.is_ok(),
                "stream_rollback_error": rollback_result.err(),
            })),
            recoverable: Some(true),
        });
    }

    Ok(AppendSessionEventsResult {
        appended_count: prepared.events.len(),
        deduped_count: prepared.deduped_count,
        last_event_seq: prepared.last_event_seq,
    })
}

pub fn read_events_jsonl(path: &Path) -> Result<Vec<AgentSessionEvent>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to open session stream: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_LOAD_OPEN_FAILED",
            "path": path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to read session stream line: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_LOAD_READ_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<AgentSessionEvent>(&line) {
            Ok(event) => events.push(event),
            Err(err) => {
                let line_trimmed = line.trim_end();
                let looks_like_trailing_partial = !line_trimmed.ends_with('}');
                if looks_like_trailing_partial {
                    continue;
                }

                return Err(AppError {
                    code: ErrorCode::JsonParseError,
                    message: format!("failed to parse session stream line: {err}"),
                    details: Some(serde_json::json!({
                        "code": "E_AGENT_SESSION_LOAD_PARSE_FAILED",
                        "line": line,
                    })),
                    recoverable: Some(true),
                });
            }
        }
    }

    Ok(events)
}

fn stream_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn rollback_stream_append(path: &Path, original_stream_len: u64) -> Result<(), AppError> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open stream for rollback: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_APPEND_ROLLBACK_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    file.set_len(original_stream_len).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to rollback session stream append: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_APPEND_ROLLBACK_TRUNCATE_FAILED",
            "path": path.to_string_lossy(),
            "length": original_stream_len,
        })),
        recoverable: Some(true),
    })
}
