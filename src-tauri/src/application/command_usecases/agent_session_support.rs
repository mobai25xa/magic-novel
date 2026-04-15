use crate::models::{AppError, ErrorCode};
use crate::services::{
    append_events_jsonl, delete_runtime_snapshot, ensure_dir, find_meta, load_index,
    load_runtime_snapshot, lock_session_index, read_events_jsonl, recover_stream_file, save_index,
    session_index_path, session_settings_path, session_stream_path, sessions_root, upsert_meta,
    AgentSessionEvent, AgentSessionIndex, AgentSessionMeta, AgentSessionSettings,
    AGENT_SESSION_SCHEMA_VERSION,
};
use crate::utils::atomic_write::atomic_write_json;
use chrono::Utc;
use std::path::{Path, PathBuf};
pub fn resolve_project_path(project_path: &str) -> Result<PathBuf, AppError> {
    if project_path.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "project_path is required".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_INVALID_PROJECT_PATH",
            })),
            recoverable: Some(true),
        });
    }
    let path = PathBuf::from(project_path);
    if !path.exists() || !path.is_dir() {
        return Err(AppError {
            code: ErrorCode::NotFound,
            message: "project_path not found".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_PROJECT_PATH_NOT_FOUND",
                "project_path": project_path,
            })),
            recoverable: Some(false),
        });
    }
    Ok(path)
}
pub fn ensure_session_exists(project_path: &Path, session_id: &str) -> Result<(), AppError> {
    let stream_path = session_stream_path(project_path, session_id);
    if stream_path.exists() {
        return Ok(());
    }
    Err(AppError {
        code: ErrorCode::NotFound,
        message: "session stream not found".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_NOT_FOUND",
            "session_id": session_id,
        })),
        recoverable: Some(false),
    })
}
pub fn create_session_meta(
    session_id: String,
    now: i64,
    title: Option<String>,
    active_chapter_path: Option<String>,
) -> AgentSessionMeta {
    AgentSessionMeta {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id,
        created_at: now,
        updated_at: now,
        title,
        last_turn: None,
        last_stop_reason: None,
        active_chapter_path,
        compaction_count: Some(0),
    }
}
pub fn normalize_title(title: Option<String>) -> Option<String> {
    title.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
pub fn normalize_active_chapter(path: Option<String>) -> Option<String> {
    path.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn apply_events_to_recovered_meta(meta: &mut AgentSessionMeta, events: &[AgentSessionEvent]) {
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
            "turn_completed" => {
                last_stop_reason = event
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("stop_reason"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .and_then(|value| match value {
                        "success" | "cancel" | "error" | "limit" => Some(value.to_string()),
                        _ => None,
                    })
                    .or_else(|| Some("success".to_string()));
            }
            "turn_failed" => {
                last_stop_reason = event
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("stop_reason"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .and_then(|value| match value {
                        "success" | "cancel" | "error" | "limit" => Some(value.to_string()),
                        _ => None,
                    })
                    .or_else(|| Some("error".to_string()));
            }
            "turn_cancelled" => {
                last_stop_reason = event
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("stop_reason"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .and_then(|value| match value {
                        "success" | "cancel" | "error" | "limit" => Some(value.to_string()),
                        _ => None,
                    })
                    .or_else(|| Some("cancel".to_string()));
            }
            "compaction_summary" | "compaction_fallback" => {
                compaction_count = compaction_count.saturating_add(1);
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

    meta.last_turn = last_turn;
    meta.last_stop_reason = last_stop_reason;
    meta.compaction_count = Some(compaction_count);
}

fn build_recovered_meta(
    project_path: &Path,
    session_id: &str,
) -> Result<Option<AgentSessionMeta>, AppError> {
    let runtime_snapshot = load_runtime_snapshot(project_path, session_id)?;
    let events = load_session_events(project_path, session_id)?;

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

    let mut meta = AgentSessionMeta {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        created_at,
        updated_at,
        title: None,
        last_turn: runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.last_turn.map(i64::from)),
        last_stop_reason: None,
        active_chapter_path: None,
        compaction_count: Some(0),
    };

    apply_events_to_recovered_meta(&mut meta, &events);
    Ok(Some(meta))
}

fn load_or_repair_session_meta_locked(
    project_path: &Path,
    index_path: &Path,
    index: &mut AgentSessionIndex,
    session_id: &str,
) -> Result<Option<AgentSessionMeta>, AppError> {
    if let Some(meta) = find_meta(index, session_id) {
        return Ok(Some(meta));
    }

    let Some(recovered) = build_recovered_meta(project_path, session_id)? else {
        return Ok(None);
    };

    tracing::warn!(
        target: "agent_session",
        project_path = %project_path.display(),
        session_id = %session_id,
        "recovered agent session metadata from persisted files"
    );
    upsert_meta(index, recovered.clone());
    save_index(index_path, index)?;

    Ok(Some(recovered))
}
pub fn create_session_start_event(
    session_id: &str,
    now: i64,
    project_path: &str,
    active_chapter_path: Option<&str>,
) -> AgentSessionEvent {
    AgentSessionEvent {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        event_type: "session_start".to_string(),
        session_id: session_id.to_string(),
        ts: now,
        event_id: Some(format!("evt_start_{}_{}", now, session_id)),
        event_seq: Some(1),
        dedupe_key: Some("session_start".to_string()),
        turn: None,
        payload: Some(serde_json::json!({
            "project_path": project_path,
            "active_chapter_path": active_chapter_path,
        })),
    }
}
pub fn create_default_settings(session_id: &str) -> AgentSessionSettings {
    AgentSessionSettings {
        session_id: session_id.to_string(),
        ..AgentSessionSettings::default()
    }
}
pub fn create_session_files(
    project_path: &Path,
    session_id: &str,
    start_event: &AgentSessionEvent,
) -> Result<(), AppError> {
    let root = sessions_root(project_path);
    ensure_dir(&root)?;
    let stream_path = session_stream_path(project_path, session_id);
    append_events_jsonl(&stream_path, std::slice::from_ref(start_event))?;
    let settings_path = session_settings_path(project_path, session_id);
    let settings = create_default_settings(session_id);
    atomic_write_json(&settings_path, &settings).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write session settings: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_SETTINGS_WRITE_FAILED",
            "session_id": session_id,
        })),
        recoverable: Some(true),
    })
}
pub fn load_session_index(project_path: &Path) -> Result<(PathBuf, AgentSessionIndex), AppError> {
    let index_path = session_index_path(project_path);
    let index = load_index(&index_path)?;
    Ok((index_path, index))
}
pub fn save_session_meta(project_path: &Path, meta: AgentSessionMeta) -> Result<(), AppError> {
    let _guard = lock_session_index();
    let (index_path, mut index) = load_session_index(project_path)?;
    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}
