//! LLM Layer - Error types and classification
//!
//! Aligned with docs/magic_plan/plan_agent/02-llm-providers-and-streaming-accumulator.md

use std::fmt;

use regex::Regex;
use serde_json::json;

use crate::models::{AppError, ErrorCode};

/// LLM error with classification for retry/routing decisions
#[derive(Debug, Clone)]
pub enum LlmError {
    /// Context window exceeded - do NOT retry, trigger compaction
    ContextLimit { message: String, provider: String },
    /// Authentication error (401/403) - fast fail or try alternate provider
    Auth { message: String, provider: String },
    /// Rate limit (429) - retryable with backoff
    RateLimit {
        message: String,
        provider: String,
        retry_after_ms: Option<u64>,
    },
    /// Server error (5xx) - retryable
    ServerError {
        status: u16,
        message: String,
        provider: String,
    },
    /// Network/connection error - retryable
    Network { message: String, provider: String },
    /// Empty response body - retryable
    EmptyBody { provider: String },
    /// Model finished turn without returning text/tool calls (semantic empty response)
    EmptyResponse { provider: String },
    /// Request was cancelled
    Cancelled { provider: String },
    /// JSON parse error in response
    ParseError { message: String, provider: String },
    /// Provider rejected tool schema before generation started
    ProviderToolSchema {
        message: String,
        provider: String,
        tool_name: Option<String>,
        schema_path: Option<String>,
        status: Option<u16>,
    },
    /// Provider rejected the prompt/output for safety or content-policy reasons
    ContentPolicy {
        message: String,
        provider: String,
        status: Option<u16>,
    },
    /// Generic/unknown error
    Unknown { message: String, provider: String },
}

