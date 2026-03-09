//! Tauri Commands - Transport layer only

use serde_json::Value;
use tauri::command;

use crate::agent_tools::contracts::{
    CreateInput, DeleteInput, EditInput, GrepInput, LsInput, MoveInput, ReadInput, ToolResult,
};
use crate::agent_tools::runtime::{
    execute_create, execute_delete, execute_edit, execute_grep, execute_ls, execute_move,
    execute_read,
};

fn make_call_id() -> String {
    format!("tool_{}", uuid::Uuid::new_v4())
}

#[command]
pub async fn tool_create(
    input: CreateInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_create(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_read(
    input: ReadInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_read(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_edit(
    input: EditInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_edit(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_delete(
    input: DeleteInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_delete(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_move(
    input: MoveInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_move(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_ls(
    input: LsInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_ls(input, call_id.unwrap_or_else(make_call_id)))
}

#[command]
pub async fn tool_grep(
    input: GrepInput,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    Ok(execute_grep(input, call_id.unwrap_or_else(make_call_id)))
}
