//! Agent Engine - Turn engine trait and implementations
//!
//! The TurnEngine abstracts a single LLM call (request → streaming → accumulated output).
//! Dev2 will provide the streaming implementation; here we define the trait and a
//! direct (non-streaming) bridge using the existing OpenAI chat completion endpoint.

use async_trait::async_trait;
use serde_json::json;

use crate::llm::errors::LlmError;
use crate::models::AppError;

use super::messages::{AgentMessage, ContentBlock, ConversationState, Role};
use super::types::{StopReason, ToolCallInfo, UsageInfo};

/// Output from a single turn (one LLM call)
#[derive(Debug, Clone)]
pub struct TurnOutput {
    pub assistant_message: AgentMessage,
    pub tool_calls: Vec<ToolCallInfo>,
    pub stop_reason: StopReason,
    pub usage: Option<UsageInfo>,
}

/// Trait for executing a single LLM turn.
///
/// Implementations:
/// - `OpenAiDirectTurnEngine`: non-streaming bridge using existing reqwest call
/// - (Future) `StreamingTurnEngine`: Dev2's streaming + accumulator
#[async_trait]
pub trait TurnEngine: Send + Sync {
    async fn execute_turn(
        &self,
        state: &ConversationState,
        tool_schemas: &serde_json::Value,
    ) -> Result<TurnOutput, AppError>;
}

/// Non-streaming bridge that calls OpenAI-compatible chat completions directly.
/// Uses the same reqwest approach as the existing `ai_openai_chat_completion` command.
pub struct OpenAiDirectTurnEngine {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl TurnEngine for OpenAiDirectTurnEngine {
    async fn execute_turn(
        &self,
        state: &ConversationState,
        tool_schemas: &serde_json::Value,
    ) -> Result<TurnOutput, AppError> {
        let messages = convert_state_to_openai_messages(state);

        let mut body = json!({
            "model": &self.model,
            "messages": messages,
            "temperature": 0.2,
        });

        if let Some(tools) = tool_schemas.as_array() {
            if !tools.is_empty() {
                body["tools"] = tool_schemas.clone();
                body["tool_choice"] = json!("auto");
            }
        }

        let client = reqwest::Client::new();
        let url = build_completions_url(&self.base_url);

        let response = client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("LLM request failed: {e}")))?;

        let status = response.status();
        let raw = response
            .text()
            .await
            .map_err(|e| AppError::internal(format!("failed to read LLM response: {e}")))?;

        if !status.is_success() {
            let llm_err = crate::llm::errors::LlmError::from_http_response(
                status.as_u16(),
                &raw,
                "openai-compatible",
            );
            return Err(llm_err.into());
        }

        let resp: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| LlmError::ParseError {
                message: format!("failed to parse LLM response: {e}"),
                provider: "openai-compatible".to_string(),
            })?;

        parse_openai_response(&resp)
    }
}

/// Convert our internal ConversationState to OpenAI messages format
fn convert_state_to_openai_messages(state: &ConversationState) -> serde_json::Value {
    let mut messages = Vec::new();

    for msg in &state.messages {
        match msg.role {
            Role::System => {
                messages.push(json!({
                    "role": "system",
                    "content": msg.text_content(),
                }));
            }
            Role::User => {
                messages.push(json!({
                    "role": "user",
                    "content": msg.text_content(),
                }));
            }
            Role::Assistant => {
                let mut entry = json!({
                    "role": "assistant",
                });

                let text = msg.text_content();
                if !text.is_empty() {
                    entry["content"] = json!(text);
                }

                let tool_calls: Vec<serde_json::Value> = msg
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::ToolCall { id, name, input } => Some(json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": input.to_string(),
                            }
                        })),
                        _ => None,
                    })
                    .collect();

                if !tool_calls.is_empty() {
                    entry["tool_calls"] = json!(tool_calls);
                }

                messages.push(entry);
            }
            Role::Tool => {
                for block in &msg.blocks {
                    if let ContentBlock::ToolResult {
                        tool_call_id,
                        content,
                        ..
                    } = block
                    {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": content,
                        }));
                    }
                }
            }
        }
    }

    json!(messages)
}

