//! Gemini SSE streaming provider
//!
//! Handles Google Gemini's SSE streaming format with parts-based responses.
//!
//! Aligned with docs/magic_plan/plan_agent/11-gemini-provider-handler.md

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

/// Gemini SSE streaming provider
pub struct GeminiProvider {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl GeminiProvider {
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

    fn stream_url(&self, model: &str) -> String {
        let normalized = self.base_url.trim().trim_end_matches('/');
        format!("{normalized}/v1beta/models/{model}:streamGenerateContent?alt=sse")
    }

    /// Build the Gemini request body
    fn build_request_body(&self, req: &LlmRequest) -> serde_json::Value {
        let mut contents = Vec::new();

        // System instruction (separate field in Gemini)
        let system_text: String = req
            .system
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Convert messages
        for msg in &req.messages {
            match msg.role {
                Role::User => {
                    contents.push(json!({
                        "role": "user",
                        "parts": [{ "text": msg.text_content() }]
                    }));
                }
                Role::Assistant => {
                    let mut parts = Vec::new();
                    for block in &msg.blocks {
                        match block {
                            ContentBlock::Text { text } => {
                                parts.push(json!({ "text": text }));
                            }
                            ContentBlock::ToolCall { id: _, name, input } => {
                                parts.push(json!({
                                    "functionCall": {
                                        "name": name,
                                        "args": input,
                                    }
                                }));
                            }
                            _ => {}
                        }
                    }
                    if !parts.is_empty() {
                        contents.push(json!({
                            "role": "model",
                            "parts": parts
                        }));
                    }
                }
                Role::Tool => {
                    let mut parts = Vec::new();
                    for block in &msg.blocks {
                        if let ContentBlock::ToolResult {
                            tool_call_id,
                            tool_name,
                            content,
                            is_error,
                        } = block
                        {
                            let fn_name = tool_name.as_deref().unwrap_or(tool_call_id);
                            parts.push(json!({
                                "functionResponse": {
                                    "name": fn_name,
                                    "response": {
                                        "content": content,
                                        "is_error": is_error,
                                    }
                                }
                            }));
                        }
                    }
                    if !parts.is_empty() {
                        contents.push(json!({
                            "role": "user",
                            "parts": parts
                        }));
                    }
                }
                Role::System => {
                    // Handled via system_instruction
                }
            }
        }

        let mut body = json!({
            "contents": contents,
            "generationConfig": {
                "temperature": req.temperature,
            }
        });

        if !system_text.is_empty() {
            body["system_instruction"] = json!({
                "parts": [{ "text": system_text }]
            });
        }

        // Tool declarations
        if !req.tools.is_empty() {
            let declarations: Vec<serde_json::Value> = req
                .tools
                .iter()
                .filter_map(|t| {
                    let func = t.get("function")?;
                    Some(json!({
                        "name": func.get("name")?,
                        "description": func.get("description").and_then(|d| d.as_str()).unwrap_or(""),
                        "parameters": func.get("parameters").cloned().unwrap_or(json!({"type": "object"})),
                    }))
                })
                .collect();

            body["tools"] = json!([{
                "functionDeclarations": declarations
            }]);

            body["toolConfig"] = match req.tool_choice {
                ToolChoice::Auto => json!({"functionCallingConfig": {"mode": "AUTO"}}),
                ToolChoice::None => json!({"functionCallingConfig": {"mode": "NONE"}}),
                ToolChoice::Required => json!({"functionCallingConfig": {"mode": "ANY"}}),
            };
        }

        body
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
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
        let url = self.stream_url(&req.model);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
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

        Ok(Box::pin(gemini_sse_stream(
            byte_stream,
            cancel,
            provider_name,
        )))
    }
}

/// Parse Gemini SSE byte stream into LlmStreamEvent items
fn gemini_sse_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    cancel: CancelToken,
    provider_name: String,
) -> impl Stream<Item = Result<LlmStreamEvent, LlmError>> + Send + 'static {
    struct ParseState {
        buffer: String,
        tool_call_counter: u32,
        cancel: CancelToken,
        provider_name: String,
        done: bool,
    }

    let state = ParseState {
        buffer: String::new(),
        tool_call_counter: 0,
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

                            if !line.starts_with("data:") {
                                continue;
                            }

                            let data = line.trim_start_matches("data:").trim();
                            if data.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(chunk_json) => {
                                    let events = parse_gemini_chunk(
                                        &chunk_json,
                                        &mut state.tool_call_counter,
                                    );
                                    for evt in events.into_iter().rev() {
                                        pending.push(Ok(evt));
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        target: "llm::gemini",
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

/// Parse a single Gemini SSE chunk into LlmStreamEvents
fn parse_gemini_chunk(
    chunk: &serde_json::Value,
    tool_call_counter: &mut u32,
) -> Vec<LlmStreamEvent> {
    let mut events = Vec::new();

    // Extract candidates[0].content.parts[]
    let parts = chunk
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());

    if let Some(parts) = parts {
        for part in parts {
            // Text part
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    events.push(LlmStreamEvent::AssistantTextDelta {
                        delta: text.to_string(),
                    });
                }
            }

            // FunctionCall part (Gemini sends args as a complete object, not incremental)
            if let Some(fc) = part.get("functionCall") {
                let name = fc
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "unknown".to_string());
                let args = fc.get("args").cloned().unwrap_or(json!({}));

                *tool_call_counter += 1;
                let id = format!("gemini_call_{}", tool_call_counter);

                // Emit start + full args + end since Gemini doesn't do incremental
                events.push(LlmStreamEvent::ToolCallStart {
                    id: id.clone(),
                    name,
                });
                events.push(LlmStreamEvent::ToolCallArgsDelta {
                    id: id.clone(),
                    delta: args.to_string(),
                });
                events.push(LlmStreamEvent::ToolCallEnd { id });
            }
        }
    }

    // Check finish reason
    let finish_reason = chunk
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("finishReason"))
        .and_then(|r| r.as_str());

    if let Some(reason) = finish_reason {
        events.push(LlmStreamEvent::Stop {
            reason: LlmStopReason::from_gemini(reason),
        });
    }

    // Usage metadata
    if let Some(meta) = chunk.get("usageMetadata") {
        let input = meta
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let output = meta
            .get("candidatesTokenCount")
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

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gemini_text_chunk() {
        let chunk = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello world" }],
                    "role": "model"
                }
            }]
        });

        let mut counter = 0;
        let events = parse_gemini_chunk(&chunk, &mut counter);
        assert_eq!(events.len(), 1);
        match &events[0] {
            LlmStreamEvent::AssistantTextDelta { delta } => assert_eq!(delta, "Hello world"),
            _ => panic!("expected AssistantTextDelta"),
        }
    }

    #[test]
    fn test_parse_gemini_function_call() {
        let chunk = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "read",
                            "args": { "path": "chapter1" }
                        }
                    }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });

        let mut counter = 0;
        let events = parse_gemini_chunk(&chunk, &mut counter);
        // ToolCallStart + ToolCallArgsDelta + ToolCallEnd + Stop = 4
        assert_eq!(events.len(), 4);
        match &events[0] {
            LlmStreamEvent::ToolCallStart { id, name } => {
                assert_eq!(name, "read");
                assert!(id.starts_with("gemini_call_"));
            }
            _ => panic!("expected ToolCallStart"),
        }
    }

    #[test]
    fn test_parse_gemini_usage() {
        let chunk = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "done" }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 50,
                "candidatesTokenCount": 25
            }
        });

        let mut counter = 0;
        let events = parse_gemini_chunk(&chunk, &mut counter);
        let has_usage = events.iter().any(|e| {
            matches!(
                e,
                LlmStreamEvent::Usage {
                    input_tokens: 50,
                    ..
                }
            )
        });
        assert!(has_usage);
    }

    #[test]
    fn test_stream_url_does_not_include_api_key() {
        let provider = GeminiProvider::new(
            "https://generativelanguage.googleapis.com".to_string(),
            "secret-key".to_string(),
        );
        let url = provider.stream_url("gemini-2.0-flash");
        assert!(url.contains("?alt=sse"));
        assert!(!url.contains("key="));
        assert!(!url.contains("secret-key"));
    }

    #[test]
    fn test_tool_result_function_response_uses_tool_name() {
        let provider = GeminiProvider::new(
            "https://generativelanguage.googleapis.com".to_string(),
            "secret-key".to_string(),
        );
        let req = LlmRequest {
            provider_name: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            system: vec![],
            messages: vec![crate::agent_engine::messages::AgentMessage {
                id: "msg_test".to_string(),
                role: crate::agent_engine::messages::Role::Tool,
                blocks: vec![crate::agent_engine::messages::ContentBlock::ToolResult {
                    tool_call_id: "gemini_call_1".to_string(),
                    tool_name: Some("read".to_string()),
                    content: "{\"ok\":true}".to_string(),
                    is_error: false,
                }],
                ts: 0,
            }],
            tools: vec![],
            tool_choice: ToolChoice::None,
            parallel_tool_calls: true,
            temperature: 0.0,
            reasoning: None,
        };

        let body = provider.build_request_body(&req);
        let name = body
            .get("contents")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("parts"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("functionResponse"))
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(name, "read");
    }

    #[test]
    fn test_tool_result_function_response_falls_back_to_call_id() {
        let provider = GeminiProvider::new(
            "https://generativelanguage.googleapis.com".to_string(),
            "secret-key".to_string(),
        );
        let req = LlmRequest {
            provider_name: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            system: vec![],
            messages: vec![crate::agent_engine::messages::AgentMessage {
                id: "msg_test".to_string(),
                role: crate::agent_engine::messages::Role::Tool,
                blocks: vec![crate::agent_engine::messages::ContentBlock::ToolResult {
                    tool_call_id: "gemini_call_2".to_string(),
                    tool_name: None,
                    content: "{\"ok\":true}".to_string(),
                    is_error: false,
                }],
                ts: 0,
            }],
            tools: vec![],
            tool_choice: ToolChoice::None,
            parallel_tool_calls: true,
            temperature: 0.0,
            reasoning: None,
        };

        let body = provider.build_request_body(&req);
        let name = body
            .get("contents")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("parts"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("functionResponse"))
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(name, "gemini_call_2");
    }

    #[test]
    fn test_provider_new_initializes_client() {
        let provider = GeminiProvider::new(
            "https://generativelanguage.googleapis.com".to_string(),
            "secret-key".to_string(),
        );
        let url = provider.stream_url("gemini-2.0-flash");
        assert!(url.contains(":streamGenerateContent?alt=sse"));
    }
}