pub fn ensure_meta_exists(project_path: &Path, session_id: &str) -> Result<(), AppError> {
    if load_session_meta(project_path, session_id)?.is_some() {
        return Ok(());
    }
    Err(AppError {
        code: ErrorCode::NotFound,
        message: "session metadata not found".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_NOT_FOUND",
            "session_id": session_id,
        })),
        recoverable: Some(false),
    })
}
pub fn load_session_meta(
    project_path: &Path,
    session_id: &str,
) -> Result<Option<AgentSessionMeta>, AppError> {
    let _guard = lock_session_index();
    let (index_path, mut index) = load_session_index(project_path)?;
    load_or_repair_session_meta_locked(project_path, &index_path, &mut index, session_id)
}
pub fn list_session_metas(
    project_path: &Path,
    limit: Option<usize>,
) -> Result<Vec<AgentSessionMeta>, AppError> {
    let _guard = lock_session_index();
    let (_, mut index) = load_session_index(project_path)?;
    index
        .sessions
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    if let Some(limit) = limit {
        index.sessions.truncate(limit);
    }
    Ok(index.sessions)
}
fn session_not_found_error(session_id: &str, message: &str) -> AppError {
    AppError {
        code: ErrorCode::NotFound,
        message: message.to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_NOT_FOUND",
            "session_id": session_id,
        })),
        recoverable: Some(false),
    }
}
fn recover_list_error(root: &Path, err: std::io::Error) -> AppError {
    AppError {
        code: ErrorCode::IoError,
        message: format!("failed to list session stream directory: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RECOVER_LIST_FAILED",
            "path": root.to_string_lossy(),
        })),
        recoverable: Some(true),
    }
}
pub fn load_session_events(
    project_path: &Path,
    session_id: &str,
) -> Result<Vec<AgentSessionEvent>, AppError> {
    let stream_path = session_stream_path(project_path, session_id);
    read_events_jsonl(&stream_path)
}
pub fn update_session_meta(
    project_path: &Path,
    session_id: &str,
    title: Option<String>,
    active_chapter_path: Option<String>,
) -> Result<(), AppError> {
    let now = Utc::now().timestamp_millis();
    let _guard = lock_session_index();
    let (index_path, mut index) = load_session_index(project_path)?;
    let mut meta =
        load_or_repair_session_meta_locked(project_path, &index_path, &mut index, session_id)?
            .ok_or_else(|| session_not_found_error(session_id, "session metadata not found"))?;
    if let Some(value) = title {
        meta.title = normalize_title(Some(value));
    }
    if let Some(value) = active_chapter_path {
        meta.active_chapter_path = normalize_active_chapter(Some(value));
    }
    meta.updated_at = now;
    upsert_meta(&mut index, meta);
    save_index(&index_path, &index)
}
pub fn delete_session(project_path: &Path, session_id: &str) -> Result<(), AppError> {
    let stream_path = session_stream_path(project_path, session_id);
    if stream_path.exists() {
        std::fs::remove_file(&stream_path).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to delete session stream: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_DELETE_STREAM_FAILED",
                "session_id": session_id,
                "path": stream_path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
    }
    let settings_path = session_settings_path(project_path, session_id);
    if settings_path.exists() {
        std::fs::remove_file(&settings_path).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to delete session settings: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_DELETE_SETTINGS_FAILED",
                "session_id": session_id,
                "path": settings_path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;
    }

    delete_runtime_snapshot(project_path, session_id)?;

    let _guard = lock_session_index();
    let (index_path, mut index) = load_session_index(project_path)?;
    let original_len = index.sessions.len();
    index.sessions.retain(|meta| meta.session_id != session_id);
    if index.sessions.len() != original_len {
        save_index(&index_path, &index)?;
    }
    Ok(())
}
pub fn recover_sessions(
    project_path: &Path,
    session_id: Option<&str>,
) -> Result<SessionRecoverySummary, AppError> {
    let root = sessions_root(project_path);
    ensure_dir(&root)?;
    let mut targets = Vec::new();
    if let Some(session_id) = session_id {
        targets.push(session_stream_path(project_path, session_id));
    } else {
        for entry in std::fs::read_dir(&root).map_err(|err| recover_list_error(&root, err))? {
            let entry = entry.map_err(|err| recover_list_error(&root, err))?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                targets.push(path);
            }
        }
    }
    let mut repaired_files = 0_i64;
    let mut truncated_bytes = 0_i64;
    let mut notes = Vec::new();
    let mut quarantined_sessions = Vec::new();
    let mut manual_repair_actions = Vec::new();
    for path in targets {
        let (truncated, reason) = recover_stream_file(&path)?;
        if truncated > 0 {
            let session_name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown")
                .to_string();
            repaired_files += 1;
            truncated_bytes += truncated;
            notes.push(format!(
                "{} truncated {} bytes{}",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("unknown"),
                truncated,
                reason
                    .as_ref()
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default()
            ));

            if let Some(reason) = reason {
                quarantined_sessions.push(session_name.clone());
                manual_repair_actions.push(format!(
                    "{session_name}: verify recovered stream consistency and replay state before resuming traffic ({reason})"
                ));
            }
        }
    }
    Ok(SessionRecoverySummary {
        repaired_files,
        truncated_bytes,
        notes,
        quarantined_sessions,
        manual_repair_actions,
    })
}

#[derive(Debug, Clone)]
pub struct SessionRecoverySummary {
    pub repaired_files: i64,
    pub truncated_bytes: i64,
    pub notes: Vec<String>,
    pub quarantined_sessions: Vec<String>,
    pub manual_repair_actions: Vec<String>,
}
