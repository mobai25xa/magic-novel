use std::io::BufRead;

use crate::models::{AppError, ErrorCode};

pub fn parse_recovery_reader<R: BufRead>(
    reader: &mut R,
) -> Result<(u64, Option<String>), AppError> {
    let mut valid_end: u64 = 0;
    let mut offset: u64 = 0;
    let mut truncated_reason: Option<String> = None;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to read stream during recovery: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_RECOVER_READ_FAILED",
            })),
            recoverable: Some(true),
        })?;

        if bytes == 0 {
            break;
        }

        offset += bytes as u64;

        if line.trim().is_empty() || serde_json::from_str::<serde_json::Value>(&line).is_ok() {
            valid_end = offset;
            continue;
        }

        truncated_reason = Some(format!("invalid json line at byte {}", valid_end));
        break;
    }

    Ok((valid_end, truncated_reason))
}

pub fn compute_truncated_bytes(current_len: u64, valid_end: u64, has_error: bool) -> i64 {
    if !has_error || valid_end >= current_len {
        return 0;
    }

    (current_len - valid_end) as i64
}
