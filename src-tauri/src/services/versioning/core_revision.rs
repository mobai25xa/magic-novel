use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppError;
use crate::utils::atomic_write::atomic_write;

use super::core_layout::{sanitize_entity_id, REVISIONS_DIR};
use super::core_utils::app_err_vc;

pub(crate) fn write_entity_json_atomic(
    entity_path: &Path,
    json: &serde_json::Value,
) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(json).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to serialize entity json: {e}"),
            true,
        )
    })?;

    if let Some(parent) = entity_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    atomic_write(entity_path, &content)
}

fn revision_entity_dir(vc_root: &Path, entity_id: &str) -> PathBuf {
    vc_root
        .join(REVISIONS_DIR)
        .join(sanitize_entity_id(entity_id))
}

fn revision_file_path(vc_root: &Path, entity_id: &str, revision: i64) -> PathBuf {
    revision_entity_dir(vc_root, entity_id).join(format!("rev_{:09}.json", revision))
}

pub(crate) fn write_revision_record(
    vc_root: &Path,
    entity_id: &str,
    revision: i64,
    json: &serde_json::Value,
    json_hash: &str,
) -> Result<(), AppError> {
    let dir = revision_entity_dir(vc_root, entity_id);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = revision_file_path(vc_root, entity_id, revision);
    let record = serde_json::json!({
        "entity_id": entity_id,
        "revision": revision,
        "json_hash": json_hash,
        "ts": chrono::Utc::now().timestamp_millis(),
        "content": json,
    });

    let content = serde_json::to_string_pretty(&record).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to serialize revision record: {e}"),
            true,
        )
    })?;

    atomic_write(&path, &content)
}

pub(crate) fn read_revision_record(
    vc_root: &Path,
    entity_id: &str,
    revision: i64,
) -> Result<Option<serde_json::Value>, AppError> {
    let path = revision_file_path(vc_root, entity_id, revision);
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to parse revision record: {e}"),
            true,
        )
    })?;

    Ok(value.get("content").cloned())
}
