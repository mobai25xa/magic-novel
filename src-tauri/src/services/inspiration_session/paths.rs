use std::path::PathBuf;

use crate::models::{AppError, ErrorCode};

const INSPIRATION_ROOT_OVERRIDE_ENV: &str = "MAGIC_NOVEL_INSPIRATION_ROOT";
const MAGIC_NOVEL_DIR: &str = "magic_novel";
const INSPIRATION_DIR: &str = "inspiration";
const SESSIONS_DIR: &str = "sessions";
const INDEX_FILE: &str = "index.json";
const STREAM_SUFFIX: &str = ".jsonl";
const RUNTIME_SUFFIX: &str = ".runtime.json";

pub const INSPIRATION_SESSION_SCHEMA_VERSION: i32 = 1;
pub const INSPIRATION_RUNTIME_SNAPSHOT_SCHEMA_VERSION: i32 = 1;

fn normalize_override_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

pub fn inspiration_root() -> Result<PathBuf, AppError> {
    if let Ok(raw) = std::env::var(INSPIRATION_ROOT_OVERRIDE_ENV) {
        if let Some(path) = normalize_override_path(&raw) {
            return Ok(path);
        }
    }

    let home = dirs::home_dir().ok_or_else(|| AppError {
        code: ErrorCode::Internal,
        message: "Cannot locate home directory".to_string(),
        details: Some(serde_json::json!({
            "code": "E_INSPIRATION_SESSION_HOME_NOT_FOUND",
        })),
        recoverable: Some(false),
    })?;

    Ok(home
        .join(".magic")
        .join(MAGIC_NOVEL_DIR)
        .join(INSPIRATION_DIR))
}

pub fn sessions_root() -> Result<PathBuf, AppError> {
    Ok(inspiration_root()?.join(SESSIONS_DIR))
}

pub fn session_stream_path(session_id: &str) -> Result<PathBuf, AppError> {
    Ok(sessions_root()?.join(format!("{session_id}{STREAM_SUFFIX}")))
}

pub fn session_runtime_path(session_id: &str) -> Result<PathBuf, AppError> {
    Ok(sessions_root()?.join(format!("{session_id}{RUNTIME_SUFFIX}")))
}

pub fn session_index_path() -> Result<PathBuf, AppError> {
    Ok(inspiration_root()?.join(INDEX_FILE))
}
