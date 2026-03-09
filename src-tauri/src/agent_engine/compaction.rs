//! Agent Engine - Context compaction (threshold + fallback truncation)
//!
//! Aligned with docs/magic_plan/plan_agent/04-context-compaction-and-memory.md
//!
//! Provides:
//! - Threshold detection with token estimation
//! - CompactionSummarizer trait (for Dev2 to implement LLM summary)
//! - Simple truncation fallback (keep recent N messages)
//! - Compaction events (COMPACTION_STARTED/FINISHED)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::AppError;

use super::emitter::EventSink;
use super::messages::{AgentMessage, CompactionSummary, ConversationState, Role};
use super::text_utils::truncate_chars;
use super::types::{DEFAULT_MODEL, DEFAULT_PROVIDER};

/// Configuration for compaction behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Estimated token threshold for preemptive compaction
    pub token_threshold: u64,
    /// Number of recent messages to keep after compaction
    pub keep_recent_count: usize,
    /// Preferred provider for compaction summarization.
    pub summary_provider: String,
    /// Preferred model for compaction summarization.
    pub summary_model: String,
    /// Token budget reserved for system injections (project context + editor state + guidelines + tool descriptions)
    pub reserved_injection_tokens: u64,
    /// Maximum accumulated summary length in chars (stable-prefix truncation)
    pub max_summary_chars: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            token_threshold: 80_000,
            keep_recent_count: 6,
            summary_provider: DEFAULT_PROVIDER.to_string(),
            summary_model: DEFAULT_MODEL.to_string(),
            reserved_injection_tokens: 1500,
            max_summary_chars: 4000,
        }
    }
}

/// Trait for LLM-based summarization (to be implemented by Dev2).
///
/// The default implementation is a simple no-op; compaction will fall back
/// to truncation.
#[async_trait]
pub trait CompactionSummarizer: Send + Sync {
    /// Generate a structured summary of the given messages.
    async fn summarize(
        &self,
        existing_summary: Option<&CompactionSummary>,
        messages: &[AgentMessage],
    ) -> Result<String, AppError>;
}

/// Fallback summarizer that just concatenates key points from text messages.
pub struct TruncationSummarizer;

#[async_trait]
impl CompactionSummarizer for TruncationSummarizer {
    async fn summarize(
        &self,
        existing_summary: Option<&CompactionSummary>,
        messages: &[AgentMessage],
    ) -> Result<String, AppError> {
        let mut summary_parts = Vec::new();
        if let Some(prev) = existing_summary {
            summary_parts.push(format!(
                "Previous summary (anchor={}): {}",
                prev.anchor_message_id, prev.summary_text
            ));
        }
        for msg in messages {
            match msg.role {
                Role::User => {
                    let text = msg.text_content();
                    if !text.is_empty() {
                        let preview = truncate_chars(&text, 200);
                        summary_parts.push(format!("User: {preview}"));
                    }
                }
                Role::Assistant => {
                    let text = msg.text_content();
                    if !text.is_empty() {
                        let preview = truncate_chars(&text, 200);
                        summary_parts.push(format!("Assistant: {preview}"));
                    }
                    // Note tool calls
                    let tools = msg.tool_calls();
                    if !tools.is_empty() {
                        let names: Vec<_> = tools.iter().map(|t| t.tool_name.as_str()).collect();
                        summary_parts.push(format!("Tools used: {}", names.join(", ")));
                    }
                }
                _ => {}
            }
        }

        Ok(summary_parts.join("\n"))
    }
}

pub fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

