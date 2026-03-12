//! Anthropic Messages API streaming provider
//!
//! Handles Anthropic's SSE streaming format with content_block_delta,
//! input_json_delta, thinking_delta, etc.
//!
//! Aligned with docs/magic_plan/plan_agent/10-anthropic-provider-handler.md

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

/// Anthropic Messages API streaming provider
pub struct AnthropicProvider {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
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

    fn messages_url(&self) -> String {
        let normalized = self.base_url.trim().trim_end_matches('/');
        if normalized.ends_with("/messages") {
            normalized.to_string()
        } else if normalized.ends_with("/v1") {
            format!("{normalized}/messages")
        } else {
            format!("{normalized}/v1/messages")
        }
    }

    /// Build the Anthropic request body
    fn build_request_body(&self, req: &LlmRequest) -> serde_json::Value {
        // System blocks
        let system: Vec<serde_json::Value> = req
            .system
            .iter()
            .map(|s| {
                let mut block = json!({ "type": "text", "text": s.text });
                if let Some(cc) = &s.cache_control {
                    block["cache_control"] = json!({ "type": cc });
                }
                block
            })
            .collect();

        // Convert messages (must ensure tool_use/tool_result pairing)
        let messages = self.convert_messages(&req.messages);

        let mut body = json!({
            "model": req.model,
            "system": system,
            "messages": messages,
            "max_tokens": 8192,
            "stream": true,
        });

        if !req.tools.is_empty() {
            let tools: Vec<serde_json::Value> = req
                .tools
                .iter()
                .filter_map(|t| {
                    let func = t.get("function")?;
                    Some(json!({
                        "name": func.get("name")?,
                        "description": func.get("description").and_then(|d| d.as_str()).unwrap_or(""),
                        "input_schema": func.get("parameters").cloned().unwrap_or(json!({"type": "object"})),
                    }))
                })
                .collect();
            body["tools"] = json!(tools);

            body["tool_choice"] = match req.tool_choice {
                ToolChoice::Auto => json!({"type": "auto"}),
                ToolChoice::None => json!({"type": "none"}),
                ToolChoice::Required => json!({"type": "any"}),
            };
        }

        // Thinking/reasoning config
        if let Some(reasoning) = &req.reasoning {
            let budget = reasoning.budget_tokens.unwrap_or(4096);
            body["thinking"] = json!({
                "type": "enabled",
                "budget_tokens": budget,
            });
        }

        body
    }

