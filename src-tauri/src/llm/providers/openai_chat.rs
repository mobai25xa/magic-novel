//! OpenAI-compatible SSE streaming provider
//!
//! Handles OpenAI Chat Completions API with streaming (SSE).
//! Compatible with OpenAI, Azure OpenAI, and any OpenAI-compatible endpoint.
//!
//! Aligned with docs/magic_plan/plan_agent/09-openai-streaming-parser.md

use async_trait::async_trait;
use futures::stream;
use futures::Stream;
use futures::StreamExt;
use serde_json::json;
use std::pin::Pin;
use std::time::Duration;

use crate::agent_engine::messages::{ContentBlock, Role};
use crate::llm::constants::{
    LLM_CONNECT_TIMEOUT_SECS, LLM_POOL_MAX_IDLE_PER_HOST, LLM_REQUEST_TIMEOUT_SECS,
};
use crate::llm::errors::LlmError;
use crate::llm::provider::{CancelToken, LlmEventStream, LlmProvider};
use crate::llm::types::{
    LlmRequest, LlmStopReason, LlmStreamEvent, ProviderCapabilities, ToolChoice,
};

#[derive(Default, Debug, Clone)]
struct PendingToolState {
    name: Option<String>,
    buffered_args: String,
    start_emitted: bool,
}

/// OpenAI-compatible streaming provider
pub struct OpenAiChatProvider {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl OpenAiChatProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(LLM_CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(LLM_REQUEST_TIMEOUT_SECS))
            .pool_max_idle_per_host(LLM_POOL_MAX_IDLE_PER_HOST)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url,
            api_key,
            client,
        }
    }

    /// Build the completions URL
    fn completions_url(&self) -> String {
        let normalized = self.base_url.trim().trim_end_matches('/');
        if normalized.ends_with("/chat/completions") {
            normalized.to_string()
        } else if normalized.ends_with("/v1") {
            format!("{normalized}/chat/completions")
        } else {
            format!("{normalized}/v1/chat/completions")
        }
    }

    /// Convert LlmRequest to OpenAI request body
    fn build_request_body(&self, req: &LlmRequest) -> serde_json::Value {
        let mut messages = Vec::new();

        // System messages
        for sys in &req.system {
            messages.push(json!({
                "role": "system",
                "content": sys.text,
            }));
        }

        // Conversation messages
        for msg in &req.messages {
            match msg.role {
                Role::User => {
                    messages.push(json!({
                        "role": "user",
                        "content": msg.text_content(),
                    }));
                }
                Role::Assistant => {
                    let mut entry = json!({ "role": "assistant" });

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
                Role::System => {
                    // Already handled above
                }
            }
        }

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "temperature": req.temperature,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        if !req.tools.is_empty() {
            body["tools"] = json!(req.tools);
            body["tool_choice"] = match req.tool_choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::None => json!("none"),
                ToolChoice::Required => json!("required"),
            };
            if req.parallel_tool_calls {
                body["parallel_tool_calls"] = json!(true);
            }
        }

        body
    }
}

#[async_trait]
impl LlmProvider for OpenAiChatProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_thinking: false,
            supports_parallel_tools: true,
            supports_tool_choice: true,
        }
    }

    async fn stream_chat(
        &self,
        req: LlmRequest,
        cancel: CancelToken,
    ) -> Result<LlmEventStream, LlmError> {
        let body = self.build_request_body(&req);
        let url = self.completions_url();

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: format!("request failed: {e}"),
                provider: self.name().to_string(),
            })?;

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(LlmError::from_http_response(
                status,
                &body_text,
                self.name(),
            ));
        }

        // Get the byte stream
        let byte_stream = response.bytes_stream();
        let provider_name = self.name().to_string();

        // Create an async stream that parses SSE
        let event_stream = openai_sse_stream(byte_stream, cancel, provider_name);

        Ok(Box::pin(event_stream))
    }
}

