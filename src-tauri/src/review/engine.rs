use std::path::{Path, PathBuf};

use crate::models::{AppError, Chapter};
use crate::services::word_count::count_text;

use super::types::*;

const MANUSCRIPTS_DIR: &str = "manuscripts";

pub fn run_review(project_path: &Path, mut input: ReviewRunInput) -> Result<ReviewReport, AppError> {
    input.scope_ref = input.scope_ref.trim().to_string();
    if input.scope_ref.is_empty() {
        return Err(AppError::invalid_argument("review scope_ref is missing"));
    }

    if input.target_refs.is_empty() {
        return Err(AppError {
            code: crate::models::ErrorCode::InvalidArgument,
            message: "review target_refs is empty".to_string(),
            details: Some(serde_json::json!({ "code": "REVIEW_INPUT_MISSING" })),
            recoverable: Some(true),
        });
    }

    if input.review_types.is_empty() {
        input.review_types = vec![ReviewType::WordCount];
    }

    let now = chrono::Utc::now().timestamp_millis();
    let mut issues: Vec<ReviewIssue> = Vec::new();
    let mut evidence_summary: Vec<String> = Vec::new();

    if input.review_types.contains(&ReviewType::WordCount) {
        for target_ref in &input.target_refs {
            let target = load_target(project_path, target_ref)?;

            let actual = target.word_count.max(0);
            let (target_words, min_words, max_words) = match target.target_words.filter(|v| *v > 0) {
                Some(t) => {
                    let min = ((t as f64) * 0.7).round() as i32;
                    let max = ((t as f64) * 1.3).round() as i32;
                    (Some(t), min.max(1), max.max(1))
                }
                None => (None, 0, 0),
            };

            if let Some(tw) = target_words {
                evidence_summary.push(format!(
                    "word_count: target_ref={} actual={} target={} min={} max={}",
                    target_ref.trim(),
                    actual,
                    tw,
                    min_words,
                    max_words
                ));

                if actual < min_words || actual > max_words {
                    issues.push(ReviewIssue {
                        issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
                        review_type: ReviewType::WordCount,
                        severity: ReviewSeverity::Block,
                        summary: format!(
                            "本章 {} 字，超出硬约束范围（{}~{}；目标 {}）",
                            actual, min_words, max_words, tw
                        ),
                        subject_refs: vec![target_ref.clone()],
                        evidence_refs: vec![
                            format!("target:{}#word_count={}", target_ref, actual),
                            format!(
                                "constraint:chapter_words[min={},max={},target={}]",
                                min_words, max_words, tw
                            ),
                        ],
                        confidence: ReviewConfidence::High,
                        suggested_fix: Some(
                            "补强场景冲突推进/信息增量，不要只加描述性填充；保持已通过部分不变".to_string(),
                        ),
                        auto_fixable: true,
                    });
                    continue;
                }

                let deviation = ((actual - tw).abs() as f64) / (tw.max(1) as f64);
                if deviation > 0.15 {
                    issues.push(ReviewIssue {
                        issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
                        review_type: ReviewType::WordCount,
                        severity: ReviewSeverity::Warn,
                        summary: format!(
                            "本章 {} 字，偏离目标字数 {} 超过 15%（允许范围 {}~{}）",
                            actual, tw, min_words, max_words
                        ),
                        subject_refs: vec![target_ref.clone()],
                        evidence_refs: vec![
                            format!("target:{}#word_count={}", target_ref, actual),
                            format!("constraint:chapter_words[target={}]", tw),
                        ],
                        confidence: ReviewConfidence::High,
                        suggested_fix: Some(
                            "微调节奏：删除重复描写或补充必要动作/对话，使字数靠近目标".to_string(),
                        ),
                        auto_fixable: true,
                    });
                }
            } else {
                evidence_summary.push(format!(
                    "word_count: target_ref={} actual={} (no target_words)",
                    target_ref.trim(),
                    actual
                ));
            }
        }
    }

    let overall_status = aggregate_overall_status(&issues);
    let recommended_action = recommend_action(overall_status.clone(), &issues);

    Ok(ReviewReport {
        schema_version: REVIEW_SCHEMA_VERSION,
        review_id: format!("rev_{}", uuid::Uuid::new_v4()),
        scope_ref: input.scope_ref,
        target_refs: input.target_refs,
        review_types: input.review_types,
        overall_status,
        issues,
        evidence_summary,
        recommended_action,
        generated_at: now,
    })
}

