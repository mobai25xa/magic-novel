//! LLM Layer - Provider trait definition
//!
//! Each provider implements this trait to convert LLM requests into
//! provider-specific API calls and return a unified stream of events.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::watch;

use super::errors::LlmError;
use super::types::{LlmRequest, LlmStreamEvent, ProviderCapabilities};

/// Cancellation token: send `true` to signal cancellation
pub type CancelToken = watch::Receiver<bool>;

/// Create a new cancel token pair (sender, receiver)
pub fn new_cancel_token() -> (watch::Sender<bool>, CancelToken) {
    watch::channel(false)
}

/// A boxed stream of LLM events
pub type LlmEventStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, LlmError>> + Send>>;

/// Trait for LLM providers (OpenAI, Anthropic, Gemini, etc.)
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identifier (e.g., "openai-compatible", "anthropic", "gemini")
    fn name(&self) -> &'static str;

    /// What this provider supports
    fn capabilities(&self) -> ProviderCapabilities;

    /// Send a chat request and return a stream of events.
    ///
    /// The stream should emit `LlmStreamEvent` variants as they arrive,
    /// ending with a `Stop` event. The caller can cancel via the `CancelToken`.
    async fn stream_chat(
        &self,
        req: LlmRequest,
        cancel: CancelToken,
    ) -> Result<LlmEventStream, LlmError>;
}
