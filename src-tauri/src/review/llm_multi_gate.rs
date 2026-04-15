use std::collections::HashSet;
use std::path::Path;

use futures::StreamExt;
use serde::Deserialize;

use crate::agent_engine::messages::AgentMessage;
use crate::kernel::search::corpus_extract::extract_tiptap_text;
use crate::llm::accumulator::StreamAccumulator;
use crate::llm::provider::new_cancel_token;
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::types::{LlmRequest, SystemBlock, ToolChoice};
use crate::mission::contextpack_types::ContextPack;
use crate::models::{AppError, ErrorCode};
use crate::review::types::{ReviewConfidence, ReviewIssue, ReviewSeverity, ReviewType};
use crate::services::read_json;

use super::target_ref::resolve_review_target_path;

#[derive(Debug, Clone)]
pub struct ReviewLlmConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct SemanticReviewOutput {
    pub issues: Vec<ReviewIssue>,
    pub evidence_summary: Vec<String>,
    pub implemented_review_types: Vec<ReviewType>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct EvidenceEntry {
    id: String,
    source_ref: String,
    snippet: String,
    reason: String,
    score: f32,
}

#[derive(Debug, Clone)]
struct EvidenceBundle {
    items: Vec<EvidenceEntry>,
    valid_ids: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct RawLlmIssue {
    review_type: String,
    severity: String,
    confidence: String,
    summary: String,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    suggested_fix: Option<String>,
    #[serde(default)]
    auto_fixable: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawLlmReviewResponse {
    #[serde(default)]
    issues: Vec<RawLlmIssue>,
}

pub fn review_llm_enabled() -> bool {
    match std::env::var("MAGIC_REVIEW_LLM") {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no" | "disabled"
        ),
        Err(_) => true,
    }
}

pub async fn run_semantic_review(
    project_path: &Path,
    target_refs: &[String],
    review_types: &[ReviewType],
    contextpack: &ContextPack,
    llm_config: &ReviewLlmConfig,
) -> Result<SemanticReviewOutput, AppError> {
    let evidence = build_evidence_bundle(project_path, target_refs, contextpack)?;
    if evidence.items.is_empty() {
        return Err(review_evidence_unavailable(
            "review evidence bundle is empty",
            Some("empty_evidence_bundle"),
        ));
    }

    let router = build_router(
        &llm_config.provider,
        llm_config.base_url.clone(),
        llm_config.api_key.clone(),
        RetryConfig::worker(),
    );

    let system_text = concat!(
        "You are a strict fiction review gate. Output only JSON with shape {\"issues\":[...]}. ",
        "Do not use markdown fences. Do not call tools. Each issue must reference evidence IDs from the bundle. ",
        "Allowed review_type values: continuity, logic, character, style, terminology, foreshadow, objective_completion. ",
        "Allowed severity values: warn, block. Allowed confidence values: low, medium, high."
    );

    let user_text = build_user_prompt(target_refs, review_types, contextpack, &evidence.items)?;

    let request = LlmRequest {
        provider_name: llm_config.provider.clone(),
        model: llm_config.model.clone(),
        system: vec![SystemBlock {
            text: system_text.to_string(),
            cache_control: None,
        }],
        messages: vec![AgentMessage::user(user_text)],
        tools: Vec::new(),
        tool_choice: ToolChoice::None,
        parallel_tool_calls: false,
        temperature: 0.1,
        reasoning: None,
    };

    let (_cancel_tx, cancel_rx) = new_cancel_token();
    let mut stream = router
        .stream_chat(request, cancel_rx)
        .await
        .map_err(review_llm_error)?;

    let mut accumulator = StreamAccumulator::new();
    while let Some(event) = stream.next().await {
        let event = event.map_err(review_llm_error)?;
        accumulator.apply(&event);
    }

    let turn_output = accumulator
        .into_turn_output()
        .map_err(review_llm_parse_error)?;
    if !turn_output.tool_calls.is_empty() {
        return Err(review_evidence_unavailable(
            "review model attempted tool calls",
            Some("tool_calls_not_allowed"),
        ));
    }

    let assistant_text = turn_output.assistant_message.text_content();
    let raw = parse_llm_review_response(&assistant_text)?;
    let issues = normalize_llm_issues(raw.issues, &evidence.valid_ids, review_types);

    Ok(SemanticReviewOutput {
        evidence_summary: vec![format!(
            "semantic_review: evidence_items={} semantic_review_types={} llm_provider={} model={}",
            evidence.items.len(),
            review_types
                .iter()
                .map(review_type_as_str)
                .collect::<Vec<_>>()
                .join(","),
            llm_config.provider,
            llm_config.model
        )],
        issues,
        implemented_review_types: review_types.to_vec(),
    })
}

fn build_user_prompt(
    target_refs: &[String],
    review_types: &[ReviewType],
    contextpack: &ContextPack,
    evidence: &[EvidenceEntry],
) -> Result<String, AppError> {
    let payload = serde_json::json!({
        "scope_ref": contextpack.scope_ref,
        "target_refs": target_refs,
        "review_types": review_types.iter().map(review_type_as_str).collect::<Vec<_>>(),
        "objective_summary": contextpack.objective_summary,
        "must_keep": contextpack.must_keep,
        "active_constraints": contextpack.active_constraints,
        "style_rules": contextpack.style_rules,
        "key_facts": contextpack.key_facts,
        "cast_notes": contextpack.cast_notes,
        "review_targets": contextpack.review_targets,
        "evidence": evidence,
        "instructions": {
            "max_issues": 20,
            "must_include_target_evidence": true,
            "block_conflict_requires_cross_evidence_for": ["continuity", "character", "foreshadow", "terminology"]
        }
    });

    serde_json::to_string(&payload).map_err(Into::into)
}

fn build_evidence_bundle(
    project_path: &Path,
    target_refs: &[String],
    contextpack: &ContextPack,
) -> Result<EvidenceBundle, AppError> {
    let mut items = Vec::new();
    let mut valid_ids = HashSet::new();

    let mut push = |prefix: &str,
                    idx: usize,
                    source_ref: String,
                    snippet: String,
                    reason: String,
                    score: f32| {
        let snippet = truncate_chars(snippet.trim(), 320);
        if snippet.is_empty() {
            return;
        }
        let id = format!("{}{}", prefix, idx);
        valid_ids.insert(id.clone());
        items.push(EvidenceEntry {
            id,
            source_ref,
            snippet,
            reason,
            score,
        });
    };

    let mut t_idx = 1;
    for target_ref in target_refs {
        let text = load_target_text(project_path, target_ref)?;
        let head = head_chars(&text, 320);
        let tail = tail_chars(&text, 320);
        push(
            "T",
            t_idx,
            target_ref.clone(),
            head,
            format!("target head for {target_ref}"),
            0.9,
        );
        t_idx += 1;
        push(
            "T",
            t_idx,
            target_ref.clone(),
            tail,
            format!("target tail for {target_ref}"),
            0.8,
        );
        t_idx += 1;
    }

    for (idx, rule) in contextpack
        .active_constraints
        .iter()
        .chain(contextpack.style_rules.iter())
        .enumerate()
    {
        push(
            "R",
            idx + 1,
            "contextpack:rules".to_string(),
            rule.clone(),
            "rule/constraint".to_string(),
            0.7,
        );
    }

    for (idx, fact) in contextpack.key_facts.iter().enumerate() {
        push(
            "F",
            idx + 1,
            "contextpack:key_facts".to_string(),
            fact.clone(),
            "key fact".to_string(),
            0.7,
        );
    }

    for (idx, note) in contextpack.cast_notes.iter().enumerate() {
        let mut snippet = note.summary.clone();
        if let Some(voice) = note.voice_signals.as_ref().filter(|v| !v.is_empty()) {
            snippet.push_str(&format!(" | voice: {}", voice.join(", ")));
        }
        push(
            "C",
            idx + 1,
            format!("character:{}", note.character_ref),
            snippet,
            "cast note".to_string(),
            0.65,
        );
    }

    for (idx, target) in contextpack.review_targets.iter().enumerate() {
        push(
            "O",
            idx + 1,
            "contextpack:review_targets".to_string(),
            target.clone(),
            "objective/success criteria".to_string(),
            0.7,
        );
    }

    for (idx, snippet) in contextpack.evidence_snippets.iter().enumerate() {
        push(
            "S",
            idx + 1,
            snippet.source_ref.clone(),
            snippet.snippet.clone(),
            snippet.reason.clone(),
            snippet.score,
        );
    }

    items.truncate(40);
    valid_ids = items.iter().map(|item| item.id.clone()).collect();

    Ok(EvidenceBundle { items, valid_ids })
}

fn load_target_text(project_path: &Path, target_ref: &str) -> Result<String, AppError> {
    let (_normalized, full) = resolve_review_target_path(project_path, target_ref)?;

    if full.extension().and_then(|v| v.to_str()) == Some("json") {
        let chapter = read_json::<crate::models::Chapter>(&full)?;
        return Ok(extract_tiptap_text(&chapter.content));
    }

    Ok(std::fs::read_to_string(full)?)
}

fn parse_llm_review_response(raw: &str) -> Result<RawLlmReviewResponse, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(review_evidence_unavailable(
            "review model returned empty response",
            Some("empty_model_response"),
        ));
    }