impl LlmError {
    /// Whether this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. }
                | Self::ServerError { .. }
                | Self::Network { .. }
                | Self::EmptyBody { .. }
                | Self::EmptyResponse { .. }
        )
    }

    /// Whether this is a context limit error (needs compaction, not retry)
    pub fn is_context_limit(&self) -> bool {
        matches!(self, Self::ContextLimit { .. })
    }

    /// Whether this is an auth error (try alternate provider or fast fail)
    pub fn is_auth(&self) -> bool {
        matches!(self, Self::Auth { .. })
    }

    /// Whether this was a cancellation
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        match self {
            Self::ContextLimit { provider, .. } => provider,
            Self::Auth { provider, .. } => provider,
            Self::RateLimit { provider, .. } => provider,
            Self::ServerError { provider, .. } => provider,
            Self::Network { provider, .. } => provider,
            Self::EmptyBody { provider } => provider,
            Self::EmptyResponse { provider } => provider,
            Self::Cancelled { provider } => provider,
            Self::ParseError { provider, .. } => provider,
            Self::ProviderToolSchema { provider, .. } => provider,
            Self::ContentPolicy { provider, .. } => provider,
            Self::Unknown { provider, .. } => provider,
        }
    }

    /// Sanitized provider diagnostic text (safe for logs/details)
    pub fn diagnostic_message(&self) -> String {
        match self {
            Self::ContextLimit { message, .. }
            | Self::Auth { message, .. }
            | Self::RateLimit { message, .. }
            | Self::Network { message, .. }
            | Self::ParseError { message, .. }
            | Self::ProviderToolSchema { message, .. }
            | Self::ContentPolicy { message, .. }
            | Self::Unknown { message, .. } => message.clone(),
            Self::ServerError {
                status, message, ..
            } => {
                format!("HTTP {status}: {message}")
            }
            Self::EmptyBody { .. } => "empty response body".to_string(),
            Self::EmptyResponse { .. } => "empty model response".to_string(),
            Self::Cancelled { .. } => "request cancelled".to_string(),
        }
    }

    /// Classify an HTTP status code + response body into an LlmError
    pub fn from_http_response(status: u16, body: &str, provider: &str) -> Self {
        // Context limit detection
        if body.contains("context_length_exceeded")
            || body.contains("maximum context length")
            || body.contains("token limit")
            || body.contains("max_tokens")
        {
            return Self::ContextLimit {
                message: truncate_body(body),
                provider: provider.to_string(),
            };
        }

        if let Some((tool_name, schema_path)) = detect_tool_schema_rejection(body) {
            return Self::ProviderToolSchema {
                message: truncate_body(body),
                provider: provider.to_string(),
                tool_name,
                schema_path,
                status: Some(status),
            };
        }

        if detect_content_policy_rejection(body) {
            return Self::ContentPolicy {
                message: truncate_body(body),
                provider: provider.to_string(),
                status: Some(status),
            };
        }

        match status {
            401 | 403 => Self::Auth {
                message: truncate_body(body),
                provider: provider.to_string(),
            },
            429 => {
                // Try to extract retry-after from body
                let retry_after = extract_retry_after(body);
                Self::RateLimit {
                    message: truncate_body(body),
                    provider: provider.to_string(),
                    retry_after_ms: retry_after,
                }
            }
            500..=599 => Self::ServerError {
                status,
                message: truncate_body(body),
                provider: provider.to_string(),
            },
            _ => Self::Unknown {
                message: format!("HTTP {status}: {}", truncate_body(body)),
                provider: provider.to_string(),
            },
        }
    }

    /// Get the error code string for event protocol
    pub fn error_code(&self) -> &str {
        match self {
            Self::ContextLimit { .. } => "E_CONTEXT_LIMIT",
            Self::Auth { .. } => "E_AUTH",
            Self::RateLimit { .. } => "E_RATE_LIMIT",
            Self::ServerError { .. } => "E_SERVER_ERROR",
            Self::Network { .. } => "E_NETWORK",
            Self::EmptyBody { .. } => "E_EMPTY_BODY",
            Self::EmptyResponse { .. } => "E_EMPTY_RESPONSE",
            Self::Cancelled { .. } => "E_CANCELLED",
            Self::ParseError { .. } => "E_PARSE_ERROR",
            Self::ProviderToolSchema { .. } => "E_PROVIDER_TOOL_SCHEMA",
            Self::ContentPolicy { .. } => "E_MODEL_CONTENT_REJECTED",
            Self::Unknown { .. } => "E_LLM_UNKNOWN",
        }
    }

    /// Suggested error category for frontend UI classification
    pub fn category_hint(&self) -> &str {
        match self {
            Self::Auth { .. } => "auth",
            Self::RateLimit { .. } => "rate_limit",
            Self::ServerError { .. } | Self::EmptyBody { .. } | Self::EmptyResponse { .. } => {
                "server"
            }
            Self::Network { .. } => "network",
            Self::ContextLimit { .. } => "context_limit",
            Self::ParseError { .. } | Self::Unknown { .. } => "client",
            Self::ProviderToolSchema { .. } => "tool_schema",
            Self::ContentPolicy { .. } => "model_content",
            Self::Cancelled { .. } => "cancelled",
        }
    }

    /// Build structured error detail for TURN_FAILED event payload
    pub fn to_event_detail(&self) -> serde_json::Value {
        let mut detail = json!({
            "provider": self.provider(),
            "retryable": self.is_retryable(),
            "diagnostic": self.diagnostic_message(),
            "category_hint": self.category_hint(),
        });

        if let Self::ServerError { status, .. } = self {
            detail["http_status"] = json!(status);
        }
        if let Self::RateLimit { retry_after_ms, .. } = self {
            detail["retry_after_ms"] = json!(retry_after_ms);
        }
        if let Self::ProviderToolSchema {
            tool_name,
            schema_path,
            status,
            ..
        } = self
        {
            detail["tool_name"] = json!(tool_name);
            detail["schema_path"] = json!(schema_path);
            detail["http_status"] = json!(status);
        }
        if let Self::ContentPolicy { status, .. } = self {
            detail["http_status"] = json!(status);
        }

        detail
    }
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContextLimit { provider, .. } => {
                write!(f, "[{provider}] context limit exceeded")
            }
            Self::Auth { provider, .. } => {
                write!(f, "[{provider}] authentication failed")
            }
            Self::RateLimit { provider, .. } => {
                write!(f, "[{provider}] rate limited")
            }
            Self::ServerError {
                status, provider, ..
            } => {
                write!(f, "[{provider}] server error {status}")
            }
            Self::Network { provider, .. } => {
                write!(f, "[{provider}] network error")
            }
            Self::EmptyBody { provider } => {
                write!(f, "[{provider}] empty response body")
            }
            Self::EmptyResponse { provider } => {
                write!(f, "[{provider}] empty model response")
            }
            Self::Cancelled { provider } => {
                write!(f, "[{provider}] request cancelled")
            }
            Self::ParseError { provider, .. } => {
                write!(f, "[{provider}] parse error")
            }
            Self::ProviderToolSchema { provider, .. } => {
                write!(f, "[{provider}] provider rejected tool schema")
            }
            Self::ContentPolicy { provider, .. } => {
                write!(f, "[{provider}] content policy rejection")
            }
            Self::Unknown { provider, .. } => {
                write!(f, "[{provider}] upstream error")
            }
        }
    }
}

