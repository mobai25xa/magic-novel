use tauri::command;

use crate::models::AppError;
use crate::services::{
    EntityHead, RecoverOutput, RollbackByCallIdInput, RollbackByRevisionInput, RollbackOutput,
    VcCommitPort, VersioningService,
};

#[command]
pub async fn vc_get_current_head(
    project_path: String,
    entity_id: String,
) -> Result<EntityHead, AppError> {
    let vc = VersioningService::new();
    vc.get_current_head(&project_path, &entity_id)
}

#[command]
pub async fn vc_rollback_by_revision(
    input: RollbackByRevisionInput,
) -> Result<RollbackOutput, AppError> {
    let vc = VersioningService::new();
    vc.rollback_by_revision(input)
}

#[command]
pub async fn vc_rollback_by_call_id(
    input: RollbackByCallIdInput,
) -> Result<RollbackOutput, AppError> {
    let vc = VersioningService::new();
    vc.rollback_by_call_id(input)
}

#[command]
pub async fn vc_recover(project_path: String) -> Result<RecoverOutput, AppError> {
    let vc = VersioningService::new();
    vc.recover(&project_path)
}