    let candidate = extract_json_object(trimmed).unwrap_or(trimmed);
    serde_json::from_str::<RawLlmReviewResponse>(candidate).map_err(|_| {
        review_evidence_unavailable("review model returned invalid JSON", Some("invalid_json"))
    })
}

fn normalize_llm_issues(
    raw_issues: Vec<RawLlmIssue>,
    valid_ids: &HashSet<String>,
    allowed_review_types: &[ReviewType],
) -> Vec<ReviewIssue> {
    raw_issues
        .into_iter()
        .take(20)
        .filter_map(|raw| {
            let review_type = parse_review_type(&raw.review_type)?;
            if !allowed_review_types.contains(&review_type) {
                return None;
            }

            let mut severity = parse_severity(&raw.severity)?;
            let mut confidence = parse_confidence(&raw.confidence)?;
            let summary = raw.summary.trim().to_string();
            if summary.is_empty() {
                return None;
            }

            let evidence_refs = raw
                .evidence_refs
                .into_iter()
                .map(|r| r.trim().to_string())
                .filter(|r| valid_ids.contains(r))
                .collect::<Vec<_>>();
            if evidence_refs.is_empty() {
                return None;
            }

            let has_target_ref = evidence_refs.iter().any(|r| r.starts_with('T'));
            let has_non_target_ref = evidence_refs.iter().any(|r| !r.starts_with('T'));
            if !has_target_ref {
                severity = ReviewSeverity::Warn;
                confidence = ReviewConfidence::Low;
            }
            if severity == ReviewSeverity::Block
                && requires_cross_evidence(&review_type)
                && !has_non_target_ref
            {
                severity = ReviewSeverity::Warn;
                confidence = ReviewConfidence::Low;
            }

            let mut auto_fixable = raw.auto_fixable.unwrap_or(true);
            if should_force_non_auto_fixable(&summary, &review_type, &evidence_refs) {
                auto_fixable = false;
            }

            Some(ReviewIssue {
                issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
                review_type,
                severity,
                summary,
                subject_refs: Vec::new(),
                evidence_refs,
                confidence,
                suggested_fix: raw
                    .suggested_fix
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                auto_fixable,
            })
        })
        .collect()
}