impl std::error::Error for LlmError {}

impl From<LlmError> for AppError {
    fn from(e: LlmError) -> Self {
        let code = e.error_code().to_string();
        let recoverable = e.is_retryable() || e.is_context_limit();
        let message = e.to_string();
        let event_detail = e.to_event_detail();

        AppError {
            code: ErrorCode::Internal,
            message,
            details: Some(json!({
                "code": code,
                "provider": event_detail["provider"],
                "retryable": event_detail["retryable"],
                "diagnostic": event_detail["diagnostic"],
                "category_hint": event_detail["category_hint"],
                "http_status": event_detail.get("http_status"),
                "retry_after_ms": event_detail.get("retry_after_ms"),
                "tool_name": event_detail.get("tool_name"),
                "schema_path": event_detail.get("schema_path"),
            })),
            recoverable: Some(recoverable),
        }
    }
}

fn detect_tool_schema_rejection(body: &str) -> Option<(Option<String>, Option<String>)> {
    let lower = body.to_ascii_lowercase();
    let looks_like_schema_error = lower.contains("invalid schema for function")
        || lower.contains("function.parameters")
        || lower.contains("invalid_function_parameters")
        || (lower.contains("tool") && lower.contains("schema") && lower.contains("function"));

    if !looks_like_schema_error {
        return None;
    }

    Some((extract_tool_name(body), extract_schema_path(body)))
}

fn detect_content_policy_rejection(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("content_policy")
        || lower.contains("content policy")
        || lower.contains("content filter")
        || lower.contains("usage policies")
        || lower.contains("safety system")
}