/// Estimate token count with CJK-aware heuristics.
/// CJK chars are counted as 1 token/char; non-CJK chars as 1 token/4 chars.
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }

    let mut cjk_chars = 0_u64;
    let mut other_chars = 0_u64;

    for ch in text.chars() {
        if is_cjk(ch) {
            cjk_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    cjk_chars + ((other_chars + 3) / 4)
}

/// Check if compaction should be triggered.
pub fn should_compact(state: &ConversationState, config: &CompactionConfig) -> bool {
    let available = config
        .token_threshold
        .saturating_sub(config.reserved_injection_tokens);
    state.total_estimated_tokens() >= available
}

/// Check if an error indicates context_limit exceeded.
pub fn is_context_limit_error(err: &AppError) -> bool {
    if let Some(details) = &err.details {
        if let Some(code) = details.get("code").and_then(|v| v.as_str()) {
            return code == "E_CONTEXT_LIMIT";
        }
    }
    err.message.contains("context_length_exceeded")
        || err.message.contains("maximum context length")
}

/// Perform compaction on the conversation state.
///
/// Uses the provided summarizer (or falls back to truncation).
pub async fn compact<S: EventSink>(
    state: &mut ConversationState,
    emitter: &S,
    summarizer: &dyn CompactionSummarizer,
    config: &CompactionConfig,
    reason: &str,
) -> Result<(), AppError> {
    let msg_count = state.messages.len();
    if msg_count <= config.keep_recent_count {
        // Not enough messages to compact
        return Ok(());
    }

    emitter.compaction_started(reason)?;

    let split_at = msg_count.saturating_sub(config.keep_recent_count);
    let old_messages = &state.messages[..split_at];
    let recent_messages = state.messages[split_at..].to_vec();

    // Find anchor (last message before split)
    let anchor_id = old_messages
        .last()
        .map(|m| m.id.clone())
        .unwrap_or_default();

    // Generate summary (incremental if a previous summary exists)
    let summary_text = match summarizer
        .summarize(state.last_compaction.as_ref(), old_messages)
        .await
    {
        Ok(text) => {
            // Stable-prefix: append new summary to existing, separated by ---
            if let Some(prev) = &state.last_compaction {
                let combined = format!("{}\n\n---\n\n{}", prev.summary_text, text);
                // Truncate oldest segments if exceeding max_summary_chars
                if combined.chars().count() > config.max_summary_chars {
                    truncate_summary_prefix(&combined, config.max_summary_chars)
                } else {
                    combined
                }
            } else {
                text
            }
        }
        Err(e) => {
            tracing::warn!(
                target: "agent_engine",
                error = %e,
                "summarizer failed, using basic truncation"
            );
            format!("[Compacted {} earlier messages]", old_messages.len())
        }
    };

    let compaction_summary = CompactionSummary {
        summary_text: summary_text.clone(),
        anchor_message_id: anchor_id,
        removed_count: old_messages.len(),
        keep_recent_count: config.keep_recent_count,
        ts: chrono::Utc::now().timestamp_millis(),
    };

    // Replace old messages with summary as a system message
    let mut new_messages = Vec::new();

    // Keep original system prompt if present
    if let Some(first) = state.messages.first() {
        if first.role == Role::System {
            new_messages.push(first.clone());
        }
    }

    // Add compaction summary as system message
    new_messages.push(AgentMessage::system(format!(
        "[Conversation Summary]\n{summary_text}"
    )));

    // Add recent messages
    new_messages.extend(recent_messages);

    state.messages = new_messages;
    state.last_compaction = Some(compaction_summary.clone());

    emitter.compaction_finished(json!({
        "removed_count": compaction_summary.removed_count,
        "keep_recent_count": compaction_summary.keep_recent_count,
        "summary_length": compaction_summary.summary_text.len(),
        "summary_provider": config.summary_provider,
        "summary_model": config.summary_model,
    }))?;

    tracing::info!(
        target: "agent_engine",
        removed = compaction_summary.removed_count,
        kept = config.keep_recent_count,
        reason = reason,
        "compaction completed"
    );

    Ok(())
}

/// Truncate the oldest summary segments to fit within max_chars.
/// Segments are separated by `\n\n---\n\n`. Drops oldest segments first.
fn truncate_summary_prefix(combined: &str, max_chars: usize) -> String {
    let segments: Vec<&str> = combined.split("\n\n---\n\n").collect();
    let mut result = String::new();
    // Keep segments from the end (newest first)
    for seg in segments.iter().rev() {
        let candidate = if result.is_empty() {
            seg.to_string()
        } else {
            format!("{}\n\n---\n\n{}", seg, result)
        };
        if candidate.chars().count() > max_chars {
            break;
        }
        result = candidate;
    }
    if result.is_empty() {
        // Even the newest segment is too long, truncate it
        truncate_chars(segments.last().unwrap_or(&""), max_chars)
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_english() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(estimate_tokens("hello world"), 3);
    }

    #[test]
    fn test_estimate_tokens_cjk() {
        assert_eq!(estimate_tokens("你好世界"), 4);
        assert_eq!(estimate_tokens("中文测试"), 4);
    }

    #[test]
    fn test_estimate_tokens_mixed() {
        assert_eq!(estimate_tokens("你好ab"), 3);
        assert_eq!(estimate_tokens("中a文b"), 3);
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let state = ConversationState::new("test".to_string());
        let config = CompactionConfig::default();
        assert!(!should_compact(&state, &config));
    }

    #[test]
    fn test_should_compact_cjk_above_threshold() {
        let mut state = ConversationState::new("test".to_string());
        state.messages.push(AgentMessage::user("中".repeat(81_000)));
        let config = CompactionConfig::default();
        assert!(should_compact(&state, &config));
    }

    #[test]
    fn test_should_compact_english_above_threshold() {
        let mut state = ConversationState::new("test".to_string());
        state.messages.push(AgentMessage::user("x".repeat(320_100)));
        let config = CompactionConfig::default();
        assert!(should_compact(&state, &config));
    }

    #[test]
    fn test_is_context_limit_error() {
        let err = AppError {
            code: crate::models::ErrorCode::Internal,
            message: "context_length_exceeded".to_string(),
            details: Some(json!({"code": "E_CONTEXT_LIMIT"})),
            recoverable: Some(true),
        };
        assert!(is_context_limit_error(&err));

        let err2 = AppError::internal("some other error");
        assert!(!is_context_limit_error(&err2));
    }

    #[test]
    fn test_should_compact_respects_reserved_injection_tokens() {
        let mut state = ConversationState::new("test".to_string());
        // 78,600 CJK chars = 78,600 tokens, above 80,000 - 1,500 = 78,500
        state.messages.push(AgentMessage::user("中".repeat(78_600)));
        let config = CompactionConfig::default();
        assert!(should_compact(&state, &config));
    }

    #[test]
    fn test_truncate_summary_prefix_keeps_newest() {
        let combined = "old segment\n\n---\n\nmiddle segment\n\n---\n\nnew segment";
        let result = truncate_summary_prefix(combined, 40);
        assert!(result.contains("new segment"));
        assert!(!result.contains("old segment"));
    }

    #[test]
    fn test_truncate_summary_prefix_keeps_all_when_fits() {
        let combined = "seg1\n\n---\n\nseg2";
        let result = truncate_summary_prefix(combined, 1000);
        assert_eq!(result, combined);
    }
}
