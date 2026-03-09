use std::fs::OpenOptions;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::models::{AppError, ErrorCode};

use super::recovery_support::{compute_truncated_bytes, parse_recovery_reader};

fn open_recovery_file(path: &Path) -> Result<std::fs::File, AppError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to open stream for recovery: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RECOVER_OPEN_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })
}

fn stream_len(file: &std::fs::File, path: &Path) -> Result<u64, AppError> {
    file.metadata()
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to stat stream during recovery: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RECOVER_STAT_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })
        .map(|value| value.len())
}

fn truncate_stream(file: &mut std::fs::File, path: &Path, valid_end: u64) -> Result<(), AppError> {
    file.seek(SeekFrom::Start(valid_end))
        .map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to seek stream during recovery: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RECOVER_SEEK_FAILED",
                "path": path.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

    file.set_len(valid_end).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to truncate stream during recovery: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_RECOVER_TRUNCATE_FAILED",
            "path": path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })
}

pub fn recover_stream_file(path: &Path) -> Result<(i64, Option<String>), AppError> {
    if !path.exists() {
        return Ok((0, None));
    }

    let file = open_recovery_file(path)?;
    let mut reader = BufReader::new(file);
    let (valid_end, truncated_reason) = parse_recovery_reader(&mut reader)?;

    let mut file = reader.into_inner();
    let current_len = stream_len(&file, path)?;
    let truncated = compute_truncated_bytes(current_len, valid_end, truncated_reason.is_some());

    if truncated == 0 {
        return Ok((0, None));
    }

    truncate_stream(&mut file, path, valid_end)?;
    Ok((truncated, truncated_reason))
}
