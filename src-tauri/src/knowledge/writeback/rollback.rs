use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mission::artifacts;
use crate::models::AppError;

use super::path::ensure_safe_relative_path;
use super::roots::knowledge_root_write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RollbackManifest {
    pub schema_version: i32,
    pub token: String,
    pub delta_id: String,
    pub created_at: i64,
    pub entries: Vec<RollbackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RollbackEntry {
    pub rel_path: String,
    pub existed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_file: Option<String>,
}

pub(super) fn rollback_dir(project_path: &Path, mission_id: &str, token: &str) -> PathBuf {
    artifacts::knowledge_dir(project_path, mission_id)
        .join("rollback")
        .join(token)
}

pub fn rollback(
    project_path: &Path,
    mission_id: &str,
    token: &str,
) -> Result<(usize, usize), AppError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::invalid_argument("rollback token is required"));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let root = knowledge_root_write(project_path)?;
    let rb_dir = rollback_dir(project_path, mission_id, token);
    let manifest_path = rb_dir.join("manifest.json");

    if !manifest_path.exists() {
        return Err(AppError::not_found("rollback manifest not found"));
    }

    let raw = std::fs::read_to_string(&manifest_path)?;
    let manifest: RollbackManifest = serde_json::from_str(&raw)?;

    let mut restored = 0_usize;
    let mut deleted = 0_usize;
    for entry in &manifest.entries {
        let rel = ensure_safe_relative_path(&entry.rel_path)?;
        let full = root.join(rel);
        if entry.existed {
            let Some(bf) = entry.backup_file.as_ref() else {
                return Err(AppError::invalid_argument("rollback manifest missing backup_file"));
            };
            let prev = std::fs::read_to_string(rb_dir.join(bf))?;
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::utils::atomic_write::atomic_write(&full, &prev)?;
            restored += 1;
        } else {
            if full.exists() {
                std::fs::remove_file(&full)?;
            }
            deleted += 1;
        }
    }

    // Touch a marker for audit.
    let _ = std::fs::write(rb_dir.join("rolled_back_at.txt"), format!("{now}"));

    Ok((restored, deleted))
}