/// Parse an SSE byte stream into LlmStreamEvent items
fn openai_sse_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    cancel: CancelToken,
    provider_name: String,
) -> impl Stream<Item = Result<LlmStreamEvent, LlmError>> + Send + 'static {
    // State for tracking tool calls by index (OpenAI uses index-based addressing)
    struct ParseState {
        buffer: String,
        // Map from tool_call index to call_id for tracking
        tool_index_to_id: std::collections::HashMap<u32, String>,
        // Pending tool-call fragments that may arrive before call_id is present
        tool_index_pending: std::collections::HashMap<u32, PendingToolState>,
        cancel: CancelToken,
        provider_name: String,
        done: bool,
    }

    let state = ParseState {
        buffer: String::new(),
        tool_index_to_id: std::collections::HashMap::new(),
        tool_index_pending: std::collections::HashMap::new(),
        cancel,
        provider_name,
        done: false,
    };

    stream::unfold(
        (
            Box::pin(byte_stream)
                as Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
            state,
            Vec::<Result<LlmStreamEvent, LlmError>>::new(),
        ),
        |(mut byte_stream, mut state, mut pending_events)| async move {
            loop {
                // Return pending events first
                if let Some(event) = pending_events.pop() {
                    return Some((event, (byte_stream, state, pending_events)));
                }

                if state.done {
                    return None;
                }

                // Check cancellation
                if *state.cancel.borrow() {
                    state.done = true;
                    return Some((
                        Err(LlmError::Cancelled {
                            provider: state.provider_name.clone(),
                        }),
                        (byte_stream, state, pending_events),
                    ));
                }

                // Read next chunk
                match byte_stream.next().await {
                    Some(Ok(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        state.buffer.push_str(&text);

                        // Process complete lines
                        while let Some(pos) = state.buffer.find('\n') {
                            let line = state.buffer[..pos].trim().to_string();
                            state.buffer = state.buffer[pos + 1..].to_string();

                            if line.is_empty() || line.starts_with(':') {
                                continue;
                            }

                            if !line.starts_with("data:") {
                                continue;
                            }

                            let data = line.trim_start_matches("data:").trim();

                            if data == "[DONE]" {
                                state.done = true;
                                break;
                            }

                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(chunk_json) => {
                                    let events = parse_openai_chunk(
                                        &chunk_json,
                                        &mut state.tool_index_to_id,
                                        &mut state.tool_index_pending,
                                    );
                                    // Reverse so we can pop from the back in order
                                    for evt in events.into_iter().rev() {
                                        pending_events.push(Ok(evt));
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        target: "llm::openai",
                                        error = %e,
                                        data = data,
                                        "failed to parse SSE chunk"
                                    );
                                }
                            }
                        }

                        // If we have pending events, return one
                        if let Some(event) = pending_events.pop() {
                            return Some((event, (byte_stream, state, pending_events)));
                        }

                        // If done, return None on next iteration
                        if state.done {
                            return None;
                        }

                        continue;
                    }
                    Some(Err(e)) => {
                        state.done = true;
                        return Some((
                            Err(LlmError::Network {
                                message: format!("stream error: {e}"),
                                provider: state.provider_name.clone(),
                            }),
                            (byte_stream, state, pending_events),
                        ));
                    }
                    None => {
                        // Stream ended
                        state.done = true;
                        return None;
                    }
                }
            }
        },
    )
}

