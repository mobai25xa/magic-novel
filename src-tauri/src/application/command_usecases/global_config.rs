use tauri::command;

use crate::models::AppError;
use crate::services::global_config;

#[command]
pub async fn import_skill(
    input_path: String,
    override_name: Option<String>,
) -> Result<String, AppError> {
    global_config::import_user_skill_from_file(&input_path, override_name.as_deref())
}

#[command]
pub async fn export_skill(name: String, output_path: String) -> Result<(), AppError> {
    global_config::export_skill_to_file(&name, &output_path)
}
