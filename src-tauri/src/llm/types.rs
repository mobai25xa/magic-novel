//! LLM Layer - Core types
//!
//! Aligned with docs/magic_plan/plan_agent/02-llm-providers-and-streaming-accumulator.md

use serde::{Deserialize, Serialize};

use crate::agent_engine::messages::AgentMessage;

/// A request to send to an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub provider_name: String,
    pub model: String,
    #[serde(default)]
    pub system: Vec<SystemBlock>,
    #[serde(default)]
    pub messages: Vec<AgentMessage>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub tool_choice: ToolChoice,
    #[serde(default = "default_true")]
    pub parallel_tool_calls: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub reasoning: Option<ReasoningConfig>,
}

/// A system prompt block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<String>,
}

/// Tool choice mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}

fn default_true() -> bool {
    true
}

fn resolve_parallel_tool_calls(provider_name: &str, model: &str) -> bool {
    if let Ok(raw) = std::env::var("MAGIC_PARALLEL_TOOL_CALLS") {
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "0" | "false" | "off" | "no" | "disabled" => return false,
            "1" | "true" | "on" | "yes" | "enabled" => return true,
            _ => {}
        }
    }

    // Default: enabled. Providers/models with strict sequential semantics can be
    // forced off via the env var above.
    let _ = (provider_name, model);
    true
}

fn default_temperature() -> f32 {
    0.2
}

/// Reasoning/thinking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub effort: ReasoningEffort,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

/// Unified stream event emitted by all providers
#[derive(Debug, Clone)]
pub enum LlmStreamEvent {
    /// Assistant text content delta
    AssistantTextDelta { delta: String },
    /// Thinking/reasoning text delta
    ThinkingDelta { delta: String },
    /// A new tool call has started
    ToolCallStart { id: String, name: String },
    /// Incremental tool call arguments (JSON string fragment)
    ToolCallArgsDelta { id: String, delta: String },
    /// Tool call arguments are complete
    ToolCallEnd { id: String },
    /// Token usage information
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cache_read: u64,
    },
    /// Stream has ended
    Stop { reason: LlmStopReason },
}

/// LLM-level stop reason (mapped to agent_engine::types::StopReason)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmStopReason {
    /// Normal end of generation
    EndTurn,
    /// Model wants to call tools
    ToolCalls,
    /// Hit max token limit
    MaxTokens,
    /// Unknown/other
    Unknown,
}

impl LlmStopReason {
    /// Convert from OpenAI finish_reason string
    pub fn from_openai(reason: &str) -> Self {
        match reason {
            "stop" => Self::EndTurn,
            "tool_calls" => Self::ToolCalls,
            "length" => Self::MaxTokens,
            _ => Self::Unknown,
        }
    }

    /// Convert from Anthropic stop_reason string
    pub fn from_anthropic(reason: &str) -> Self {
        match reason {
            "end_turn" => Self::EndTurn,
            "tool_use" => Self::ToolCalls,
            "max_tokens" => Self::MaxTokens,
            _ => Self::Unknown,
        }
    }

    /// Convert from Gemini finishReason string
    pub fn from_gemini(reason: &str) -> Self {
        match reason {
            "STOP" => Self::EndTurn,
            "MAX_TOKENS" => Self::MaxTokens,
            _ => Self::Unknown,
        }
    }

    /// Map to agent_engine's StopReason
    pub fn to_engine_stop_reason(&self) -> crate::agent_engine::types::StopReason {
        match self {
            Self::EndTurn => crate::agent_engine::types::StopReason::Success,
            Self::ToolCalls => crate::agent_engine::types::StopReason::Success,
            Self::MaxTokens => crate::agent_engine::types::StopReason::Limit,
            Self::Unknown => crate::agent_engine::types::StopReason::Success,
        }
    }
}

/// Provider capabilities
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub supports_parallel_tools: bool,
    pub supports_tool_choice: bool,
}

/// Helper: build LlmRequest from ConversationState
impl LlmRequest {
    pub fn from_conversation(
        provider_name: &str,
        model: &str,
        state: &crate::agent_engine::messages::ConversationState,
        tool_schemas: &serde_json::Value,
        temperature: f32,
    ) -> Self {
        // Extract system messages and non-system messages
        let mut system_blocks = Vec::new();
        let mut messages = Vec::new();

        for msg in &state.messages {
            match msg.role {
                crate::agent_engine::messages::Role::System => {
                    system_blocks.push(SystemBlock {
                        text: msg.text_content(),
                        cache_control: None,
                    });
                }
                _ => {
                    messages.push(msg.clone());
                }
            }
        }

        let tools = tool_schemas.as_array().cloned().unwrap_or_default();

        let tool_choice = if tools.is_empty() {
            ToolChoice::None
        } else {
            ToolChoice::Auto
        };

        Self {
            provider_name: provider_name.to_string(),
            model: model.to_string(),
            system: system_blocks,
            messages,
            tools,
            tool_choice,
            parallel_tool_calls: resolve_parallel_tool_calls(provider_name, model),
            temperature,
            reasoning: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_request_deserialize_uses_defaults_for_optional_fields() {
        let raw = serde_json::json!({
            "provider_name": "openai-compatible",
            "model": "gpt-4o-mini"
        });

        let req: LlmRequest = serde_json::from_value(raw).expect("request should deserialize");
        assert!(req.system.is_empty());
        assert!(req.messages.is_empty());
        assert!(req.tools.is_empty());
        assert!(matches!(req.tool_choice, ToolChoice::Auto));
        assert!(req.parallel_tool_calls);
        assert_eq!(req.temperature, 0.2);
        assert!(req.reasoning.is_none());
    }
}