fn extract_tool_name(body: &str) -> Option<String> {
    static TOOL_NAME_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = TOOL_NAME_RE.get_or_init(|| {
        Regex::new(r#"(?i)function\s+['\"]([^'\"]+)['\"]"#).expect("valid tool name regex")
    });

    re.captures(body)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_schema_path(body: &str) -> Option<String> {
    static SCHEMA_PATH_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = SCHEMA_PATH_RE.get_or_init(|| {
        Regex::new(r#"(?i)(function\.parameters[^\s,;\]\}]+|tools\[[0-9]+\][^\s,;\]\}]+|\$\.[A-Za-z0-9_\.\[\]-]+)"#)
            .expect("valid schema path regex")
    });

    re.captures(body)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn truncate_body(body: &str) -> String {
    let plain = body.trim();
    if plain.is_empty() {
        return "upstream returned empty body".to_string();
    }

    if looks_like_html(plain) {
        return "upstream returned non-JSON error body".to_string();
    }

    sanitize_plain_text(plain, 180)
}

fn sanitize_plain_text(input: &str, max_len: usize) -> String {
    let line = input
        .replace('\n', " ")
        .replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if line.len() <= max_len {
        line
    } else {
        format!("{}...", &line[..max_len])
    }
}

fn looks_like_html(input: &str) -> bool {
    let lower = input.trim_start().to_ascii_lowercase();
    if lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || lower.starts_with("<head")
        || lower.starts_with("<body")
    {
        return true;
    }

    static HTML_TAG_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = HTML_TAG_RE.get_or_init(|| {
        Regex::new(r"(?is)<\s*(html|head|body|script|style|div|span|meta|title)\b")
            .expect("valid html regex")
    });
    re.is_match(input)
}

fn extract_retry_after(body: &str) -> Option<u64> {
    // Try to parse retry_after from JSON body
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(secs) = v.get("retry_after").and_then(|v| v.as_f64()) {
            return Some((secs * 1000.0) as u64);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let err = LlmError::from_http_response(429, "rate limited", "openai");
        assert!(err.is_retryable());
        assert!(!err.is_context_limit());
        assert!(!err.is_auth());

        let err = LlmError::from_http_response(
            400,
            "context_length_exceeded: max 128000 tokens",
            "openai",
        );
        assert!(err.is_context_limit());
        assert!(!err.is_retryable());

        let err = LlmError::from_http_response(401, "invalid api key", "openai");
        assert!(err.is_auth());
        assert!(!err.is_retryable());

        let err = LlmError::from_http_response(500, "internal error", "openai");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_to_app_error() {
        let err = LlmError::ContextLimit {
            message: "too many tokens".to_string(),
            provider: "openai".to_string(),
        };
        let app_err: AppError = err.into();
        assert!(app_err.recoverable.unwrap_or(false));
        assert!(app_err.message.contains("context limit"));
    }

    #[test]
    fn test_truncate_body_html_sanitized() {
        let msg = truncate_body("<html><body>cloudflare challenge</body></html>");
        assert_eq!(msg, "upstream returned non-JSON error body");
    }

    #[test]
    fn test_truncate_body_plain_text_trimmed() {
        let msg = truncate_body("upstream timeout and retry later");
        assert!(msg.contains("upstream timeout"));
        assert!(!msg.contains('<'));
    }

    #[test]
    fn test_category_hint_mapping() {
        let err = LlmError::Auth {
            message: "bad key".into(),
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "auth");

        let err = LlmError::RateLimit {
            message: "slow down".into(),
            provider: "openai".into(),
            retry_after_ms: Some(5000),
        };
        assert_eq!(err.category_hint(), "rate_limit");

        let err = LlmError::ServerError {
            status: 500,
            message: "oops".into(),
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "server");

        let err = LlmError::Network {
            message: "timeout".into(),
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "network");

        let err = LlmError::ContextLimit {
            message: "too long".into(),
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "context_limit");

        let err = LlmError::ParseError {
            message: "bad json".into(),
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "client");

        let err = LlmError::EmptyResponse {
            provider: "openai".into(),
        };
        assert_eq!(err.category_hint(), "server");
    }

    #[test]
    fn test_to_event_detail_includes_structured_fields() {
        let err = LlmError::ServerError {
            status: 502,
            message: "bad gateway".into(),
            provider: "openai-compatible".into(),
        };
        let detail = err.to_event_detail();
        assert_eq!(detail["provider"], "openai-compatible");
        assert_eq!(detail["retryable"], true);
        assert_eq!(detail["category_hint"], "server");
        assert_eq!(detail["http_status"], 502);
        assert!(detail["diagnostic"].as_str().unwrap().contains("502"));
    }

    #[test]
    fn test_to_event_detail_rate_limit_includes_retry_after() {
        let err = LlmError::RateLimit {
            message: "slow".into(),
            provider: "openai".into(),
            retry_after_ms: Some(3000),
        };
        let detail = err.to_event_detail();
        assert_eq!(detail["retry_after_ms"], 3000);
        assert_eq!(detail["category_hint"], "rate_limit");
    }

    #[test]
    fn test_http_response_classifies_tool_schema_rejection() {
        let err = LlmError::from_http_response(
            400,
            "Invalid schema for function 'edit': function.parameters.ops.items uses unsupported keyword 'oneOf'",
            "openai-compatible",
        );

        match err {
            LlmError::ProviderToolSchema {
                tool_name,
                schema_path,
                status,
                ..
            } => {
                assert_eq!(tool_name.as_deref(), Some("edit"));
                assert_eq!(
                    schema_path.as_deref(),
                    Some("function.parameters.ops.items")
                );
                assert_eq!(status, Some(400));
            }
            other => panic!("expected ProviderToolSchema, got {other:?}"),
        }
    }

    #[test]
    fn test_http_response_classifies_content_policy_rejection() {
        let err = LlmError::from_http_response(
            400,
            "This request violates our usage policies and was blocked by the safety system.",
            "openai-compatible",
        );

        assert!(matches!(err, LlmError::ContentPolicy { .. }));
        assert_eq!(err.error_code(), "E_MODEL_CONTENT_REJECTED");
        assert_eq!(err.category_hint(), "model_content");
    }
}