    /// Convert internal messages to Anthropic format, ensuring tool_use/tool_result pairing
    fn convert_messages(
        &self,
        messages: &[crate::agent_engine::messages::AgentMessage],
    ) -> Vec<serde_json::Value> {
        let mut result = Vec::new();
        // Track tool_use IDs that need tool_results
        let mut pending_tool_uses: Vec<String> = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    // System handled separately
                }
                Role::User => {
                    // Before user message, inject cancelled tool_results for any unpaired tool_uses
                    if !pending_tool_uses.is_empty() {
                        let mut content = Vec::new();
                        for id in pending_tool_uses.drain(..) {
                            content.push(json!({
                                "type": "tool_result",
                                "tool_use_id": id,
                                "content": "cancelled",
                                "is_error": true,
                            }));
                        }
                        result.push(json!({
                            "role": "user",
                            "content": content,
                        }));
                    }

                    result.push(json!({
                        "role": "user",
                        "content": msg.text_content(),
                    }));
                }
                Role::Assistant => {
                    let mut content = Vec::new();

                    for block in &msg.blocks {
                        match block {
                            ContentBlock::Thinking { .. } => {
                                // Do not send thinking blocks back to Anthropic.
                                // Some Anthropic thinking formats require signatures and may be rejected on replay.
                            }
                            ContentBlock::Text { text } => {
                                content.push(json!({
                                    "type": "text",
                                    "text": text,
                                }));
                            }
                            ContentBlock::ToolCall { id, name, input } => {
                                content.push(json!({
                                    "type": "tool_use",
                                    "id": id,
                                    "name": name,
                                    "input": input,
                                }));
                                pending_tool_uses.push(id.clone());
                            }
                            _ => {}
                        }
                    }

                    if !content.is_empty() {
                        result.push(json!({
                            "role": "assistant",
                            "content": content,
                        }));
                    }
                }
                Role::Tool => {
                    let mut content = Vec::new();
                    for block in &msg.blocks {
                        if let ContentBlock::ToolResult {
                            tool_call_id,
                            content: result_content,
                            is_error,
                            ..
                        } = block
                        {
                            content.push(json!({
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": result_content,
                                "is_error": is_error,
                            }));
                            pending_tool_uses.retain(|id| id != tool_call_id);
                        }
                    }
                    if !content.is_empty() {
                        result.push(json!({
                            "role": "user",
                            "content": content,
                        }));
                    }
                }
            }
        }

        // Inject cancelled results for any remaining unpaired tool_uses
        if !pending_tool_uses.is_empty() {
            let mut content = Vec::new();
            for id in pending_tool_uses {
                content.push(json!({
                    "type": "tool_result",
                    "tool_use_id": id,
                    "content": "cancelled",
                    "is_error": true,
                }));
            }
            result.push(json!({
                "role": "user",
                "content": content,
            }));
        }

        result
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_thinking: true,
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
        let url = self.messages_url();

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let byte_stream = response.bytes_stream();
        let provider_name = self.name().to_string();

        Ok(Box::pin(anthropic_sse_stream(
            byte_stream,
            cancel,
            provider_name,
        )))
    }
}

/// Parse Anthropic SSE byte stream into LlmStreamEvent items
fn anthropic_sse_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    cancel: CancelToken,
    provider_name: String,
) -> impl Stream<Item = Result<LlmStreamEvent, LlmError>> + Send + 'static {
    struct ParseState {
        buffer: String,
        // Track current content block type/id for mapping deltas
        current_block_type: Option<String>,
        current_tool_id: Option<String>,
        cancel: CancelToken,
        provider_name: String,
        done: bool,
    }

    let state = ParseState {
        buffer: String::new(),
        current_block_type: None,
        current_tool_id: None,
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
        |(mut byte_stream, mut state, mut pending)| async move {
            loop {
                if let Some(event) = pending.pop() {
                    return Some((event, (byte_stream, state, pending)));
                }

                if state.done {
                    return None;
                }

                if *state.cancel.borrow() {
                    state.done = true;
                    return Some((
                        Err(LlmError::Cancelled {
                            provider: state.provider_name.clone(),
                        }),
                        (byte_stream, state, pending),
                    ));
                }

                match byte_stream.next().await {
                    Some(Ok(chunk)) => {
                        state.buffer.push_str(&String::from_utf8_lossy(&chunk));

                        while let Some(pos) = state.buffer.find('\n') {
                            let line = state.buffer[..pos].trim().to_string();
                            state.buffer = state.buffer[pos + 1..].to_string();

                            if line.is_empty() || line.starts_with(':') {
                                continue;
                            }

                            // Anthropic sends "event: <type>" then "data: <json>"
                            if line.starts_with("event:") {
                                // We handle events from data lines
                                continue;
                            }

                            if !line.starts_with("data:") {
                                continue;
                            }

                            let data = line.trim_start_matches("data:").trim();
                            if data.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(event_json) => {
                                    let events = parse_anthropic_event(
                                        &event_json,
                                        &mut state.current_block_type,
                                        &mut state.current_tool_id,
                                    );
                                    for evt in events.into_iter().rev() {
                                        pending.push(Ok(evt));
                                    }

                                    // Check for message_stop
                                    if event_json.get("type").and_then(|t| t.as_str())
                                        == Some("message_stop")
                                    {
                                        state.done = true;
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        target: "llm::anthropic",
                                        error = %e,
                                        "failed to parse SSE data"
                                    );
                                }
                            }
                        }

                        if let Some(event) = pending.pop() {
                            return Some((event, (byte_stream, state, pending)));
                        }
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
                            (byte_stream, state, pending),
                        ));
                    }
                    None => {
                        state.done = true;
                        return None;
                    }
                }
            }
        },
    )
}

