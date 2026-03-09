use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::models::AppError;

use super::core_types::{CallIndexRecord, WalRecord, WalType};
use super::core_utils::app_err_vc;

pub(crate) fn append_call_index(path: &Path, rec: &CallIndexRecord) -> Result<(), AppError> {
    let line = serde_json::to_string(rec).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to serialize call index record: {e}"),
            true,
        )
    })?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| app_err_vc("E_VC_IO_WRITE_FAIL", e.to_string(), true))?;

    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub(crate) fn find_call_index_by_call_id(
    path: &Path,
    call_id: &str,
) -> Result<Option<CallIndexRecord>, AppError> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<CallIndexRecord>(line) {
            if rec.call_id == call_id {
                return Ok(Some(rec));
            }
        }
    }

    Ok(None)
}

pub(crate) fn find_call_index_by_call_id_entity(
    path: &Path,
    call_id: &str,
    entity_id: &str,
) -> Result<Option<CallIndexRecord>, AppError> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<CallIndexRecord>(line) {
            if rec.call_id == call_id && rec.entity_id == entity_id {
                return Ok(Some(rec));
            }
        }
    }

    Ok(None)
}

pub(crate) fn find_entity_id_by_call_id(
    path: &Path,
    call_id: &str,
) -> Result<Option<String>, AppError> {
    Ok(find_call_index_by_call_id(path, call_id)?.map(|r| r.entity_id))
}

pub(crate) fn find_commit_after_hash_by_tx_id(
    path: &Path,
    tx_id: &str,
) -> Result<Option<String>, AppError> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_str::<WalRecord>(line) else {
            continue;
        };
        if rec.r#type == WalType::Commit && rec.tx_id == tx_id {
            return Ok(rec.after_hash);
        }
    }

    Ok(None)
}
