//! Tauri Commands - Transport layer only

use serde_json::Value;
use tauri::command;

use crate::agent_engine::types::ToolCallInfo;
use crate::agent_tools::contracts::ToolResult;

fn make_call_id() -> String {
    format!("tool_{}", uuid::Uuid::new_v4())
}

#[command]
pub async fn tool_invoke(
    project_path: String,
    tool: String,
    args: Value,
    call_id: Option<String>,
) -> Result<ToolResult<Value>, crate::models::AppError> {
    let call_id = call_id.unwrap_or_else(make_call_id);
    let tc = ToolCallInfo {
        llm_call_id: call_id.clone(),
        tool_name: tool,
        args,
    };

    Ok(crate::agent_engine::tool_dispatch::execute_tool_call(
        &tc,
        &project_path,
        &call_id,
        None,
        None,
    ))
}