/// Parse a single OpenAI SSE chunk JSON into zero or more LlmStreamEvents
fn parse_openai_chunk(
    chunk: &serde_json::Value,
    tool_index_to_id: &mut std::collections::HashMap<u32, String>,
    tool_index_pending: &mut std::collections::HashMap<u32, PendingToolState>,
) -> Vec<LlmStreamEvent> {
    let mut events = Vec::new();

    let choices = match chunk.get("choices").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => {
            // Check for usage-only chunk (stream_options: include_usage)
            if let Some(usage) = chunk.get("usage") {
                let input = usage
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let output = usage
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let cache = usage
                    .get("prompt_tokens_details")
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                events.push(LlmStreamEvent::Usage {
                    input_tokens: input,
                    output_tokens: output,
                    cache_read: cache,
                });
            }
            return events;
        }
    };

    for choice in choices {
        let delta = match choice.get("delta") {
            Some(d) => d,
            None => continue,
        };

        // Text content delta
        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
            if !content.is_empty() {
                events.push(LlmStreamEvent::AssistantTextDelta {
                    delta: content.to_string(),
                });
            }
        }

        // Tool calls delta
        if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in tool_calls {
                let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                let function_name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty());
                let args_delta = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string);

                // If args/name arrived before id, buffer by index first.
                let pending = tool_index_pending.entry(index).or_default();
                if let Some(name) = function_name.clone() {
                    pending.name = Some(name);
                }
                if let Some(delta) = args_delta.clone() {
                    pending.buffered_args.push_str(&delta);
                }

                // If id now exists in this chunk, bind index->id and flush buffered state.
                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                    let id = id.to_string();
                    tool_index_to_id.insert(index, id.clone());

                    let pending = tool_index_pending.entry(index).or_default();
                    if !pending.start_emitted {
                        let name = pending
                            .name
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());
                        events.push(LlmStreamEvent::ToolCallStart {
                            id: id.clone(),
                            name,
                        });
                        pending.start_emitted = true;
                    }

                    if !pending.buffered_args.is_empty() {
                        events.push(LlmStreamEvent::ToolCallArgsDelta {
                            id: id.clone(),
                            delta: pending.buffered_args.clone(),
                        });
                        pending.buffered_args.clear();
                    }
                    continue;
                }

                // Normal path: id already known from previous chunk for this index.
                if let Some(id) = tool_index_to_id.get(&index) {
                    let pending = tool_index_pending.entry(index).or_default();
                    if !pending.start_emitted {
                        let name = pending
                            .name
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());
                        events.push(LlmStreamEvent::ToolCallStart {
                            id: id.clone(),
                            name,
                        });
                        pending.start_emitted = true;
                    }

                    if !pending.buffered_args.is_empty() {
                        events.push(LlmStreamEvent::ToolCallArgsDelta {
                            id: id.clone(),
                            delta: pending.buffered_args.clone(),
                        });
                        pending.buffered_args.clear();
                    }
                }
            }
        }

        // Finish reason
        if let Some(finish_reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
            // Emit ToolCallEnd for all tool calls when finish_reason is "tool_calls"
            if finish_reason == "tool_calls" {
                for (_, id) in tool_index_to_id.iter() {
                    events.push(LlmStreamEvent::ToolCallEnd { id: id.clone() });
                }
            }

            events.push(LlmStreamEvent::Stop {
                reason: LlmStopReason::from_openai(finish_reason),
            });
        }

        // Usage in choice (some providers include it here)
        if let Some(usage) = choice.get("usage") {
            let input = usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output = usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if input > 0 || output > 0 {
                events.push(LlmStreamEvent::Usage {
                    input_tokens: input,
                    output_tokens: output,
                    cache_read: 0,
                });
            }
        }
    }

    // Top-level usage (stream_options: include_usage)
    if let Some(usage) = chunk.get("usage") {
        if choices.is_empty() || choices.iter().all(|c| c.get("finish_reason").is_some()) {
            let input = usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output = usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cache = usage
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if input > 0 || output > 0 {
                events.push(LlmStreamEvent::Usage {
                    input_tokens: input,
                    output_tokens: output,
                    cache_read: cache,
                });
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions_url() {
        let p = OpenAiChatProvider::new("https://api.openai.com".to_string(), "key".to_string());
        assert_eq!(
            p.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );

        let p = OpenAiChatProvider::new("https://api.openai.com/v1".to_string(), "key".to_string());
        assert_eq!(
            p.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );

        let p = OpenAiChatProvider::new(
            "https://api.openai.com/v1/chat/completions".to_string(),
            "key".to_string(),
        );
        assert_eq!(
            p.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_parse_text_delta_chunk() {
        let chunk = json!({
            "choices": [{
                "index": 0,
                "delta": { "content": "Hello" },
                "finish_reason": null
            }]
        });

        let mut map = std::collections::HashMap::new();
        let mut pending = std::collections::HashMap::new();
        let events = parse_openai_chunk(&chunk, &mut map, &mut pending);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::AssistantTextDelta { delta } => assert_eq!(delta, "Hello"),
            _ => panic!("expected AssistantTextDelta"),
        }
    }

    #[test]
    fn test_parse_tool_call_chunks() {
        let mut map = std::collections::HashMap::new();

        // First chunk: tool call start
        let chunk1 = json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "read",
                            "arguments": ""
                        }
                    }]
                },
                "finish_reason": null
            }]
        });

        let mut pending = std::collections::HashMap::new();
        let events = parse_openai_chunk(&chunk1, &mut map, &mut pending);
        assert_eq!(events.len(), 1); // ToolCallStart (empty args delta is skipped)
        match &events[0] {
            LlmStreamEvent::ToolCallStart { id, name } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "read");
            }
            _ => panic!("expected ToolCallStart"),
        }

        // Second chunk: args delta
        let chunk2 = json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {
                            "arguments": "{\"path\":"
                        }
                    }]
                },
                "finish_reason": null
            }]
        });

        let events = parse_openai_chunk(&chunk2, &mut map, &mut pending);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::ToolCallArgsDelta { id, delta } => {
                assert_eq!(id, "call_abc");
                assert_eq!(delta, "{\"path\":");
            }
            _ => panic!("expected ToolCallArgsDelta"),
        }

        // Final chunk: finish
        let chunk3 = json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "tool_calls"
            }]
        });

        let events = parse_openai_chunk(&chunk3, &mut map, &mut pending);
        // ToolCallEnd + Stop
        assert!(events.len() >= 2);
    }

    #[test]
    fn test_parse_usage_chunk() {
        let chunk = json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_tokens_details": {
                    "cached_tokens": 20
                }
            }
        });

        let mut map = std::collections::HashMap::new();
        let mut pending = std::collections::HashMap::new();
        let events = parse_openai_chunk(&chunk, &mut map, &mut pending);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read,
            } => {
                assert_eq!(*input_tokens, 100);
                assert_eq!(*output_tokens, 50);
                assert_eq!(*cache_read, 20);
            }
            _ => panic!("expected Usage"),
        }
    }

    #[test]
    fn test_parse_tool_call_args_arrive_before_id_are_buffered_and_flushed() {
        let mut map = std::collections::HashMap::new();
        let mut pending = std::collections::HashMap::new();

        // Chunk 1: args arrive first (no id yet)
        let chunk1 = json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {
                            "name": "edit",
                            "arguments": "{\"path\":\"chapter.md\","
                        }
                    }]
                },
                "finish_reason": null
            }]
        });

        let events1 = parse_openai_chunk(&chunk1, &mut map, &mut pending);
        assert!(events1.is_empty());

        // Chunk 2: id arrives + trailing args
        let chunk2 = json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_late",
                        "type": "function",
                        "function": {
                            "arguments": "\"content\":\"hello\"}"
                        }
                    }]
                },
                "finish_reason": null
            }]
        });

        let events2 = parse_openai_chunk(&chunk2, &mut map, &mut pending);
        assert_eq!(events2.len(), 2);

        match &events2[0] {
            LlmStreamEvent::ToolCallStart { id, name } => {
                assert_eq!(id, "call_late");
                assert_eq!(name, "edit");
            }
            _ => panic!("expected ToolCallStart"),
        }

        match &events2[1] {
            LlmStreamEvent::ToolCallArgsDelta { id, delta } => {
                assert_eq!(id, "call_late");
                assert!(delta.contains("\"path\":\"chapter.md\""));
                assert!(delta.contains("\"content\":\"hello\""));
            }
            _ => panic!("expected ToolCallArgsDelta"),
        }
    }

    #[test]
    fn test_provider_new_initializes_client() {
        let provider = OpenAiChatProvider::new(
            "https://api.openai.com".to_string(),
            "dummy-key".to_string(),
        );
        let url = provider.completions_url();
        assert!(url.contains("/chat/completions"));
    }
}
