//! LLM Layer - Router with retry/backoff/jitter
//!
//! Routes requests to the appropriate provider, handles retries with
//! exponential backoff and jitter.
//!
//! Aligned with docs/magic_plan/plan_agent/02-llm-providers-and-streaming-accumulator.md

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::llm::errors::LlmError;
use crate::llm::provider::{CancelToken, LlmEventStream, LlmProvider};
use crate::llm::types::LlmRequest;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub factor: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            factor: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Config for foreground user-triggered chat (fail faster, retry less).
    pub fn interactive() -> Self {
        Self {
            max_retries: 1,
            base_delay_ms: 500,
            factor: 2.0,
            jitter: true,
        }
    }

    /// Config for worker/exec context (more aggressive retries)
    pub fn worker() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 2000,
            factor: 2.5,
            jitter: true,
        }
    }

    /// Calculate delay for the nth retry attempt
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        let base = self.base_delay_ms as f64 * self.factor.powi(attempt as i32);
        let delay = base as u64;

        if self.jitter {
            // Add random jitter: 50% - 150% of calculated delay
            use rand::Rng;
            let jitter_factor = 0.5 + rand::thread_rng().gen::<f64>();
            (delay as f64 * jitter_factor) as u64
        } else {
            delay
        }
    }
}

/// A registered provider with its configuration
struct ProviderEntry {
    provider: Arc<dyn LlmProvider>,
    /// Provider names this entry handles (e.g., ["openai-compatible", "openai"])
    aliases: Vec<String>,
}

/// LLM Router: selects provider and handles retry logic
pub struct LlmRouter {
    providers: Vec<ProviderEntry>,
    retry_config: RetryConfig,
    rr_counter: AtomicUsize,
}

impl LlmRouter {
    pub fn new(retry_config: RetryConfig) -> Self {
        Self {
            providers: Vec::new(),
            retry_config,
            rr_counter: AtomicUsize::new(0),
        }
    }

    /// Register a provider with optional aliases
    pub fn register(&mut self, provider: Arc<dyn LlmProvider>, aliases: Vec<String>) {
        let mut all_aliases = aliases;
        // Always include the provider's canonical name
        let name = provider.name().to_string();
        if !all_aliases.contains(&name) {
            all_aliases.push(name);
        }
        self.providers.push(ProviderEntry {
            provider,
            aliases: all_aliases,
        });
    }

    /// Find providers by name (supports multiple entries for same alias/base_url group).
    fn find_providers(&self, name: &str) -> Vec<&Arc<dyn LlmProvider>> {
        self.providers
            .iter()
            .filter(|e| e.aliases.iter().any(|a| a == name))
            .map(|e| &e.provider)
            .collect()
    }

    /// Find all alternate providers excluding the requested alias.
    fn find_alternates(&self, exclude_name: &str) -> Vec<&Arc<dyn LlmProvider>> {
        self.providers
            .iter()
            .filter(|e| !e.aliases.iter().any(|a| a == exclude_name))
            .map(|e| &e.provider)
            .collect()
    }

    /// Route a request: select provider, call with retry logic
    ///
    /// Retry strategy:
    /// - context_limit: no retry, return immediately (caller triggers compaction)
    /// - auth: try alternate provider if available, otherwise fail fast
    /// - retryable (429/5xx/network): exponential backoff with jitter
    /// - cancelled: no retry
    pub async fn stream_chat(
        &self,
        req: LlmRequest,
        cancel: CancelToken,
    ) -> Result<LlmEventStream, LlmError> {
        let provider_pool = self.find_providers(&req.provider_name);
        if provider_pool.is_empty() {
            return Err(LlmError::Unknown {
                message: format!("no provider registered for '{}'", req.provider_name),
                provider: req.provider_name,
            });
        }

        let start_idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) % provider_pool.len();
        let provider = provider_pool[start_idx].clone();

        let mut last_error: Option<LlmError> = None;

        for attempt in 0..=self.retry_config.max_retries {
            if *cancel.borrow() {
                return Err(LlmError::Cancelled {
                    provider: provider.name().to_string(),
                });
            }

            if attempt > 0 {
                let delay = self.retry_config.delay_ms(attempt - 1);
                tracing::info!(
                    target: "llm::router",
                    attempt,
                    delay_ms = delay,
                    provider = provider.name(),
                    "retrying LLM request"
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }

            match provider.stream_chat(req.clone(), cancel.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    tracing::warn!(
                        target: "llm::router",
                        attempt,
                        error = %e,
                        retryable = e.is_retryable(),
                        "LLM request failed"
                    );

                    if e.is_context_limit() {
                        return Err(e);
                    }

                    if e.is_cancelled() {
                        return Err(e);
                    }

                    if e.is_auth() {
                        // First try same-provider pool fallback (another base_url instance)
                        if provider_pool.len() > 1 {
                            for i in 1..provider_pool.len() {
                                let idx = (start_idx + i) % provider_pool.len();
                                let alt = provider_pool[idx];
                                tracing::info!(
                                    target: "llm::router",
                                    from = provider.name(),
                                    to = alt.name(),
                                    strategy = "same_provider_pool",
                                    "falling back after auth error"
                                );

                                let mut alt_req = req.clone();
                                alt_req.provider_name = alt.name().to_string();
                                if let Ok(stream) = alt.stream_chat(alt_req, cancel.clone()).await {
                                    return Ok(stream);
                                }
                            }
                        }

                        // Then try cross-provider fallback
                        let mut last_auth_err = e.clone();
                        for alt in self.find_alternates(&req.provider_name) {
                            tracing::info!(
                                target: "llm::router",
                                from = provider.name(),
                                to = alt.name(),
                                strategy = "cross_provider",
                                "falling back after auth error"
                            );
                            let mut alt_req = req.clone();
                            alt_req.provider_name = alt.name().to_string();
                            match alt.stream_chat(alt_req, cancel.clone()).await {
                                Ok(stream) => return Ok(stream),
                                Err(err) => {
                                    if err.is_auth() {
                                        last_auth_err = err;
                                        continue;
                                    }
                                    return Err(err);
                                }
                            }
                        }

                        return Err(last_auth_err);
                    }

                    if !e.is_retryable() {
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Unknown {
            message: "all retries exhausted".to_string(),
            provider: req.provider_name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_no_jitter() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1000,
            factor: 2.0,
            jitter: false,
        };

        assert_eq!(config.delay_ms(0), 1000); // 1000 * 2^0
        assert_eq!(config.delay_ms(1), 2000); // 1000 * 2^1
        assert_eq!(config.delay_ms(2), 4000); // 1000 * 2^2
    }

    #[test]
    fn test_retry_delay_with_jitter() {
        let config = RetryConfig::default();

        // With jitter, delay should be in range [500, 1500] for attempt 0
        let delay = config.delay_ms(0);
        assert!(delay >= 500 && delay <= 1500, "delay was {delay}");
    }

    #[test]
    fn test_worker_config() {
        let config = RetryConfig::worker();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay_ms, 2000);
    }

    #[test]
    fn test_interactive_config() {
        let config = RetryConfig::interactive();
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.base_delay_ms, 500);
    }
}
