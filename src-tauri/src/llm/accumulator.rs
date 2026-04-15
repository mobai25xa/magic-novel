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
use crate::llm::errors::{LlmError, StreamToolCallError};
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
    /// Fully parsed args from a complete JSON object, suitable for execution
    pub final_args: Option<serde_json::Value>,
    /// Final parse error observed when the tool call ended or the stream closed
    pub final_parse_error: Option<String>,
    /// Whether closed top-level fields were recovered from partial JSON for preview
    pub partial_fields_recovered: bool,
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
                    final_args: None,
                    final_parse_error: None,
                    partial_fields_recovered: false,
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
                    update_tool_call_preview(accum);
                }
            }
            LlmStreamEvent::ToolCallEnd { id } => {
                if let Some(accum) = self.tool_calls.get_mut(id) {
                    accum.complete = true;
                    finalize_tool_call_args(accum);
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
    pub fn into_turn_output(mut self) -> Result<TurnOutput, AppError> {
        let malformed_tool_calls = self.collect_malformed_tool_calls();
        if !malformed_tool_calls.is_empty() {
            for tool_call in &malformed_tool_calls {
                tracing::warn!(
                    target: "llm::accumulator",
                    tool_name = %tool_call.tool_name,
                    tool_call_id = %tool_call.call_id,
                    raw_args_len = tool_call.raw_args_len,
                    final_json_parse_failed = tool_call.final_json_parse_failed,
                    partial_fields_recovered = tool_call.partial_fields_recovered,
                    failure_kind = %tool_call.failure_kind,
                    json_error = ?tool_call.json_error,
                    downstream_error_code = "none",
                    "malformed streamed tool-call arguments blocked before execution"
                );
            }

            return Err(LlmError::StreamToolArgs {
                provider: "streaming".to_string(),
                tool_calls: malformed_tool_calls,
            }
            .into());
        }

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
                let input = accum
                    .final_args
                    .clone()
                    .unwrap_or_else(|| accum.parsed_args.clone());
                blocks.push(ContentBlock::ToolCall {
                    id: accum.id.clone(),
                    name: accum.name.clone(),
                    input: input.clone(),
                });
                tool_calls.push(ToolCallInfo {
                    llm_call_id: accum.id.clone(),
                    tool_name: accum.name.clone(),
                    args: input,
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

impl StreamAccumulator {
    fn collect_malformed_tool_calls(&mut self) -> Vec<StreamToolCallError> {
        let mut malformed = Vec::new();

        for id in &self.tool_call_order {
            let Some(accum) = self.tool_calls.get_mut(id) else {
                continue;
            };

            if accum.final_args.is_none() && accum.final_parse_error.is_none() {
                finalize_tool_call_args(accum);
            }

            if let Some(detail) = malformed_tool_call_detail(accum) {
                malformed.push(detail);
            }
        }

        malformed
    }
}

fn update_tool_call_preview(accum: &mut ToolCallAccum) {
    match serde_json::from_str::<serde_json::Value>(&accum.raw_args) {
        Ok(parsed) => {
            accum.parsed_args = parsed.clone();
            accum.final_args = Some(parsed);
            accum.final_parse_error = None;
        }
        Err(_) => {
            accum.final_args = None;
            merge_partial_preview(accum);
        }
    }
}

fn finalize_tool_call_args(accum: &mut ToolCallAccum) {
    match serde_json::from_str::<serde_json::Value>(&accum.raw_args) {
        Ok(parsed) => {
            accum.parsed_args = parsed.clone();
            accum.final_args = Some(parsed);
            accum.final_parse_error = None;
        }
        Err(error) => {
            accum.final_args = None;
            accum.final_parse_error = Some(error.to_string());
            merge_partial_preview(accum);
        }
    }
}

fn merge_partial_preview(accum: &mut ToolCallAccum) {
    let extracted = extract_closed_json_fields(&accum.raw_args);
    if extracted.is_empty() {
        return;
    }

    accum.partial_fields_recovered = true;
    if !accum.parsed_args.is_object() {
        accum.parsed_args = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(obj) = accum.parsed_args.as_object_mut() {
        for (k, v) in extracted {
            obj.insert(k, v);
        }
    }
}

fn malformed_tool_call_detail(accum: &ToolCallAccum) -> Option<StreamToolCallError> {
    match accum.final_args.as_ref() {
        Some(args) if args.is_object() => None,
        Some(args) => Some(StreamToolCallError {
            call_id: accum.id.clone(),
            tool_name: accum.name.clone(),
            raw_args_len: accum.raw_args.len(),
            final_json_parse_failed: false,
            partial_fields_recovered: accum.partial_fields_recovered,
            failure_kind: "top_level_non_object".to_string(),
            json_error: None,
            parsed_json_type: Some(json_value_type(args).to_string()),
        }),
        None => Some(StreamToolCallError {
            call_id: accum.id.clone(),
            tool_name: accum.name.clone(),
            raw_args_len: accum.raw_args.len(),
            final_json_parse_failed: true,
            partial_fields_recovered: accum.partial_fields_recovered,
            failure_kind: if accum.complete {
                "invalid_json".to_string()
            } else {
                "stream_incomplete".to_string()
            },
            json_error: Some(accum.final_parse_error.clone().unwrap_or_else(|| {
                "stream ended before tool-call args formed valid JSON".to_string()
            })),
            parsed_json_type: None,
        }),
    }
}

fn json_value_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
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
            name: "context_read".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"target_ref""#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#":"chapter:manuscripts/vol_1/ch_1.json"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });
        acc.apply(&LlmStreamEvent::Stop {
            reason: LlmStopReason::ToolCalls,
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.tool_calls.len(), 1);
        assert_eq!(output.tool_calls[0].tool_name, "context_read");
        assert_eq!(
            output.tool_calls[0].args["target_ref"],
            "chapter:manuscripts/vol_1/ch_1.json"
        );
        // Has tool calls -> stop_reason is Success (loop continues)
        assert_eq!(output.stop_reason, StopReason::Success);
    }

    #[test]
    fn test_accumulate_multiple_tool_calls() {
        let mut acc = StreamAccumulator::new();

        // Tool 1
        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "context_read".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"target_ref":"chapter:manuscripts/vol_1/ch_1.json"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });

        // Tool 2
        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_2".to_string(),
            name: "workspace_map".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_2".to_string(),
            delta: r#"{"scope":"book"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_2".to_string(),
        });

        let output = acc.into_turn_output().unwrap();
        assert_eq!(output.tool_calls.len(), 2);
        assert_eq!(output.tool_calls[0].tool_name, "context_read");
        assert_eq!(output.tool_calls[1].tool_name, "workspace_map");
    }

    #[test]
    fn test_best_effort_parse_incomplete_json() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "draft_write".to_string(),
        });

        // Partial JSON - should not crash, keeps last-good
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: "{\"target_ref\":\"".to_string(),
        });

        // At this point, raw_args is incomplete JSON
        let tc = acc.tool_calls.get("call_1").unwrap();
        // parsed_args should still be the initial empty object (last-good)
        assert!(tc.parsed_args.is_object());

        // Complete the JSON
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"chapter:manuscripts/vol_1/ch_1.json"}"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });

        let tc = acc.tool_calls.get("call_1").unwrap();
        assert_eq!(
            tc.raw_args,
            r#"{"target_ref":"chapter:manuscripts/vol_1/ch_1.json"}"#
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&tc.raw_args).expect("raw_args should be valid JSON");
        assert_eq!(parsed["target_ref"], "chapter:manuscripts/vol_1/ch_1.json");
        assert_eq!(
            tc.parsed_args["target_ref"],
            "chapter:manuscripts/vol_1/ch_1.json"
        );
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

    #[test]
    fn test_invalid_final_tool_json_returns_stream_tool_args_error() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "knowledge_write".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"changes":[{"target_ref":"knowledge:.magic_novel/terms/foo.json""#
                .to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });
        acc.apply(&LlmStreamEvent::Stop {
            reason: LlmStopReason::ToolCalls,
        });

        let err = acc.into_turn_output().unwrap_err();
        let details = err.details.expect("details");
        assert_eq!(details["code"], "E_STREAM_TOOL_ARGS_INVALID");
        assert_eq!(details["tool_name"], "knowledge_write");
        assert_eq!(details["failure_kind"], "invalid_json");
        assert_eq!(details["final_json_parse_failed"], true);
        assert!(details["partial_fields_recovered"].is_boolean());
        assert!(details["tool_calls"].is_array());
    }

    #[test]
    fn test_non_object_tool_args_return_stream_tool_args_error() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "workspace_map".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"["book"]"#.to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallEnd {
            id: "call_1".to_string(),
        });

        let err = acc.into_turn_output().unwrap_err();
        let details = err.details.expect("details");
        assert_eq!(details["code"], "E_STREAM_TOOL_ARGS_INVALID");
        assert_eq!(details["tool_name"], "workspace_map");
        assert_eq!(details["failure_kind"], "top_level_non_object");
        assert_eq!(details["parsed_json_type"], "array");
        assert_eq!(details["final_json_parse_failed"], false);
    }

    #[test]
    fn test_missing_tool_call_end_is_classified_as_stream_incomplete() {
        let mut acc = StreamAccumulator::new();

        acc.apply(&LlmStreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "context_read".to_string(),
        });
        acc.apply(&LlmStreamEvent::ToolCallArgsDelta {
            id: "call_1".to_string(),
            delta: r#"{"target_ref":"chapter:manuscripts/vol_1/"#.to_string(),
        });

        let err = acc.into_turn_output().unwrap_err();
        let details = err.details.expect("details");
        assert_eq!(details["code"], "E_STREAM_TOOL_ARGS_INVALID");
        assert_eq!(details["tool_name"], "context_read");
        assert_eq!(details["failure_kind"], "stream_incomplete");
        assert_eq!(details["final_json_parse_failed"], true);
    }
}
