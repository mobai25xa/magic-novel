use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::models::AppError;
use crate::services::versioning_port::EntityHead;
use crate::utils::atomic_write::atomic_write;

use super::core_layout::{entity_path_from_entity_id, HEAD_FILE};
use super::core_types::{HeadState, WalRecord};
use super::core_utils::{app_err_vc, compute_json_hash};

pub(crate) fn load_or_init_head_state(
    vc_root: &Path,
    project_path: &str,
) -> Result<HeadState, AppError> {
    let head_path = vc_root.join(HEAD_FILE);
    if !head_path.exists() {
        let st = HeadState {
            project_path: project_path.to_string(),
            last_event_seq: 0,
            entities: std::collections::HashMap::new(),
        };
        write_head_state_atomic(&head_path, &st)?;
        return Ok(st);
    }

    let content = fs::read_to_string(&head_path)?;
    let st: HeadState = serde_json::from_str(&content).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to parse head.json: {e}"),
            true,
        )
    })?;
    Ok(st)
}

pub(crate) fn write_head_state_atomic(head_path: &Path, state: &HeadState) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(state).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to serialize head.json: {e}"),
            true,
        )
    })?;

    atomic_write(head_path, &content)
}

pub(crate) fn next_event_seq(vc_root: &Path, project_path: &str) -> Result<i64, AppError> {
    let mut head_state = load_or_init_head_state(vc_root, project_path)?;
    head_state.last_event_seq += 1;
    write_head_state_atomic(&vc_root.join(HEAD_FILE), &head_state)?;
    Ok(head_state.last_event_seq)
}

pub(crate) fn append_wal(wal_path: &Path, rec: &WalRecord) -> Result<(), AppError> {
    let line = serde_json::to_string(rec).map_err(|e| {
        app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            format!("failed to serialize wal record: {e}"),
            true,
        )
    })?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(wal_path)
        .map_err(|e| app_err_vc("E_VC_IO_WRITE_FAIL", e.to_string(), true))?;

    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub(crate) fn read_wal_lines(wal_path: &Path) -> Result<Vec<String>, AppError> {
    if !wal_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(wal_path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect())
}

pub(crate) fn current_head_from_entity_file(
    project_path: &str,
    entity_id: &str,
    stored: Option<&EntityHead>,
) -> Result<EntityHead, AppError> {
    let mut head = stored.cloned().unwrap_or_default();
    let entity_path = entity_path_from_entity_id(project_path, entity_id)?;

    if entity_path.exists() {
        let content = fs::read_to_string(&entity_path)?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            app_err_vc(
                "E_VC_IO_WRITE_FAIL",
                format!("failed to parse entity json: {e}"),
                true,
            )
        })?;

        let hash = compute_json_hash(&json);
        if head.json_hash != hash {
            head.json_hash = hash;
        }

        if head.updated_at == 0 {
            head.updated_at = chrono::Utc::now().timestamp_millis();
        }
    }

    Ok(head)
}

pub(crate) fn truncate_wal_if_corrupted(wal_path: &Path) -> Result<i64, AppError> {
    if !wal_path.exists() {
        return Ok(0);
    }

    let mut file = OpenOptions::new().read(true).open(wal_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut lines: Vec<&str> = content.split('\n').collect();
    if lines.is_empty() {
        return Ok(0);
    }

    while matches!(lines.last(), Some(l) if l.trim().is_empty()) {
        lines.pop();
    }

    if let Some(last) = lines.last().copied() {
        if serde_json::from_str::<serde_json::Value>(last).is_err() {
            let mut truncated_content = lines[..lines.len().saturating_sub(1)].join("\n");
            truncated_content.push('\n');
            let old_len = content.as_bytes().len() as i64;
            let new_len = truncated_content.as_bytes().len() as i64;
            fs::write(wal_path, truncated_content.as_bytes())?;
            return Ok(old_len - new_len);
        }
    }

    Ok(0)
}
