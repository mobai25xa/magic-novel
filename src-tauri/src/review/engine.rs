use std::path::{Path, PathBuf};

use crate::mission::contextpack_types::ContextPack;
use crate::writing_rules::types::{
    ChapterWordsConstraint as WritingRuleChapterWordsConstraint, EffectiveRules,
};

use crate::models::{AppError, Chapter};
use crate::services::word_count::count_text;

use super::llm_multi_gate::{self, ReviewLlmConfig};
use super::types::*;

const MANUSCRIPTS_DIR: &str = "manuscripts";

pub struct ReviewRuntimeOptions<'a> {
    pub contextpack: Option<&'a ContextPack>,
    pub llm_config: Option<&'a ReviewLlmConfig>,
    /// Optional override for the resolved effective rules (e.g. chapter-card bound versions).
    pub effective_rules_override: Option<EffectiveRules>,
}

impl<'a> Default for ReviewRuntimeOptions<'a> {
    fn default() -> Self {
        Self {
            contextpack: None,
            llm_config: None,
            effective_rules_override: None,
        }
    }
}

fn run_review_with_effective_rules(
    project_path: &Path,
    mut input: ReviewRunInput,
    effective_rules: EffectiveRules,
) -> Result<ReviewReport, AppError> {
    input.scope_ref = input.scope_ref.trim().to_string();
    if input.scope_ref.is_empty() {
        return Err(AppError::invalid_argument("review scope_ref is missing"));
    }

    input.target_refs = normalize_target_refs(&input.target_refs);
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
    } else {
        input.review_types = normalize_review_types(&input.review_types);
    }

    let now = chrono::Utc::now().timestamp_millis();

    let raw_threshold = input
        .severity_threshold
        .as_deref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let severity_threshold = parse_severity_threshold(raw_threshold);

    let ctx = ReviewContext {
        project_path,
        input: &input,
        effective_rules: &effective_rules,
        now,
        severity_threshold,
    };

    let mut issues: Vec<ReviewIssue> = Vec::new();
    let mut evidence_summary: Vec<String> = Vec::new();

    let mut implemented_review_types: Vec<ReviewType> = Vec::new();

    evidence_summary.push(format!(
        "rules_fingerprint={}",
        effective_rules.rules_fingerprint
    ));

    if let Some(cw) = effective_rules.chapter_words.as_ref() {
        evidence_summary.push(format!(
            "chapter_words: min={} max={} target={} source={}",
            cw.min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            cw.max
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            cw.target
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            "rulesets"
        ));
    } else {
        evidence_summary.push("chapter_words: (none)".to_string());
    }

    match (raw_threshold, severity_threshold) {
        (Some(_), Some(th)) => {
            evidence_summary.push(format!("severity_threshold={}", th.as_str()));
        }
        (Some(raw), None) => {
            evidence_summary.push(format!("severity_threshold=invalid:{}", raw));
        }
        (None, _) => {
            evidence_summary.push("severity_threshold=(none)".to_string());
        }
    }

    if input.review_types.contains(&ReviewType::WordCount) {
        let out = WordCountCheck.run(&ctx)?;
        issues.extend(out.issues);
        evidence_summary.extend(out.evidence_summary);
        implemented_review_types.push(ReviewType::WordCount);
    }

    let skipped = input
        .review_types
        .iter()
        .filter(|t| !implemented_review_types.contains(t))
        .map(review_type_as_str)
        .collect::<Vec<_>>();
    if !skipped.is_empty() {
        evidence_summary.push(format!("skipped_review_types={}", skipped.join(",")));
    }

    apply_severity_threshold(&mut issues, severity_threshold);

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

pub fn run_review(
    project_path: &Path,
    mut input: ReviewRunInput,
) -> Result<ReviewReport, AppError> {
    input.scope_ref = input.scope_ref.trim().to_string();
    let effective_rules = resolve_review_effective_rules(project_path, &input.scope_ref);
    run_review_with_effective_rules(project_path, input, effective_rules)
}

fn resolve_review_effective_rules(project_path: &Path, scope_ref: &str) -> EffectiveRules {
    crate::writing_rules::resolver::resolve_effective_rules(project_path, scope_ref)
}

