use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::AppError;

use super::path::normalize_path;
use super::roots::knowledge_root_read;

pub(super) const STORED_OBJECT_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredKnowledgeObject {
    pub schema_version: i32,
    pub r#ref: String,
    pub kind: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    pub revision: i64,
    #[serde(default)]
    pub source_session_ids: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_review_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub fields: serde_json::Value,
}

pub(super) fn stored_object_path(project_path: &Path, target_ref: &str) -> PathBuf {
    knowledge_root_read(project_path).join(target_ref)
}

pub(super) fn history_object_ref(target_ref: &str, revision: i64) -> String {
    let target_ref = normalize_path(target_ref);
    if let Some(prefix) = target_ref.strip_suffix(".json") {
        format!("_history/{prefix}.rev_{revision}.json")
    } else {
        format!("_history/{target_ref}.rev_{revision}.json")
    }
}

pub(super) fn read_stored_object(path: &Path) -> Result<Option<StoredKnowledgeObject>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let obj: StoredKnowledgeObject = serde_json::from_str(&raw).map_err(|e| {
        AppError::invalid_argument(format!(
            "KNOWLEDGE_PROPOSAL_INVALID: stored object parse error at {}: {e}",
            path.to_string_lossy()
        ))
    })?;
    Ok(Some(obj))
}