/// Parse a single Anthropic SSE event JSON into LlmStreamEvents
fn parse_anthropic_event(
    event: &serde_json::Value,
    current_block_type: &mut Option<String>,
    current_tool_id: &mut Option<String>,
) -> Vec<LlmStreamEvent> {
    let mut events = Vec::new();

    let event_type = match event.get("type").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return events,
    };

    match event_type {
        "content_block_start" => {
            if let Some(content_block) = event.get("content_block") {
                let block_type = content_block
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                *current_block_type = Some(block_type.to_string());

                match block_type {
                    "tool_use" => {
                        let id = content_block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = content_block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_ascii_lowercase())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "unknown".to_string());
                        *current_tool_id = Some(id.clone());
                        events.push(LlmStreamEvent::ToolCallStart { id, name });
                    }
                    _ => {
                        // text or thinking block start - no event needed
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Some(delta) = event.get("delta") {
                let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match delta_type {
                    "text_delta" => {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                events.push(LlmStreamEvent::AssistantTextDelta {
                                    delta: text.to_string(),
                                });
                            }
                        }
                    }
                    "input_json_delta" => {
                        if let Some(partial_json) =
                            delta.get("partial_json").and_then(|t| t.as_str())
                        {
                            if let Some(id) = current_tool_id.as_ref() {
                                events.push(LlmStreamEvent::ToolCallArgsDelta {
                                    id: id.clone(),
                                    delta: partial_json.to_string(),
                                });
                            }
                        }
                    }
                    "thinking_delta" => {
                        if let Some(thinking) = delta.get("thinking").and_then(|t| t.as_str()) {
                            if !thinking.is_empty() {
                                events.push(LlmStreamEvent::ThinkingDelta {
                                    delta: thinking.to_string(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        "content_block_stop" => {
            // If current block was a tool_use, emit ToolCallEnd
            if current_block_type.as_deref() == Some("tool_use") {
                if let Some(id) = current_tool_id.take() {
                    events.push(LlmStreamEvent::ToolCallEnd { id });
                }
            }
            *current_block_type = None;
        }
        "message_delta" => {
            // Usage and stop_reason
            if let Some(delta) = event.get("delta") {
                if let Some(stop_reason) = delta.get("stop_reason").and_then(|s| s.as_str()) {
                    events.push(LlmStreamEvent::Stop {
                        reason: LlmStopReason::from_anthropic(stop_reason),
                    });
                }
            }
            if let Some(usage) = event.get("usage") {
                let output = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if output > 0 {
                    events.push(LlmStreamEvent::Usage {
                        input_tokens: 0, // Anthropic sends input in message_start
                        output_tokens: output,
                        cache_read: 0,
                    });
                }
            }
        }
        "message_start" => {
            // Extract input token usage from message_start
            if let Some(message) = event.get("message") {
                if let Some(usage) = message.get("usage") {
                    let input = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let cache = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    if input > 0 {
                        events.push(LlmStreamEvent::Usage {
                            input_tokens: input,
                            output_tokens: 0,
                            cache_read: cache,
                        });
                    }
                }
            }
        }
        "message_stop" => {
            // Stream is done - handled by caller
        }
        _ => {}
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_url() {
        let p = AnthropicProvider::new("https://api.anthropic.com".to_string(), "key".to_string());
        assert_eq!(p.messages_url(), "https://api.anthropic.com/v1/messages");

        let p = AnthropicProvider::new(
            "https://api.anthropic.com/v1".to_string(),
            "key".to_string(),
        );
        assert_eq!(p.messages_url(), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn test_parse_text_delta() {
        let event = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });

        let mut block_type = Some("text".to_string());
        let mut tool_id = None;
        let events = parse_anthropic_event(&event, &mut block_type, &mut tool_id);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::AssistantTextDelta { delta } => assert_eq!(delta, "Hello"),
            _ => panic!("expected AssistantTextDelta"),
        }
    }

    #[test]
    fn test_parse_tool_use_flow() {
        let mut block_type = None;
        let mut tool_id = None;

        // content_block_start with tool_use
        let start = json!({
            "type": "content_block_start",
            "content_block": {
                "type": "tool_use",
                "id": "toolu_123",
                "name": "read"
            }
        });
        let events = parse_anthropic_event(&start, &mut block_type, &mut tool_id);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::ToolCallStart { id, name } => {
                assert_eq!(id, "toolu_123");
                assert_eq!(name, "read");
            }
            _ => panic!("expected ToolCallStart"),
        }

        // input_json_delta
        let delta = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "input_json_delta",
                "partial_json": "{\"path\":"
            }
        });
        let events = parse_anthropic_event(&delta, &mut block_type, &mut tool_id);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::ToolCallArgsDelta { id, delta } => {
                assert_eq!(id, "toolu_123");
                assert_eq!(delta, "{\"path\":");
            }
            _ => panic!("expected ToolCallArgsDelta"),
        }

        // content_block_stop
        let stop = json!({ "type": "content_block_stop" });
        let events = parse_anthropic_event(&stop, &mut block_type, &mut tool_id);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::ToolCallEnd { id } => assert_eq!(id, "toolu_123"),
            _ => panic!("expected ToolCallEnd"),
        }
    }

    #[test]
    fn test_parse_message_delta_stop() {
        let event = json!({
            "type": "message_delta",
            "delta": { "stop_reason": "end_turn" },
            "usage": { "output_tokens": 42 }
        });

        let mut block_type = None;
        let mut tool_id = None;
        let events = parse_anthropic_event(&event, &mut block_type, &mut tool_id);
        assert!(!events.is_empty());
        let has_stop = events.iter().any(
            |e| matches!(e, LlmStreamEvent::Stop { reason } if *reason == LlmStopReason::EndTurn),
        );
        assert!(has_stop);
    }

    #[test]
    fn test_parse_thinking_delta() {
        let event = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "thinking_delta",
                "thinking": "Analyzing context"
            }
        });

        let mut block_type = Some("thinking".to_string());
        let mut tool_id = None;
        let events = parse_anthropic_event(&event, &mut block_type, &mut tool_id);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::ThinkingDelta { delta } => assert_eq!(delta, "Analyzing context"),
            _ => panic!("expected ThinkingDelta"),
        }
    }

    #[test]
    fn test_parse_thinking_flow_with_block_start_and_stop() {
        let mut block_type = None;
        let mut tool_id = None;

        let start = json!({
            "type": "content_block_start",
            "content_block": {
                "type": "thinking"
            }
        });
        let start_events = parse_anthropic_event(&start, &mut block_type, &mut tool_id);
        assert!(start_events.is_empty());
        assert_eq!(block_type.as_deref(), Some("thinking"));

        let delta = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "thinking_delta",
                "thinking": "step-by-step"
            }
        });
        let delta_events = parse_anthropic_event(&delta, &mut block_type, &mut tool_id);
        assert_eq!(delta_events.len(), 1);
        assert!(matches!(
            delta_events[0],
            LlmStreamEvent::ThinkingDelta { .. }
        ));

        let stop = json!({ "type": "content_block_stop" });
        let stop_events = parse_anthropic_event(&stop, &mut block_type, &mut tool_id);
        assert!(stop_events.is_empty());
        assert!(block_type.is_none());
    }

    #[test]
    fn test_provider_new_initializes_client() {
        let provider = AnthropicProvider::new(
            "https://api.anthropic.com".to_string(),
            "dummy-key".to_string(),
        );
        let url = provider.messages_url();
        assert!(url.ends_with("/v1/messages") || url.ends_with("/messages"));
    }
}