fn requires_cross_evidence(review_type: &ReviewType) -> bool {
    matches!(
        review_type,
        ReviewType::Continuity
            | ReviewType::Character
            | ReviewType::Foreshadow
            | ReviewType::Terminology
    )
}

fn should_force_non_auto_fixable(
    summary: &str,
    review_type: &ReviewType,
    evidence_refs: &[String],
) -> bool {
    let lower = summary.to_ascii_lowercase();
    matches!(review_type, ReviewType::Continuity | ReviewType::Character)
        && (lower.contains("canon")
            || lower.contains("ooc")
            || lower.contains("out of character")
            || lower.contains("设定冲突")
            || lower.contains("方向")
            || lower.contains("分支")
            || lower.contains("二选一")
            || evidence_refs.iter().any(|r| r.starts_with('R')))
}

fn parse_review_type(raw: &str) -> Option<ReviewType> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "continuity" => Some(ReviewType::Continuity),
        "logic" => Some(ReviewType::Logic),
        "character" => Some(ReviewType::Character),
        "style" => Some(ReviewType::Style),
        "terminology" => Some(ReviewType::Terminology),
        "foreshadow" => Some(ReviewType::Foreshadow),
        "objective_completion" => Some(ReviewType::ObjectiveCompletion),
        _ => None,
    }
}

