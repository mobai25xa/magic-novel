use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::knowledge::types::{KnowledgeProposalBundle, KNOWLEDGE_BRANCH_STALE};
use crate::models::AppError;

use super::roots::knowledge_root_read;

const DEFAULT_ACTIVE_BRANCH_ID: &str = "branch/main";
const BRANCH_STATE_FILE: &str = "branch_state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BranchStateDoc {
    pub schema_version: i32,
    pub active_branch_id: String,
    pub updated_at: i64,
}

fn normalize_branch_id<'a>(value: Option<&'a String>) -> Option<&'a str> {
    value.map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn branch_state_path(project_path: &Path) -> PathBuf {
    knowledge_root_read(project_path).join(BRANCH_STATE_FILE)
}

pub(super) fn resolve_active_branch_id(project_path: &Path) -> String {
    let p = branch_state_path(project_path);
    if let Ok(raw) = std::fs::read_to_string(&p) {
        if let Ok(doc) = serde_json::from_str::<BranchStateDoc>(&raw) {
            let id = doc.active_branch_id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }

    DEFAULT_ACTIVE_BRANCH_ID.to_string()
}

pub(super) fn branch_stale_reason(
    project_path: &Path,
    bundle_branch_id: Option<&String>,
) -> Option<String> {
    let active = resolve_active_branch_id(project_path);
    let Some(branch_id) = normalize_branch_id(bundle_branch_id) else {
        return Some(format!(
            "bundle.branch_id is missing; active_branch_id={active}"
        ));
    };

    if branch_id != active {
        return Some(format!(
            "bundle.branch_id={branch_id} does not match active_branch_id={active}"
        ));
    }

    None
}

pub fn validate_bundle_branch_active(
    project_path: &Path,
    bundle: &KnowledgeProposalBundle,
) -> Result<(), AppError> {
    if let Some(reason) = branch_stale_reason(project_path, bundle.branch_id.as_ref()) {
        return Err(AppError::invalid_argument(format!(
            "{KNOWLEDGE_BRANCH_STALE}: {reason}"
        )));
    }
    Ok(())
}

