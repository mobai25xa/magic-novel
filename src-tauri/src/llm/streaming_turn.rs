//! LLM Layer - StreamingTurnEngine
//!
//! Bridges the llm/ streaming layer to the agent_engine::turn::TurnEngine trait.
//! Consumes the provider's stream, feeds events to both the accumulator and the
//! event emitter, and returns a TurnOutput when done.

use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::agent_engine::emitter::EventSink;
use crate::agent_engine::messages::ConversationState;
use crate::agent_engine::turn::{TurnEngine, TurnOutput};
use crate::models::AppError;

use super::accumulator::StreamAccumulator;
use super::provider::new_cancel_token;
use super::router::LlmRouter;
use super::types::{LlmRequest, LlmStreamEvent};

/// A TurnEngine that uses streaming LLM calls via the LlmRouter.
///
/// For each turn:
/// 1. Converts ConversationState to LlmRequest
/// 2. Calls router.stream_chat() to get a stream
/// 3. Emits real-time delta events via AgentEventEmitter
/// 4. Accumulates into StreamAccumulator
/// 5. Returns TurnOutput
pub struct StreamingTurnEngine<S: EventSink> {
    pub router: Arc<LlmRouter>,
    pub emitter: S,
    pub provider_name: String,
    pub model: String,
    pub cancel_token: Option<CancellationToken>,
}

impl<S: EventSink> StreamingTurnEngine<S> {
    pub fn new(router: Arc<LlmRouter>, emitter: S, provider_name: String, model: String) -> Self {
        Self {
            router,
            emitter,
            provider_name,
            model,
            cancel_token: None,
        }
    }

    /// Set an external CancellationToken for cooperative cancellation.
    pub fn with_cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = Some(token);
        self
    }
}

#[async_trait]
impl<S: EventSink> TurnEngine for StreamingTurnEngine<S> {
    async fn execute_turn(
        &self,
        state: &ConversationState,
        tool_schemas: &serde_json::Value,
    ) -> Result<TurnOutput, AppError> {
        // 1. Build LlmRequest from ConversationState
        let req = LlmRequest::from_conversation(
            &self.provider_name,
            &self.model,
            state,
            tool_schemas,
            0.2,
        );

        // 2. Create cancel token for the LLM provider
        let (cancel_tx, cancel_rx) = new_cancel_token();
        let _ = cancel_tx; // keep alive

        // 3. Call router to get stream
        let mut stream = self
            .router
            .stream_chat(req, cancel_rx)
            .await
            .map_err(|e| -> AppError { e.into() })?;

        // 4. Emit STREAMING_STARTED
        let _ = self.emitter.streaming_started();

        // 5. Consume stream: emit events + accumulate, with cancellation support
        let mut accumulator = StreamAccumulator::new();

        if let Some(ref cancel_token) = self.cancel_token {
            // Cancellation-aware streaming loop
            loop {
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                        tracing::info!(
                            target: "llm::streaming_turn",
                            "streaming cancelled via CancellationToken"
                        );
                        return Err(super::errors::LlmError::Cancelled {
                            provider: self.provider_name.clone(),
                        }
                        .into());
                    }
                    item = stream.next() => {
                        match item {
                            Some(Ok(event)) => {
                                self.emit_event(&event);
                                accumulator.apply(&event);
                            }
                            Some(Err(e)) => {
                                tracing::error!(
                                    target: "llm::streaming_turn",
                                    error = %e,
                                    "stream error during turn"
                                );
                                return Err(e.into());
                            }
                            None => break,
                        }
                    }
                }
            }
        } else {
            // No cancellation token — simple streaming loop
            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => {
                        self.emit_event(&event);
                        accumulator.apply(&event);
                    }
                    Err(e) => {
                        tracing::error!(
                            target: "llm::streaming_turn",
                            error = %e,
                            "stream error during turn"
                        );
                        return Err(e.into());
                    }
                }
            }
        }

        // 6. Convert accumulated state to TurnOutput
        accumulator.into_turn_output()
    }
}

impl<S: EventSink> StreamingTurnEngine<S> {
    /// Emit a single stream event to the UI via the emitter
    fn emit_event(&self, event: &LlmStreamEvent) {
        match event {
            LlmStreamEvent::AssistantTextDelta { delta } => {
                let _ = self.emitter.assistant_text_delta(delta);
            }
            LlmStreamEvent::ThinkingDelta { delta } => {
                let _ = self.emitter.thinking_text_delta(delta);
            }
            LlmStreamEvent::ToolCallStart { id, name } => {
                tracing::debug!(
                    target: "llm::streaming_turn",
                    tool_id = %id,
                    tool_name = %name,
                    "tool call started (from LLM)"
                );
                // Tool call started events are emitted by the tool_scheduler
                // when execution begins, not during LLM streaming
            }
            LlmStreamEvent::ToolCallArgsDelta { .. } => {
                // Args deltas are accumulated; no UI event during streaming
            }
            LlmStreamEvent::ToolCallEnd { .. } => {
                // Tool end is internal to accumulator
            }
            LlmStreamEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read,
            } => {
                let usage = crate::agent_engine::types::UsageInfo {
                    input_tokens: *input_tokens,
                    output_tokens: *output_tokens,
                    cache_read_tokens: *cache_read,
                    thinking_tokens: 0,
                };
                let _ = self.emitter.usage_update(&usage);
            }
            LlmStreamEvent::Stop { .. } => {
                // Stop is handled after the stream ends (turn_completed in loop)
            }
        }
    }
}