fn aggregate_overall_status(issues: &[ReviewIssue]) -> ReviewOverallStatus {
    if issues.iter().any(|i| i.severity == ReviewSeverity::Block) {
        return ReviewOverallStatus::Block;
    }
    if issues.iter().any(|i| i.severity == ReviewSeverity::Warn) {
        return ReviewOverallStatus::Warn;
    }
    ReviewOverallStatus::Pass
}

fn recommend_action(status: ReviewOverallStatus, issues: &[ReviewIssue]) -> ReviewRecommendedAction {
    match status {
        ReviewOverallStatus::Pass => ReviewRecommendedAction::Accept,
        ReviewOverallStatus::Warn => {
            if issues.iter().any(|i| i.severity == ReviewSeverity::Warn && !i.auto_fixable) {
                ReviewRecommendedAction::Escalate
            } else {
                ReviewRecommendedAction::Revise
            }
        }
        ReviewOverallStatus::Block => {
            if issues.iter().any(|i| i.severity == ReviewSeverity::Block && !i.auto_fixable) {
                ReviewRecommendedAction::Escalate
            } else {
                ReviewRecommendedAction::Revise
            }
        }
    }
}

struct ReviewTarget {
    word_count: i32,
    target_words: Option<i32>,
}

fn load_target(project_path: &Path, target_ref: &str) -> Result<ReviewTarget, AppError> {
    let normalized = target_ref.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err(AppError::invalid_argument("empty target_ref"));
    }
    if normalized.starts_with('/') || normalized.split('/').any(|seg| seg == "..") {
        return Err(AppError::invalid_argument(format!(
            "invalid target_ref: {normalized}"
        )));
    }

    let candidates: [PathBuf; 2] = [
        PathBuf::from(project_path)
            .join(MANUSCRIPTS_DIR)
            .join(&normalized),
        PathBuf::from(project_path).join(&normalized),
    ];

    let full = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| {
            AppError {
                code: crate::models::ErrorCode::NotFound,
                message: format!("review target not found: {normalized}"),
                details: Some(serde_json::json!({
                    "code": "REVIEW_INPUT_MISSING",
                    "target_ref": normalized,
                })),
                recoverable: Some(true),
            }
        })?;

    if full.extension().and_then(|s| s.to_str()) == Some("json") {
        let raw = std::fs::read_to_string(&full)?;
        let chapter: Chapter = serde_json::from_str(&raw)?;
        let counts = count_text(&chapter.content);
        Ok(ReviewTarget {
            word_count: counts.word_count.unwrap_or(0),
            target_words: chapter.target_words,
        })
    } else {
        let raw = std::fs::read_to_string(&full)?;
        Ok(ReviewTarget {
            word_count: count_words_like_app(&raw),
            target_words: None,
        })
    }
}

fn count_words_like_app(text: &str) -> i32 {
    let mut count = 0;
    let mut in_word = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if in_word {
                count += 1;
                in_word = false;
            }
        } else if is_cjk(c) {
            if in_word {
                count += 1;
                in_word = false;
            }
            count += 1;
        } else {
            in_word = true;
        }
    }

    if in_word {
        count += 1;
    }

    count
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{2B820}'..='\u{2CEAF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}' |
        '\u{3000}'..='\u{303F}' |
        '\u{3040}'..='\u{309F}' |
        '\u{30A0}'..='\u{30FF}' |
        '\u{FF00}'..='\u{FFEF}'
    )
}