pub async fn run_review_with_runtime(
    project_path: &Path,
    input: ReviewRunInput,
    options: ReviewRuntimeOptions<'_>,
) -> Result<ReviewReport, AppError> {
    let threshold = parse_severity_threshold(
        input
            .severity_threshold
            .as_deref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty()),
    );

    let mut input = input;
    input.scope_ref = input.scope_ref.trim().to_string();
    let effective_rules = options
        .effective_rules_override
        .unwrap_or_else(|| resolve_review_effective_rules(project_path, &input.scope_ref));
    let mut report = run_review_with_effective_rules(project_path, input.clone(), effective_rules)?;
    let semantic_review_types = input
        .review_types
        .iter()
        .filter(|t| is_semantic_review_type(t))
        .cloned()
        .collect::<Vec<_>>();

    if semantic_review_types.is_empty() {
        return Ok(report);
    }

    if !llm_multi_gate::review_llm_enabled() {
        return Ok(report);
    }

    let contextpack = options.contextpack.ok_or_else(|| AppError {
        code: crate::models::ErrorCode::Internal,
        message: "semantic review requires contextpack".to_string(),
        details: Some(serde_json::json!({
            "code": "REVIEW_EVIDENCE_UNAVAILABLE",
            "reason": "missing_contextpack",
        })),
        recoverable: Some(true),
    })?;
    let llm_config = options.llm_config.ok_or_else(|| AppError {
        code: crate::models::ErrorCode::Internal,
        message: "semantic review requires llm configuration".to_string(),
        details: Some(serde_json::json!({
            "code": "REVIEW_EVIDENCE_UNAVAILABLE",
            "reason": "missing_llm_config",
        })),
        recoverable: Some(true),
    })?;

    let semantic_output = llm_multi_gate::run_semantic_review(
        project_path,
        &report.target_refs,
        &semantic_review_types,
        contextpack,
        llm_config,
    )
    .await?;

    report.issues.extend(semantic_output.issues);
    report
        .evidence_summary
        .extend(semantic_output.evidence_summary);
    update_skipped_review_types_summary(&mut report.evidence_summary, &report.review_types, &{
        let mut implemented = vec![ReviewType::WordCount];
        for review_type in semantic_output.implemented_review_types {
            if !implemented.contains(&review_type) {
                implemented.push(review_type);
            }
        }
        implemented
    });

    apply_severity_threshold(&mut report.issues, threshold);
    report.overall_status = aggregate_overall_status(&report.issues);
    report.recommended_action = recommend_action(report.overall_status.clone(), &report.issues);

    Ok(report)
}

struct ReviewContext<'a> {
    project_path: &'a Path,
    input: &'a ReviewRunInput,
    effective_rules: &'a EffectiveRules,
    #[allow(dead_code)]
    now: i64,
    #[allow(dead_code)]
    severity_threshold: Option<SeverityThreshold>,
}

struct CheckOutput {
    issues: Vec<ReviewIssue>,
    evidence_summary: Vec<String>,
}

trait ReviewCheck {
    fn run(&self, ctx: &ReviewContext) -> Result<CheckOutput, AppError>;
}

struct WordCountCheck;

