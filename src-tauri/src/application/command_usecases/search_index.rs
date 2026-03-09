use serde::Deserialize;
use serde::Serialize;
use tauri::command;

use crate::application::search_usecases::index::manager::{
    EnsureResult, SearchIndexManager, SearchIndexStatus,
};
use crate::models::AppError;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchIndexStatusInput {
    pub project_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchIndexRebuildInput {
    pub project_path: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchIndexCancelInput {
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchIndexEnsureOutput {
    pub result: EnsureResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchIndexCancelOutput {
    pub cancelled: bool,
}

#[command]
pub async fn search_index_status(
    input: SearchIndexStatusInput,
) -> Result<SearchIndexStatus, AppError> {
    let project = input.project_path.trim();
    if project.is_empty() {
        return Err(AppError::invalid_argument("project_path is required"));
    }

    Ok(SearchIndexManager::global().status(project))
}

#[command]
pub async fn search_index_rebuild(
    input: SearchIndexRebuildInput,
) -> Result<SearchIndexEnsureOutput, AppError> {
    let project = input.project_path.trim();
    if project.is_empty() {
        return Err(AppError::invalid_argument("project_path is required"));
    }

    let result = SearchIndexManager::global().rebuild(project, input.force)?;
    Ok(SearchIndexEnsureOutput { result })
}

#[command]
pub async fn search_index_cancel(
    input: SearchIndexCancelInput,
) -> Result<SearchIndexCancelOutput, AppError> {
    let project = input.project_path.trim();
    if project.is_empty() {
        return Err(AppError::invalid_argument("project_path is required"));
    }

    let cancelled = SearchIndexManager::global().cancel(project);
    Ok(SearchIndexCancelOutput { cancelled })
}
