//! LLM Layer - Stream Accumulator
//!
//! Accumulates streaming events into a final TurnOutput.
//! Handles tool_call args best-effort JSON parsing.
//!
//! Aligned with docs/magic_plan/plan_agent/02-llm-providers-and-streaming-accumulator.md

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::agent_engine::messages::{AgentMessage, ContentBlock, Role};
use crate::agent_engine::turn::{validate_turn_output, TurnOutput};
use crate::agent_engine::types::{StopReason, ToolCallInfo, UsageInfo};
use crate::models::AppError;

use super::types::{LlmStopReason, LlmStreamEvent};

/// Accumulated state for a single tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallAccum {
    pub id: String,
    pub name: String,
    /// Raw accumulated JSON string (incremental)
    pub raw_args: String,
    /// Best-effort parsed args (last successful parse)
    pub parsed_args: serde_json::Value,
    /// Whether the tool call is complete
    pub complete: bool,
}

/// Usage accumulator
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageAccum {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read: u64,
}

/// Unified stream accumulator that processes LlmStreamEvent sequences
#[derive(Debug, Clone)]
pub struct StreamAccumulator {
    /// Accumulated assistant text
    pub assistant_text: String,
    /// Accumulated thinking/reasoning text
    pub thinking_text: String,
    /// Tool calls indexed by their ID
    pub tool_calls: HashMap<String, ToolCallAccum>,
    /// Ordered list of tool call IDs (preserves order from LLM)
    pub tool_call_order: Vec<String>,
    /// Token usage
    pub usage: UsageAccum,
    /// Stop reason from the LLM
    pub stop_reason: Option<LlmStopReason>,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self {
            assistant_text: String::new(),
            thinking_text: String::new(),
            tool_calls: HashMap::new(),
            tool_call_order: Vec::new(),
            usage: UsageAccum::default(),
            stop_reason: None,
        }
    }

    /// Apply a single stream event to the accumulator
    pub fn apply(&mut self, event: &LlmStreamEvent) {
        match event {
            LlmStreamEvent::AssistantTextDelta { delta } => {
                self.assistant_text.push_str(delta);
            }
            LlmStreamEvent::ThinkingDelta { delta } => {
                self.thinking_text.push_str(delta);
            }
            LlmStreamEvent::ToolCallStart { id, name } => {
                let accum = ToolCallAccum {
                    id: id.clone(),
                    name: name.clone(),
                    raw_args: String::new(),
                    parsed_args: serde_json::Value::Object(serde_json::Map::new()),
                    complete: false,
                };
                self.tool_calls.insert(id.clone(), accum);
                if !self.tool_call_order.contains(id) {
                    self.tool_call_order.push(id.clone());
                }
            }
            LlmStreamEvent::ToolCallArgsDelta { id, delta } => {
                if let Some(accum) = self.tool_calls.get_mut(id) {
                    accum.raw_args.push_str(delta);
                    // Best-effort parse: try to parse after each delta
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&accum.raw_args) {
                        accum.parsed_args = parsed;
                    } else {
                        // Fallback: extract any closed fields from partial JSON so UI can show key args.
                        let extracted = extract_closed_json_fields(&accum.raw_args);
                        if !extracted.is_empty() {
                            if !accum.parsed_args.is_object() {
                                accum.parsed_args = serde_json::Value::Object(serde_json::Map::new());
                            }
                            if let Some(obj) = accum.parsed_args.as_object_mut() {
                                for (k, v) in extracted {
                                    obj.insert(k, v);
                                }
                            }
                        }
                    }
                    // If parse fails, keep last-good parsed_args
                }
            }
            LlmStreamEvent::ToolCallEnd { id } => {
                if let Some(accum) = self.tool_calls.get_mut(id) {
                    accum.complete = true;
                    // Final parse attempt
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&accum.raw_args) {
                        accum.parsed_args = parsed;
                    } else {
                        let extracted = extract_closed_json_fields(&accum.raw_args);
                        if !extracted.is_empty() {
                            if !accum.parsed_args.is_object() {
                                accum.parsed_args = serde_json::Value::Object(serde_json::Map::new());
                            }
                            if let Some(obj) = accum.parsed_args.as_object_mut() {
                                for (k, v) in extracted {
                                    obj.insert(k, v);
                                }
                            }
                        }
                    }
                }
            }
            LlmStreamEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read,
            } => {
                // Providers may report usage in multiple chunks (e.g. input first, output later).
                // Preserve previously observed non-zero values.
                if *input_tokens > 0 {
                    self.usage.input_tokens = *input_tokens;
                }
                if *output_tokens > 0 {
                    self.usage.output_tokens = *output_tokens;
                }
                if *cache_read > 0 {
                    self.usage.cache_read = *cache_read;
                }
            }
            LlmStreamEvent::Stop { reason } => {
                self.stop_reason = Some(reason.clone());
            }
        }
    }

    /// Convert accumulated state into a TurnOutput for the agent engine
    pub fn into_turn_output(self) -> Result<TurnOutput, AppError> {
        // Build content blocks
        let mut blocks = Vec::new();

        if !self.thinking_text.is_empty() {
            blocks.push(ContentBlock::Thinking {
                text: self.thinking_text,
            });
        }

        if !self.assistant_text.is_empty() {
            blocks.push(ContentBlock::Text {
                text: self.assistant_text,
            });
        }

        // Build tool calls in order
        let mut tool_calls = Vec::new();
        for id in &self.tool_call_order {
            if let Some(accum) = self.tool_calls.get(id) {
                blocks.push(ContentBlock::ToolCall {
                    id: accum.id.clone(),
                    name: accum.name.clone(),
                    input: accum.parsed_args.clone(),
                });
                tool_calls.push(ToolCallInfo {
                    llm_call_id: accum.id.clone(),
                    tool_name: accum.name.clone(),
                    args: accum.parsed_args.clone(),
                });
            }
        }

        // Determine stop reason
        let stop_reason = if !tool_calls.is_empty() {
            StopReason::Success // has tool calls, loop will continue
        } else {
            self.stop_reason
                .as_ref()
                .map(|r| r.to_engine_stop_reason())
                .unwrap_or(StopReason::Success)
        };

        let usage = Some(UsageInfo {
            input_tokens: self.usage.input_tokens,
            output_tokens: self.usage.output_tokens,
            cache_read_tokens: self.usage.cache_read,
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

        validate_turn_output(output, "streaming")
    }
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_closed_json_fields(raw: &str) -> serde_json::Map<String, serde_json::Value> {
    // Extract top-level closed primitive fields from a partial JSON object.
    // This is intentionally conservative: strings + primitives only.
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?:^|[,{])\s*"(?P<key>[^"\\]+)"\s*:\s*(?P<val>"(?:\\.|[^"\\])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)"#)
            .expect("valid regex")
    });

    let mut out = serde_json::Map::new();
    for caps in re.captures_iter(raw) {
        let key = caps.name("key").map(|m| m.as_str()).unwrap_or("");
        let val = caps.name("val").map(|m| m.as_str()).unwrap_or("");
        if key.trim().is_empty() || val.trim().is_empty() {
            continue;
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(val) {
            out.insert(key.to_string(), value);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate_text_deltas() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::AssistantTextDelta {
            delta: "Hello ".to_string(),
        });
        acc.apply(&LlmStreamEvent::AssistantTextDelta {
            delta: "world!".to_string(),
        });
        acc.apply(&LlmStreamEvent::Stop {
            reason: LlmStopReason::EndTurn,
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.assistant_message.text_content(), "Hello world!");
        assert!(output.tool_calls.is_empty());
        assert_eq!(output.stop_reason, StopReason::Success);
    }

    #[test]
    fn test_accumulate_tool_calls() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "read".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"path""#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#":"chapter1"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });
        acc.apply(&LlmStreamEvent::Stop {
            reason: LlmStopReason::ToolCalls,
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.tool_calls.len(), 1);
        assert_eq!(output.tool_calls[0].tool_name, "read");
        assert_eq!(output.tool_calls[0].args["path"], "chapter1");
        // Has tool calls -> stop_reason is Success (loop continues)
        assert_eq!(output.stop_reason, StopReason::Success);
    }

    #[test]
    fn test_accumulate_multiple_tool_calls() {
        let mut acc = StreamAccumulator::new();

        // Tool 1
        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "read".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"path":"ch1"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });

        // Tool 2
        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_2".to_string(),
            name: "ls".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_2".to_string(),
            delta: r#"{"cwd":"/"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_2".to_string(),
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.tool_calls.len(), 2);
        assert_eq!(output.tool_calls[0].tool_name, "read");
        assert_eq!(output.tool_calls[1].tool_name, "ls");
    }

    #[test]
    fn test_best_effort_parse_incomplete_json() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "edit".to_string(),
        });

        // Partial JSON - should not crash, keeps last-good
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"path":"#.to_string(),
        });

        // At this point, raw_args is incomplete JSON
        let tc = acc.tool_calls.get("call_1").unwrap();
        // parsed_args should still be the initial empty object (last-good)
        assert!(tc.parsed_args.is_object());

        // Complete the JSON
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#""test.txt"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });

        let tc = acc.tool_calls.get("call_1").unwrap();
        assert_eq!(tc.parsed_args["path"], "test.txt");
        assert!(tc.complete);
    }

    #[test]
    fn test_usage_accumulation() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read: 20,
        });
        acc.apply(&LlmStreamEvent::AssistantTextDelta {
            delta: "ok".to_string(),
        });

        let output = acc.into_turn_output().unwrap();
        let usage = output.usage.unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.cache_read_tokens, 20);
    }

    #[test]
    fn test_thinking_text() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ThinkingDelta {
            delta: "Let me think...".to_string(),
        });
        acc.apply(&LlmStreamEvent::AssistantTextDelta {
            delta: "Here's my answer".to_string(),
        });
        acc.apply(&LlmStreamEvent::Stop {
            reason: LlmStopReason::EndTurn,
        });

        assert_eq!(acc.thinking_text, "Let me think...");
        let output = acc.into_turn_output().unwrap();
        // Thinking block should come before text block
        assert_eq!(output.assistant_message.blocks.len(), 2);
        match &output.assistant_message.blocks[0] {
            ContentBlock::Thinking { text } => assert_eq!(text, "Let me think..."),
            _ => panic!("expected thinking block"),
        }
    }

    #[test]
    fn test_multiple_thinking_deltas_are_accumulated_into_single_block() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ThinkingDelta {
            delta: "Analyzing ".to_string(),
        });
        acc.apply(&LlmStreamEvent::ThinkingDelta {
            delta: "constraints".to_string(),
        });
        acc.apply(&LlmStreamEvent::AssistantTextDelta {
            delta: "done".to_string(),
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.assistant_message.blocks.len(), 2);
        match &output.assistant_message.blocks[0] {
            ContentBlock::Thinking { text } => assert_eq!(text, "Analyzing constraints"),
            _ => panic!("expected thinking block"),
        }
    }

    #[test]
    fn test_empty_streaming_response_returns_error() {
        let acc = StreamAccumulator::new();
        let err = acc.into_turn_output().unwrap_err();
        let code = err
            .details
            .as_ref()
            .and_then(|d| d.get("code"))
            .and_then(|v| v.as_str());
        assert_eq!(code, Some("E_EMPTY_RESPONSE"));
    }
}