impl ReviewCheck for WordCountCheck {
    fn run(&self, ctx: &ReviewContext) -> Result<CheckOutput, AppError> {
        let mut issues = Vec::new();
        let mut evidence_summary = Vec::new();

        for target_ref in &ctx.input.target_refs {
            let target = load_target(ctx.project_path, target_ref)?;
            let actual = target.word_count.max(0);

            let resolved = resolve_word_count_constraint(
                ctx.effective_rules.chapter_words.as_ref(),
                target.target_words,
            );

            match resolved.as_ref() {
                Some(c) => {
                    let source = match c.source {
                        WordCountConstraintSource::EffectiveRules => format!(
                            "effective_rules({})",
                            c.rules_source.as_ref().copied().unwrap_or("unknown")
                        ),
                        WordCountConstraintSource::ChapterMeta => "chapter_meta".to_string(),
                    };

                    evidence_summary.push(format!(
                        "word_count: target_ref={} actual={} min={} max={} target={} source={} rules_fp={}"
                        ,
                        target_ref,
                        actual,
                        c.min,
                        c.max,
                        c.target
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        source,
                        ctx.effective_rules.rules_fingerprint
                    ));

                    if actual < c.min || actual > c.max {
                        issues.push(build_word_count_issue_block(target_ref, actual, c));
                        continue;
                    }

                    if let Some(tw) = c.target {
                        let deviation = ((actual - tw).abs() as f64) / (tw.max(1) as f64);
                        if deviation > 0.15 {
                            issues.push(build_word_count_issue_warn(target_ref, actual, tw, c));
                        }
                    }
                }
                None => {
                    evidence_summary.push(format!(
                        "word_count: target_ref={} actual={} constraint_source=none rules_fp={}",
                        target_ref, actual, ctx.effective_rules.rules_fingerprint
                    ));
                }
            }
        }

        Ok(CheckOutput {
            issues,
            evidence_summary,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeverityThreshold {
    Warn,
    Block,
}

impl SeverityThreshold {
    fn as_str(&self) -> &'static str {
        match self {
            SeverityThreshold::Warn => "warn",
            SeverityThreshold::Block => "block",
        }
    }
}

fn parse_severity_threshold(raw: Option<&str>) -> Option<SeverityThreshold> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }

    match raw.to_ascii_lowercase().as_str() {
        "block" => Some(SeverityThreshold::Block),
        "warn" => Some(SeverityThreshold::Warn),
        _ => None,
    }
}

fn apply_severity_threshold(issues: &mut Vec<ReviewIssue>, threshold: Option<SeverityThreshold>) {
    match threshold {
        Some(SeverityThreshold::Block) => {
            issues.retain(|i| i.severity == ReviewSeverity::Block);
        }
        Some(SeverityThreshold::Warn) => {
            issues.retain(|i| i.severity != ReviewSeverity::Info);
        }
        None => {}
    }
}

fn normalize_target_refs(target_refs: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for v in target_refs {
        let norm = v.trim().replace('\\', "/");
        if norm.is_empty() {
            continue;
        }
        if !out.contains(&norm) {
            out.push(norm);
        }
    }
    out
}

fn normalize_review_types(review_types: &[ReviewType]) -> Vec<ReviewType> {
    let mut out = Vec::new();
    for t in review_types {
        if !out.contains(t) {
            out.push(t.clone());
        }
    }
    out
}

fn is_semantic_review_type(review_type: &ReviewType) -> bool {
    !matches!(review_type, ReviewType::WordCount)
}

fn update_skipped_review_types_summary(
    evidence_summary: &mut Vec<String>,
    requested_review_types: &[ReviewType],
    implemented_review_types: &[ReviewType],
) {
    evidence_summary.retain(|line| !line.starts_with("skipped_review_types="));

    let skipped = requested_review_types
        .iter()
        .filter(|t| !implemented_review_types.contains(t))
        .map(review_type_as_str)
        .collect::<Vec<_>>();
    if !skipped.is_empty() {
        evidence_summary.push(format!("skipped_review_types={}", skipped.join(",")));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordCountConstraintSource {
    EffectiveRules,
    ChapterMeta,
}

#[derive(Debug, Clone)]
struct ResolvedWordCountConstraint {
    min: i32,
    max: i32,
    target: Option<i32>,
    source: WordCountConstraintSource,
    rules_source: Option<&'static str>,
}

fn resolve_word_count_constraint(
    effective_rules: Option<&WritingRuleChapterWordsConstraint>,
    chapter_target_words: Option<i32>,
) -> Option<ResolvedWordCountConstraint> {
    if let Some(cw) = effective_rules {
        let min = cw.min?;
        let max = cw.max?;
        if min > 0 && max > 0 && min <= max {
            return Some(ResolvedWordCountConstraint {
                min,
                max,
                target: cw.target.filter(|v| *v > 0),
                source: WordCountConstraintSource::EffectiveRules,
                rules_source: Some("rulesets"),
            });
        }
    }

    let tw = chapter_target_words.filter(|v| *v > 0)?;
    let min = ((tw as f64) * 0.7).round() as i32;
    let max = ((tw as f64) * 1.3).round() as i32;

    Some(ResolvedWordCountConstraint {
        min: min.max(1),
        max: max.max(1),
        target: Some(tw),
        source: WordCountConstraintSource::ChapterMeta,
        rules_source: None,
    })
}

fn build_word_count_issue_block(
    target_ref: &str,
    actual: i32,
    c: &ResolvedWordCountConstraint,
) -> ReviewIssue {
    let source = match c.source {
        WordCountConstraintSource::EffectiveRules => "effective_rules",
        WordCountConstraintSource::ChapterMeta => "chapter_meta",
    };

    let summary = match c.target {
        Some(tw) => format!(
            "本章 {} 字，超出硬约束范围（{}~{}；目标 {}）",
            actual, c.min, c.max, tw
        ),
        None => format!("本章 {} 字，超出硬约束范围（{}~{}）", actual, c.min, c.max),
    };

    let mut evidence_refs = vec![
        format!("target:{}#word_count={}", target_ref, actual),
        format!(
            "constraint:chapter_words[min={},max={},target={}]{{source={}}}",
            c.min,
            c.max,
            c.target
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            source
        ),
    ];

    if c.source == WordCountConstraintSource::EffectiveRules {
        if let Some(rs) = c.rules_source.as_ref() {
            evidence_refs.push(format!("rule_source:{rs}"));
        }
    }

    ReviewIssue {
        issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
        review_type: ReviewType::WordCount,
        severity: ReviewSeverity::Block,
        summary,
        subject_refs: vec![target_ref.to_string()],
        evidence_refs,
        confidence: ReviewConfidence::High,
        suggested_fix: Some(
            "补强场景冲突推进/信息增量，不要只加描述性填充；保持已通过部分不变".to_string(),
        ),
        auto_fixable: true,
    }
}

fn build_word_count_issue_warn(
    target_ref: &str,
    actual: i32,
    target_words: i32,
    c: &ResolvedWordCountConstraint,
) -> ReviewIssue {
    let source = match c.source {
        WordCountConstraintSource::EffectiveRules => "effective_rules",
        WordCountConstraintSource::ChapterMeta => "chapter_meta",
    };

    let mut evidence_refs = vec![
        format!("target:{}#word_count={}", target_ref, actual),
        format!(
            "constraint:chapter_words[min={},max={},target={}]{{source={}}}",
            c.min, c.max, target_words, source
        ),
    ];

    if c.source == WordCountConstraintSource::EffectiveRules {
        if let Some(rs) = c.rules_source.as_ref() {
            evidence_refs.push(format!("rule_source:{rs}"));
        }
    }

    ReviewIssue {
        issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
        review_type: ReviewType::WordCount,
        severity: ReviewSeverity::Warn,
        summary: format!(
            "本章 {} 字，偏离目标字数 {} 超过 15%（允许范围 {}~{}）",
            actual, target_words, c.min, c.max
        ),
        subject_refs: vec![target_ref.to_string()],
        evidence_refs,
        confidence: ReviewConfidence::High,
        suggested_fix: Some(
            "微调节奏：删除重复描写或补充必要动作/对话，使字数靠近目标".to_string(),
        ),
        auto_fixable: true,
    }
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

fn recommend_action(
    status: ReviewOverallStatus,
    issues: &[ReviewIssue],
) -> ReviewRecommendedAction {
    match status {
        ReviewOverallStatus::Pass => ReviewRecommendedAction::Accept,
        ReviewOverallStatus::Warn => {
            if issues
                .iter()
                .any(|i| i.severity == ReviewSeverity::Warn && !i.auto_fixable)
            {
                ReviewRecommendedAction::Escalate
            } else {
                ReviewRecommendedAction::Revise
            }
        }
        ReviewOverallStatus::Block => {
            if issues
                .iter()
                .any(|i| i.severity == ReviewSeverity::Block && !i.auto_fixable)
            {
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
        .ok_or_else(|| AppError {
            code: crate::models::ErrorCode::NotFound,
            message: format!("review target not found: {normalized}"),
            details: Some(serde_json::json!({
                "code": "REVIEW_INPUT_MISSING",
                "target_ref": normalized,
            })),
            recoverable: Some(true),
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

fn review_type_as_str(t: &ReviewType) -> &'static str {
    match t {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::atomic_write::atomic_write_json;
    use std::fs;
    use std::path::PathBuf;

    fn temp_project_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_words(n: usize) -> String {
        (0..n).map(|_| "w").collect::<Vec<_>>().join(" ")
    }

    fn write_global_word_count_ruleset(project: &Path, min: i32, max: i32, target: Option<i32>) {
        let dir = project.join(".magic_novel").join("rules").join("rulesets");
        fs::create_dir_all(&dir).unwrap();
        let target_line = target
            .map(|value| format!("    target: {value}\n"))
            .unwrap_or_default();
        let yaml = format!(
            "schema_version: 1\nruleset_id: wc\nversion: 1\nstatus: accepted\nscope: global\nscope_ref: project\nconstraints:\n  chapter_words:\n    min: {min}\n    max: {max}\n{target_line}"
        );
        fs::write(dir.join("global.v0001.yaml"), yaml).unwrap();
    }

    fn write_chapter(project: &Path, rel: &str, words: usize, target_words: Option<i32>) {
        let full = project.join("manuscripts").join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();

        let mut ch = Chapter::new("T".to_string());
        ch.content = serde_json::Value::String(make_words(words));
        ch.target_words = target_words;
        atomic_write_json(&full, &ch).unwrap();
    }

    #[test]
    fn ruleset_word_count_blocks_outside_range() {
        let project = temp_project_dir();
        write_global_word_count_ruleset(&project, 10, 20, Some(15));
        write_chapter(&project, "vol1/ch1.json", 9, None);

        let input = ReviewRunInput {
            scope_ref: "chapter:vol1/ch1.json".to_string(),
            target_refs: vec!["vol1/ch1.json".to_string()],
            branch_id: None,
            review_types: vec![ReviewType::WordCount],
            task_card_ref: None,
            context_pack_ref: None,
            effective_rules_fingerprint: None,
            severity_threshold: None,
        };

        let report = run_review(&project, input).unwrap();
        assert_eq!(report.overall_status, ReviewOverallStatus::Block);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].severity, ReviewSeverity::Block);
        assert!(report
            .evidence_summary
            .iter()
            .any(|s| s.starts_with("rules_fingerprint=")));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn writing_rules_word_count_blocks_outside_range() {
        let project = temp_project_dir();
        write_global_word_count_ruleset(&project, 10, 20, Some(15));

        write_chapter(&project, "vol1/ch1.json", 9, None);

        let input = ReviewRunInput {
            scope_ref: "chapter:vol1/ch1.json".to_string(),
            target_refs: vec!["vol1/ch1.json".to_string()],
            branch_id: None,
            review_types: vec![ReviewType::WordCount],
            task_card_ref: None,
            context_pack_ref: None,
            effective_rules_fingerprint: None,
            severity_threshold: None,
        };

        let report = run_review(&project, input).unwrap();
        assert_eq!(report.overall_status, ReviewOverallStatus::Block);
        assert!(report
            .evidence_summary
            .iter()
            .any(|s| s.contains("source=rulesets")));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn ruleset_word_count_warns_on_deviation() {
        let project = temp_project_dir();
        write_global_word_count_ruleset(&project, 10, 20, Some(15));
        write_chapter(&project, "vol1/ch1.json", 19, None);

        let input = ReviewRunInput {
            scope_ref: "chapter:vol1/ch1.json".to_string(),
            target_refs: vec!["vol1/ch1.json".to_string()],
            branch_id: None,
            review_types: vec![ReviewType::WordCount],
            task_card_ref: None,
            context_pack_ref: None,
            effective_rules_fingerprint: None,
            severity_threshold: None,
        };

        let report = run_review(&project, input).unwrap();
        assert_eq!(report.overall_status, ReviewOverallStatus::Warn);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].severity, ReviewSeverity::Warn);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn severity_threshold_block_filters_warns() {
        let project = temp_project_dir();
        write_global_word_count_ruleset(&project, 10, 20, Some(15));
        write_chapter(&project, "vol1/ch1.json", 19, None);

        let input = ReviewRunInput {
            scope_ref: "chapter:vol1/ch1.json".to_string(),
            target_refs: vec!["vol1/ch1.json".to_string()],
            branch_id: None,
            review_types: vec![ReviewType::WordCount],
            task_card_ref: None,
            context_pack_ref: None,
            effective_rules_fingerprint: None,
            severity_threshold: Some("block".to_string()),
        };

        let report = run_review(&project, input).unwrap();
        assert_eq!(report.overall_status, ReviewOverallStatus::Pass);
        assert_eq!(report.issues.len(), 0);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn no_constraints_yields_none() {
        let none = resolve_word_count_constraint(None, None);
        assert!(none.is_none());
    }
}
