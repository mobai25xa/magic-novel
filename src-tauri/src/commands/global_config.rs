//! Tauri commands for global config management (~/.magic/)

use tauri::command;

use crate::models::AppError;
use crate::services::global_config;

#[command]
pub async fn list_skills() -> Result<Vec<crate::agent_engine::skills::SkillDefinition>, AppError> {
    Ok(crate::agent_engine::skills::get_skill_definitions())
}

#[command]
pub async fn save_skill(name: String, content: String) -> Result<(), AppError> {
    global_config::save_user_skill(&name, &content)
}

#[command]
pub async fn delete_skill(name: String) -> Result<(), AppError> {
    global_config::delete_user_skill(&name)
}

#[command]
pub async fn list_workers() -> Result<Vec<global_config::WorkerDefinition>, AppError> {
    Ok(global_config::load_worker_definitions())
}

#[command]
pub async fn save_worker(definition: global_config::WorkerDefinition) -> Result<(), AppError> {
    global_config::save_worker_definition(&definition)
}

#[command]
pub async fn delete_worker(name: String) -> Result<(), AppError> {
    global_config::delete_worker_definition(&name)
}

#[command]
pub async fn get_global_rules() -> Result<Option<String>, AppError> {
    Ok(global_config::load_global_rules().map(|r| r.content))
}

#[command]
pub async fn save_global_rules(content: String) -> Result<(), AppError> {
    global_config::save_global_rules(&content)
}
