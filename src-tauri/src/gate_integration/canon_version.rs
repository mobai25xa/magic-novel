//! DevC: canon_version meta — the single source of truth for the accepted
//! knowledge baseline revision.
//!
//! File: `.magic_novel/_meta/canon_version.json`
//! Updated after every knowledge writeback apply or rollback.
//! Read by DevE's reminder builder to populate `Canon Version: accepted@{revision}`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonVersion {
    pub schema_version: i32,
    pub branch_id: String,
    /// Monotonically increasing counter. Does not have to equal any object revision.
    pub revision: i64,
    pub updated_at: i64,
}

impl CanonVersion {
    pub fn new(branch_id: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            branch_id: branch_id.into(),
            revision: 1,
            updated_at: now_ms(),
        }
    }
}

fn meta_file_path(project_path: &Path) -> PathBuf {
    project_path
        .join(".magic_novel")
        .join("_meta")
        .join("canon_version.json")
}

/// Read the canon version file. Returns `None` if the file does not exist.
/// Returns `Err` only on IO or parse errors.
pub fn read_canon_version(project_path: &Path) -> Result<Option<CanonVersion>, AppError> {
    let path = meta_file_path(project_path);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| AppError::internal(format!("canon_version read error: {e}")))?;
    let cv: CanonVersion = serde_json::from_str(&text)
        .map_err(|e| AppError::internal(format!("canon_version parse error: {e}")))?;
    Ok(Some(cv))
}

/// Increment the revision counter and write back to disk.
///
/// If the file does not exist, initialises with `branch_id="branch/main"` and `revision=1`.
/// Called after every successful knowledge writeback apply or rollback.
pub fn bump_canon_version(project_path: &Path) -> Result<CanonVersion, AppError> {
    let path = meta_file_path(project_path);

    let mut cv = match read_canon_version(project_path)? {
        Some(existing) => existing,
        None => CanonVersion::new("branch/main"),
    };

    // Only increment if the file already existed (new() starts at 1 which is the first bump)
    if path.exists() {
        cv.revision = cv.revision.saturating_add(1);
    }
    cv.updated_at = now_ms();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::internal(format!("canon_version mkdir error: {e}")))?;
    }

    crate::utils::atomic_write::atomic_write_json(&path, &cv)
        .map_err(|e| AppError::internal(format!("canon_version write error: {e}")))?;

    Ok(cv)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cv_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn read_returns_none_when_missing() {
        let project = temp_project();
        assert!(read_canon_version(&project).unwrap().is_none());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn bump_creates_file_with_revision_1() {
        let project = temp_project();
        let cv = bump_canon_version(&project).unwrap();
        assert_eq!(cv.revision, 1);
        assert_eq!(cv.schema_version, 1);
        assert_eq!(cv.branch_id, "branch/main");
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn bump_increments_monotonically() {
        let project = temp_project();
        let cv1 = bump_canon_version(&project).unwrap();
        let cv2 = bump_canon_version(&project).unwrap();
        let cv3 = bump_canon_version(&project).unwrap();
        assert_eq!(cv1.revision, 1);
        assert_eq!(cv2.revision, 2);
        assert_eq!(cv3.revision, 3);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn read_roundtrip() {
        let project = temp_project();
        let written = bump_canon_version(&project).unwrap();
        let read = read_canon_version(&project).unwrap().unwrap();
        assert_eq!(written.revision, read.revision);
        assert_eq!(written.branch_id, read.branch_id);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn bump_preserves_branch_id() {
        let project = temp_project();
        // Manually write a version with a custom branch
        let path = project.join(".magic_novel").join("_meta");
        fs::create_dir_all(&path).unwrap();
        let cv = CanonVersion {
            schema_version: 1,
            branch_id: "branch/dev".to_string(),
            revision: 10,
            updated_at: 0,
        };
        fs::write(
            path.join("canon_version.json"),
            serde_json::to_string(&cv).unwrap(),
        )
        .unwrap();

        let bumped = bump_canon_version(&project).unwrap();
        assert_eq!(bumped.revision, 11);
        assert_eq!(bumped.branch_id, "branch/dev");
        let _ = fs::remove_dir_all(&project);
    }
}