/// Parse OpenAI chat completion response into TurnOutput
fn parse_openai_response(resp: &serde_json::Value) -> Result<TurnOutput, AppError> {
    let choice = resp
        .get("choices")
        .and_then(|c| c.get(0))
        .ok_or_else(|| AppError::internal("no choices in LLM response"))?;

    let message = choice
        .get("message")
        .ok_or_else(|| AppError::internal("no message in choice"))?;

    let finish_reason = choice
        .get("finish_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("stop");

    // Extract text content
    let text = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract tool calls
    let tool_calls: Vec<ToolCallInfo> = message
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let id = tc.get("id")?.as_str()?.to_string();
                    let func = tc.get("function")?;
                    let name = func.get("name")?.as_str()?.trim().to_ascii_lowercase();
                    if name.is_empty() {
                        return None;
                    }
                    let args_str = func.get("arguments")?.as_str().unwrap_or("{}");
                    let args: serde_json::Value =
                        serde_json::from_str(args_str).unwrap_or(json!({}));

                    Some(ToolCallInfo {
                        llm_call_id: id,
                        tool_name: name,
                        args,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Build assistant message blocks
    let mut blocks = Vec::new();
    if !text.is_empty() {
        blocks.push(ContentBlock::Text { text });
    }
    for tc in &tool_calls {
        blocks.push(ContentBlock::ToolCall {
            id: tc.llm_call_id.clone(),
            name: tc.tool_name.clone(),
            input: tc.args.clone(),
        });
    }

    let stop_reason = if !tool_calls.is_empty() {
        StopReason::Success // will continue loop
    } else {
        match finish_reason {
            "stop" => StopReason::Success,
            "length" => StopReason::Limit,
            _ => StopReason::Success,
        }
    };

    // Extract usage
    let usage = resp.get("usage").map(|u| UsageInfo {
        input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        output_tokens: u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read_tokens: 0,
        thinking_tokens: 0,
    });

    let assistant_message = AgentMessage {
        id: format!("msg_{}", uuid::Uuid::new_v4()),
        role: Role::Assistant,
        blocks,
        ts: chrono::Utc::now().timestamp_millis(),
    };

    let output = TurnOutput {
        assistant_message,
        tool_calls,
        stop_reason,
        usage,
    };

    validate_turn_output(output, "openai-compatible")
}

pub fn validate_turn_output(output: TurnOutput, provider: &str) -> Result<TurnOutput, AppError> {
    let has_text = !output.assistant_message.text_content().trim().is_empty();
    let has_tool_calls = !output.tool_calls.is_empty();
    let should_wait = matches!(
        output.stop_reason,
        StopReason::WaitingConfirmation | StopReason::WaitingAskuser
    );

    if has_text || has_tool_calls || should_wait {
        return Ok(output);
    }

    Err(LlmError::EmptyResponse {
        provider: provider.to_string(),
    }
    .into())
}

fn build_completions_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai_response_text_only() {
        let resp = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5
            }
        });

        let out = parse_openai_response(&resp).unwrap();
        assert_eq!(out.assistant_message.text_content(), "Hello!");
        assert!(out.tool_calls.is_empty());
        assert_eq!(out.stop_reason, StopReason::Success);
        assert_eq!(out.usage.unwrap().input_tokens, 10);
    }

    #[test]
    fn test_parse_openai_response_with_tool_calls() {
        let resp = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "read",
                            "arguments": "{\"project_path\":\"/p\",\"path\":\"ch1\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let out = parse_openai_response(&resp).unwrap();
        assert_eq!(out.tool_calls.len(), 1);
        assert_eq!(out.tool_calls[0].tool_name, "read");
        assert_eq!(out.tool_calls[0].llm_call_id, "call_123");
    }

    #[test]
    fn test_validate_turn_output_empty_response() {
        let out = TurnOutput {
            assistant_message: AgentMessage {
                id: "msg_empty".to_string(),
                role: Role::Assistant,
                blocks: vec![],
                ts: 0,
            },
            tool_calls: vec![],
            stop_reason: StopReason::Success,
            usage: None,
        };

        let err = validate_turn_output(out, "openai-compatible").unwrap_err();
        let code = err
            .details
            .as_ref()
            .and_then(|d| d.get("code"))
            .and_then(|v| v.as_str());
        assert_eq!(code, Some("E_EMPTY_RESPONSE"));
    }

    #[test]
    fn test_build_completions_url() {
        assert_eq!(
            build_completions_url("https://api.openai.com"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_completions_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_completions_url("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}
