use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppError;

use super::core_utils::app_err_vc;

pub(crate) const VC_DIR: &str = ".magic_nover/vc";
pub(crate) const HEAD_FILE: &str = "head.json";
pub(crate) const WAL_FILE: &str = "wal.jsonl";
pub(crate) const CALL_INDEX_FILE: &str = "call_index.jsonl";
pub(crate) const SNAPSHOTS_DIR: &str = "snapshots";
pub(crate) const REVISIONS_DIR: &str = "revisions";
pub(crate) const LOCKS_DIR: &str = "locks";

pub(crate) const ENTITY_TX_TMP_SUFFIX: &str = ".tmp.vc_tx";

pub(crate) fn vc_root_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(VC_DIR)
}

pub(crate) fn ensure_vc_layout(vc_root: &Path) -> Result<(), AppError> {
    if !vc_root.exists() {
        fs::create_dir_all(vc_root)?;
    }
    let snapshots_dir = vc_root.join(SNAPSHOTS_DIR);
    if !snapshots_dir.exists() {
        fs::create_dir_all(&snapshots_dir)?;
    }
    let revisions_dir = vc_root.join(REVISIONS_DIR);
    if !revisions_dir.exists() {
        fs::create_dir_all(&revisions_dir)?;
    }
    let locks_dir = vc_root.join(LOCKS_DIR);
    if !locks_dir.exists() {
        fs::create_dir_all(&locks_dir)?;
    }
    Ok(())
}

pub(crate) fn lock_path(vc_root: &Path, entity_id: &str) -> PathBuf {
    let safe = sanitize_entity_id(entity_id);
    vc_root.join(LOCKS_DIR).join(format!("{}.lock", safe))
}

pub(crate) fn sanitize_entity_id(entity_id: &str) -> String {
    entity_id
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

pub(crate) fn entity_path_from_entity_id(
    project_path: &str,
    entity_id: &str,
) -> Result<PathBuf, AppError> {
    const MANUSCRIPTS_DIR: &str = "manuscripts";

    let (kind, rest) = entity_id
        .split_once(':')
        .ok_or_else(|| app_err_vc("E_VC_IO_WRITE_FAIL", "invalid entity_id".to_string(), false))?;

    if kind != "chapter" {
        return Err(app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            "unsupported entity kind".to_string(),
            false,
        ));
    }

    if rest.contains('/') {
        Ok(PathBuf::from(project_path).join(MANUSCRIPTS_DIR).join(rest))
    } else {
        Err(app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            "entity_id chapter:<chapter_path> required in this repo".to_string(),
            false,
        ))
    }
}
