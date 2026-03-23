use std::path::{Path, PathBuf};

use crate::models::AppError;

pub(super) fn knowledge_root_read(project_path: &Path) -> PathBuf {
    crate::services::knowledge_paths::resolve_knowledge_root_for_read(project_path)
}

pub(super) fn knowledge_root_write(project_path: &Path) -> Result<PathBuf, AppError> {
    crate::services::knowledge_paths::resolve_knowledge_root_for_write(project_path)
}

