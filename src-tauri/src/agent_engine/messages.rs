//! Agent Engine - Internal message IR
//!
//! Aligned with docs/magic_plan/plan_agent/08-message-ir-and-provider-conversion.md

use serde::{Deserialize, Serialize};

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A content block within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        content: String,
        is_error: bool,
    },
}

/// An internal message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub role: Role,
    pub blocks: Vec<ContentBlock>,
    pub ts: i64,
}

impl AgentMessage {
    pub fn user(text: String) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            role: Role::User,
            blocks: vec![ContentBlock::Text { text }],
            ts: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn system(text: String) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            role: Role::System,
            blocks: vec![ContentBlock::Text { text }],
            ts: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn tool_result(
        tool_call_id: String,
        tool_name: Option<String>,
        content: String,
        is_error: bool,
    ) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            role: Role::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_call_id,
                tool_name,
                content,
                is_error,
            }],
            ts: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Extract text content from all Text blocks
    pub fn text_content(&self) -> String {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn semantic_signature(&self) -> String {
        let role = serde_json::to_string(&self.role).unwrap_or_else(|_| "\"unknown\"".to_string());
        let blocks = serde_json::to_string(&self.blocks).unwrap_or_else(|_| "[]".to_string());
        format!("{role}:{blocks}")
    }

    /// Extract tool calls from all ToolCall blocks
    pub fn tool_calls(&self) -> Vec<super::types::ToolCallInfo> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolCall { id, name, input } => Some(super::types::ToolCallInfo {
                    llm_call_id: id.clone(),
                    tool_name: name.clone(),
                    args: input.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// Approximate token count for compaction threshold.
    pub fn estimated_tokens(&self) -> u64 {
        self.blocks
            .iter()
            .map(|b| match b {
                ContentBlock::Text { text } => super::compaction::estimate_tokens(text),
                ContentBlock::Thinking { text } => super::compaction::estimate_tokens(text),
                ContentBlock::ToolCall { input, .. } => {
                    super::compaction::estimate_tokens(&input.to_string())
                }
                ContentBlock::ToolResult { content, .. } => {
                    super::compaction::estimate_tokens(content)
                }
            })
            .sum()
    }
}

/// Summary from a compaction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    pub summary_text: String,
    pub anchor_message_id: String,
    pub removed_count: usize,
    pub keep_recent_count: usize,
    pub ts: i64,
}

/// Full conversation state held by the engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    pub session_id: String,
    pub messages: Vec<AgentMessage>,
    pub current_turn: u32,
    pub total_tool_calls: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_compaction: Option<CompactionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_usage: Option<super::types::UsageInfo>,
}

impl ConversationState {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            messages: Vec::new(),
            current_turn: 0,
            total_tool_calls: 0,
            last_compaction: None,
            last_usage: None,
        }
    }

    /// Approximate total estimated token count across all messages.
    pub fn total_estimated_tokens(&self) -> u64 {
        self.messages.iter().map(|m| m.estimated_tokens()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentMessage, ContentBlock, ConversationState, Role};

    #[test]
    fn tool_result_deserializes_without_tool_name() {
        let raw = serde_json::json!({
            "id": "msg_1",
            "role": "tool",
            "blocks": [{
                "type": "tool_result",
                "tool_call_id": "call_1",
                "content": "{}",
                "is_error": false
            }],
            "ts": 0
        });

        let msg: AgentMessage =
            serde_json::from_value(raw).expect("should deserialize legacy tool_result");
        assert!(matches!(msg.role, Role::Tool));

        match &msg.blocks[0] {
            ContentBlock::ToolResult { tool_name, .. } => {
                assert!(tool_name.is_none());
            }
            _ => panic!("expected tool_result block"),
        }
    }

    #[test]
    fn estimated_tokens_counts_cjk_higher_than_ascii() {
        let ascii = AgentMessage::user("abcd".to_string());
        let cjk = AgentMessage::user("中文中文".to_string());

        assert_eq!(ascii.estimated_tokens(), 1);
        assert_eq!(cjk.estimated_tokens(), 4);
    }

    #[test]
    fn total_estimated_tokens_sums_messages() {
        let mut state = ConversationState::new("s1".to_string());
        state.messages.push(AgentMessage::user("abcd".to_string()));
        state.messages.push(AgentMessage::user("中文".to_string()));

        assert_eq!(state.total_estimated_tokens(), 3);
    }
}
