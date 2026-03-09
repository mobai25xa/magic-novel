use async_trait::async_trait;

use crate::agent_engine::compaction::{CompactionSummarizer, TruncationSummarizer};
use crate::agent_engine::messages::{AgentMessage, CompactionSummary};
use crate::models::AppError;

pub struct LlmCompactionSummarizer {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

#[async_trait]
impl CompactionSummarizer for LlmCompactionSummarizer {
    async fn summarize(
        &self,
        existing_summary: Option<&CompactionSummary>,
        messages: &[AgentMessage],
    ) -> Result<String, AppError> {
        if self.base_url.trim().is_empty() || self.api_key.trim().is_empty() {
            return TruncationSummarizer
                .summarize(existing_summary, messages)
                .await;
        }

        let fallback = TruncationSummarizer;
        let fallback_summary = fallback
            .summarize(existing_summary, messages)
            .await
            .unwrap_or_else(|_| format!("[Compacted {} earlier messages]", messages.len()));

        let prompt = build_summary_prompt(
            &self.provider,
            &self.model,
            existing_summary,
            messages,
            &fallback_summary,
        );
        let request = reqwest::Client::new()
            .post(compact_chat_url(&self.base_url))
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": "You summarize agent conversation context. Return concise plain text, preserving decisions, tool outputs, constraints, open TODOs, and verified facts."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.1,
            }))
            .send()
            .await;

        let response = match request {
            Ok(resp) => resp,
            Err(_) => return Ok(fallback_summary),
        };

        if !response.status().is_success() {
            return Ok(fallback_summary);
        }

        let payload = match response.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(_) => return Ok(fallback_summary),
        };

        let text = payload
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback_summary);

        Ok(text)
    }
}

fn compact_chat_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    }
}

fn build_summary_prompt(
    provider: &str,
    model: &str,
    existing_summary: Option<&CompactionSummary>,
    messages: &[AgentMessage],
    fallback_summary: &str,
) -> String {
    let mut sections = Vec::new();
    if let Some(prev) = existing_summary {
        sections.push(format!("Previous summary:\n{}", prev.summary_text));
    }

    let delta = messages
        .iter()
        .map(|msg| format!("- {:?}: {}", msg.role, msg.text_content()))
        .collect::<Vec<_>>()
        .join("\n");

    sections.push(format!("New messages:\n{}", delta));
    sections.push(format!("Fallback summary hint:\n{}", fallback_summary));

    format!(
        "Provider: {provider}\nModel: {model}\nPlease produce an updated compact summary with sections: Goals, Decisions, Entities, Recent Actions, Open TODOs, Verified Facts.\n\n{}",
        sections.join("\n\n")
    )
}