fn parse_severity(raw: &str) -> Option<ReviewSeverity> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "warn" => Some(ReviewSeverity::Warn),
        "block" => Some(ReviewSeverity::Block),
        _ => None,
    }
}

fn parse_confidence(raw: &str) -> Option<ReviewConfidence> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => Some(ReviewConfidence::Low),
        "medium" => Some(ReviewConfidence::Medium),
        "high" => Some(ReviewConfidence::High),
        _ => None,
    }
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    (end > start).then_some(&raw[start..=end])
}

fn truncate_chars(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let mut out = input
        .chars()
        .take(max.saturating_sub(15))
        .collect::<String>();
    out.push_str("[...truncated]");
    out
}

fn head_chars(input: &str, max: usize) -> String {
    input.chars().take(max).collect()
}

fn tail_chars(input: &str, max: usize) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let len = chars.len();
    chars[len.saturating_sub(max)..].iter().collect()
}

fn review_type_as_str(review_type: &ReviewType) -> &'static str {
    match review_type {
        ReviewType::WordCount => "word_count",
        ReviewType::Continuity => "continuity",
        ReviewType::Logic => "logic",
        ReviewType::Character => "character",
        ReviewType::Style => "style",
        ReviewType::Terminology => "terminology",
        ReviewType::Foreshadow => "foreshadow",
        ReviewType::ObjectiveCompletion => "objective_completion",
    }
}

fn review_llm_error<E: Into<AppError>>(err: E) -> AppError {
    let err: AppError = err.into();
    review_evidence_unavailable(
        "semantic review LLM call failed",
        err.details
            .as_ref()
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str()),
    )
}

fn review_llm_parse_error<E: Into<AppError>>(_err: E) -> AppError {
    review_evidence_unavailable(
        "semantic review stream accumulation failed",
        Some("stream_parse_failed"),
    )
}

fn review_evidence_unavailable(message: &str, reason: Option<&str>) -> AppError {
    AppError {
        code: ErrorCode::Internal,
        message: message.to_string(),
        details: Some(serde_json::json!({
            "code": "REVIEW_EVIDENCE_UNAVAILABLE",
            "reason": reason,
        })),
        recoverable: Some(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_llm_review_response_accepts_fenced_json() {
        let raw = "```json\n{\"issues\":[{\"review_type\":\"logic\",\"severity\":\"warn\",\"confidence\":\"high\",\"summary\":\"x\",\"evidence_refs\":[\"T1\"]}]}\n```";
        let parsed = parse_llm_review_response(raw).unwrap();
        assert_eq!(parsed.issues.len(), 1);
    }

    #[test]
    fn normalize_llm_issues_downgrades_block_without_cross_evidence() {
        let valid = HashSet::from(["T1".to_string()]);
        let issues = normalize_llm_issues(
            vec![RawLlmIssue {
                review_type: "continuity".to_string(),
                severity: "block".to_string(),
                confidence: "high".to_string(),
                summary: "canon conflict".to_string(),
                evidence_refs: vec!["T1".to_string()],
                suggested_fix: None,
                auto_fixable: Some(true),
            }],
            &valid,
            &[ReviewType::Continuity],
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ReviewSeverity::Warn);
        assert_eq!(issues[0].confidence, ReviewConfidence::Low);
        assert!(!issues[0].auto_fixable);
    }

    #[test]
    fn normalize_llm_issues_drops_unknown_evidence_refs() {
        let valid = HashSet::from(["T1".to_string(), "F1".to_string()]);
        let issues = normalize_llm_issues(
            vec![RawLlmIssue {
                review_type: "logic".to_string(),
                severity: "warn".to_string(),
                confidence: "medium".to_string(),
                summary: "missing motivation".to_string(),
                evidence_refs: vec!["X1".to_string(), "T1".to_string(), "F1".to_string()],
                suggested_fix: Some("补充动机".to_string()),
                auto_fixable: Some(true),
            }],
            &valid,
            &[ReviewType::Logic],
        );

        assert_eq!(
            issues[0].evidence_refs,
            vec!["T1".to_string(), "F1".to_string()]
        );
    }
}
