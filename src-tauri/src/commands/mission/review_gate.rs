//! ReviewGate helpers and fixup tracking for missions.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;

use crate::mission::agent_profile::SessionSource;
use crate::mission::artifacts;
use crate::mission::contextpack_builder::{self, BuildContextPackInput};
use crate::mission::contextpack_staleness::{
    check_contextpack_staleness, ContextPackStalenessStatus,
};
use crate::mission::contextpack_types::{ContextPack, TokenBudget};
use crate::mission::events::MissionEventEmitter;
use crate::mission::layer1_types::{ChapterWorkflowKind, LAYER1_SCHEMA_VERSION};
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::mission::worker_profile::builtin_general_worker_profile;
use crate::models::AppError;
use crate::review::{
    engine as review_engine, llm_multi_gate::ReviewLlmConfig, types as review_types,
};

use super::runtime::*;
use crate::commands::mission::scheduler::{
    active_in_process_delegate_worker_ids, cancel_in_process_delegates,
    spawn_and_initialize_worker, spawn_in_process_delegate_supervision_task,
    spawn_process_delegate_supervision_task, start_feature_in_process, start_feature_on_worker,
};

use super::{DelegateTransportMode, MissionStartConfig, REVIEW_FIXUP_MAX_ATTEMPTS};

mod gate_select;

// ── M3: Review Gate fixup tracking ─────────────────────────────

#[derive(Debug, Clone)]
pub(super) struct ReviewFixupTracker {
    key: String,
    attempts: u32,
}

pub(super) type ReviewFixupRegistry = DashMap<String, ReviewFixupTracker>;

pub(super) fn review_fixup_registry() -> &'static ReviewFixupRegistry {
    static REGISTRY: std::sync::OnceLock<ReviewFixupRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(DashMap::new)
}

fn normalize_review_target_ref(raw: &str) -> Option<String> {
    let mut p = raw.trim().replace('\\', "/");
    if p.is_empty() {
        return None;
    }
    while p.starts_with("./") {
        p = p[2..].to_string();
    }
    while p.contains("//") {
        p = p.replace("//", "/");
    }
    Some(p)
}

fn review_fixup_key_for_targets(target_refs: &[String]) -> String {
    let mut refs = target_refs
        .iter()
        .filter_map(|r| normalize_review_target_ref(r))
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    refs.join("|")
}

// ── M3: Review Gate helpers ───────────────────────────────────

pub(super) fn infer_review_scope_ref(project_path: &std::path::Path, mission_id: &str) -> String {
    fn normalize_chapter_locator(raw: &str) -> Option<String> {
        let mut norm = raw.trim().replace('\\', "/");
        if norm.is_empty() {
            return None;
        }
        if let Some(stripped) = norm.strip_prefix("manuscripts/") {
            norm = stripped.to_string();
        }
        while norm.starts_with("./") {
            norm = norm[2..].to_string();
        }
        while norm.contains("//") {
            norm = norm.replace("//", "/");
        }
        let norm = norm.trim();
        if norm.is_empty() {
            return None;
        }
        if norm.starts_with('/') || norm.contains(':') {
            return None;
        }
        if norm.split('/').any(|seg| seg == "..") {
            return None;
        }
        if !norm.to_ascii_lowercase().ends_with(".json") {
            return None;
        }
        Some(norm.to_string())
    }

    if let Ok(Some(cc)) = artifacts::read_layer1_chapter_card(project_path, mission_id) {
        if let Some(locator) = cc
            .scope_locator
            .as_deref()
            .and_then(normalize_chapter_locator)
        {
            return format!("chapter:{locator}");
        }

        let scope_ref = cc.scope_ref.trim();
        if !scope_ref.is_empty() {
            return scope_ref.to_string();
        }
    }

    format!("mission:{mission_id}")
}

pub(super) fn persist_review_report(
    project_path: &std::path::Path,
    mission_id: &str,
    report: &review_types::ReviewReport,
) -> Result<(), AppError> {
    artifacts::write_review_latest(project_path, mission_id, report)?;
    let _ = artifacts::append_review_report(project_path, mission_id, report);
    Ok(())
}

pub(super) fn filter_chapter_write_targets(
    project_path: &std::path::Path,
    write_paths: &[String],
) -> Vec<String> {
    let mut out = Vec::new();

    for raw in write_paths {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        let mut norm = raw.replace('\\', "/");
        if let Some(stripped) = norm.strip_prefix("manuscripts/") {
            norm = stripped.to_string();
        }

        let norm = norm.trim();
        if norm.is_empty() {
            continue;
        }
        if norm.starts_with('/') || norm.contains(':') {
            continue;
        }
        if norm.split('/').any(|seg| seg == "..") {
            continue;
        }
        if !norm.to_ascii_lowercase().ends_with(".json") {
            continue;
        }

        let full = project_path.join("manuscripts").join(norm);
        if !full.exists() {
            continue;
        }

        // Only treat manuscript JSONs that parse as Chapter as chapter-level targets.
        if crate::services::read_json::<crate::models::Chapter>(&full).is_ok() {
            let canonical = norm.to_string();
            if !out.contains(&canonical) {
                out.push(canonical);
            }
        }
    }

    out
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewGatePolicy {
    pub(crate) review_types: Vec<review_types::ReviewType>,
    pub(crate) severity_threshold: Option<String>,
    pub(crate) strict_warn: bool,
    pub(crate) auto_fix_on_block: bool,
    pub(crate) effective_rules_fingerprint: Option<String>,
}

/// DevC: Resolve the effective gate policy for the given scope.
///
/// Priority:
/// - If a `ValidationProfile` is configured via `EffectiveRules.validation_profile_id`, use it.
/// - Otherwise fall back to the legacy macro gate policy knobs and default review types.
pub(crate) fn resolve_chapter_gate_policy(
    project_path: &std::path::Path,
    mission_id: &str,
    scope_ref: &str,
    fallback_strict_warn: bool,
    fallback_auto_fix_on_block: bool,
) -> ReviewGatePolicy {
    let accepted_rulesets = crate::writing_rules::loader::load_accepted_rulesets(project_path);
    let structured_rulesets_active = !accepted_rulesets.is_empty();

    let (rules, effective_rules_fingerprint) = if structured_rulesets_active {
        let now = chrono::Utc::now().timestamp_millis();
        let card = artifacts::read_layer1_chapter_card(project_path, mission_id)
            .ok()
            .flatten();
        let has_binding = card.as_ref().is_some_and(|cc| !cc.rules_sources.is_empty());

        if has_binding {
            let bound_sources = card
                .as_ref()
                .map(|cc| cc.rules_sources.as_slice())
                .unwrap_or(&[]);

            let bound_rules = crate::writing_rules::chapter_binding::resolve_from_binding(
                project_path,
                scope_ref,
                bound_sources,
            );

            let rules = match bound_rules {
                Some(r) => r,
                None => {
                    // Missing bound versions: fall back to current effective_at_chapter selector
                    // but do not overwrite the binding evidence.
                    let (effective, _binding) =
                        crate::writing_rules::chapter_binding::resolve_and_bind(
                            project_path,
                            scope_ref,
                        );
                    effective
                }
            };

            (rules.clone(), Some(rules.rules_fingerprint.clone()))
        } else {
            // No binding yet: resolve using effective_from_chapter-aware selector and persist binding.
            let (effective, binding) =
                crate::writing_rules::chapter_binding::resolve_and_bind(project_path, scope_ref);

            if let Some(mut cc) = card {
                let fp_missing = cc
                    .rules_fingerprint
                    .as_deref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true);
                let sources_missing = cc.rules_sources.is_empty();
                if (fp_missing || sources_missing) && !binding.rules_sources.is_empty() {
                    cc.schema_version = LAYER1_SCHEMA_VERSION;
                    cc.rules_fingerprint = Some(binding.rules_fingerprint.clone());
                    cc.rules_sources = binding.rules_sources.clone();
                    cc.bound_validation_profile_id = binding.validation_profile_id.clone();
                    cc.bound_style_template_id = binding.style_template_id.clone();
                    cc.updated_at = now;

                    if let Err(e) =
                        artifacts::write_layer1_chapter_card(project_path, mission_id, &cc)
                    {
                        tracing::warn!(
                            target: "mission",
                            mission_id = %mission_id,
                            error = %e,
                            "failed to persist chapter_card rule binding"
                        );
                    }
                }
            }

            (effective.clone(), Some(binding.rules_fingerprint))
        }
    } else {
        let rules =
            crate::writing_rules::resolver::resolve_from_rulesets(&accepted_rulesets, scope_ref);
        let fp = Some(
            crate::mission::contextpack_staleness::compute_rules_fingerprint(project_path)
                .to_string(),
        );
        (rules, fp)
    };

    let profile_id = rules
        .validation_profile_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let profile = profile_id
        .and_then(|pid| crate::writing_rules::loader::load_validation_profile(project_path, pid));

    if let Some(profile) = profile.as_ref() {
        let assembled = crate::gate_integration::profile_assembler::assemble_review_input(
            scope_ref,
            Vec::new(),
            profile,
            effective_rules_fingerprint.clone(),
        );

        return ReviewGatePolicy {
            review_types: assembled.review_types,
            severity_threshold: assembled.severity_threshold,
            strict_warn: profile.strict_warn,
            auto_fix_on_block: profile.auto_fix_on_block,
            effective_rules_fingerprint,
        };
    }

    ReviewGatePolicy {
        review_types: default_chapter_review_types(project_path, mission_id),
        severity_threshold: None,
        strict_warn: fallback_strict_warn,
        auto_fix_on_block: fallback_auto_fix_on_block,
        effective_rules_fingerprint,
    }
}

pub(super) fn default_chapter_review_types(
    project_path: &std::path::Path,
    mission_id: &str,
) -> Vec<review_types::ReviewType> {
    let mut types = vec![
        review_types::ReviewType::WordCount,
        review_types::ReviewType::Continuity,
        review_types::ReviewType::Logic,
        review_types::ReviewType::Character,
        review_types::ReviewType::Style,
        review_types::ReviewType::ObjectiveCompletion,
    ];

    if gate_select::should_enable_terminology_gate(project_path, mission_id) {
        types.push(review_types::ReviewType::Terminology);
    }
    if gate_select::should_enable_foreshadow_gate(project_path, mission_id) {
        types.push(review_types::ReviewType::Foreshadow);
    }

    types
}

#[derive(Debug, Clone)]
pub(super) struct ReviewGateRunMeta {
    pub(super) contextpack: Option<ContextPack>,
    pub(super) staleness: ContextPackStalenessStatus,
    pub(super) rebuilt: bool,
}

fn ensure_contextpack_for_review(
    project_path: &std::path::Path,
    mission_id: &str,
    scope_ref: &str,
) -> Result<(Option<ContextPack>, ContextPackStalenessStatus, bool), AppError> {
    let existing = artifacts::read_latest_contextpack(project_path, mission_id)?;
    let staleness = check_contextpack_staleness(project_path, mission_id, existing.as_ref())?;

    if !staleness.present || staleness.stale {
        let cp = contextpack_builder::build_and_persist_contextpack(
            project_path,
            mission_id,
            BuildContextPackInput {
                scope_ref: Some(scope_ref.to_string()),
                token_budget: None,
                active_chapter_path: None,
                selected_text: None,
            },
        )?;
        return Ok((Some(cp), staleness, true));
    }

    Ok((existing, staleness, false))
}

fn ensure_contextpack_for_review_with_budget(
    project_path: &std::path::Path,
    mission_id: &str,
    scope_ref: &str,
    budget: TokenBudget,
) -> Result<(Option<ContextPack>, ContextPackStalenessStatus, bool), AppError> {
    let existing = artifacts::read_latest_contextpack(project_path, mission_id)?;
    let staleness = check_contextpack_staleness(project_path, mission_id, existing.as_ref())?;

    if !staleness.present || staleness.stale {
        let cp = contextpack_builder::build_and_persist_contextpack(
            project_path,
            mission_id,
            BuildContextPackInput {
                scope_ref: Some(scope_ref.to_string()),
                token_budget: Some(budget),
                active_chapter_path: None,
                selected_text: None,
            },
        )?;
        return Ok((Some(cp), staleness, true));
    }

    Ok((existing, staleness, false))
}

pub(super) fn token_budget_as_str(budget: &TokenBudget) -> &'static str {
    match budget {
        TokenBudget::Small => "small",
        TokenBudget::Medium => "medium",
        TokenBudget::Large => "large",
    }
}

fn annotate_review_report_with_contextpack(
    report: &mut review_types::ReviewReport,
    staleness: &ContextPackStalenessStatus,
    rebuilt: bool,
    contextpack: Option<&ContextPack>,
) {
    let mut reasons = staleness.reasons.clone();
    if reasons.len() > 6 {
        reasons.truncate(6);
        reasons.push("...more".to_string());
    }

    report.evidence_summary.push(format!(
        "contextpack: present={} stale={} rebuilt={} reasons={} ",
        staleness.present,
        staleness.stale,
        rebuilt,
        if reasons.is_empty() {
            "-".to_string()
        } else {
            reasons.join(",")
        }
    ));

    if let Some(cp) = contextpack {
        let rules_fp = cp
            .source_revisions
            .iter()
            .find(|r| r.r#ref.trim() == "rules:fingerprint")
            .map(|r| r.revision.to_string())
            .unwrap_or_else(|| "-".to_string());

        report.evidence_summary.push(format!(
            "contextpack: token_budget={} generated_at={} source_revisions={} rules_fp={} ",
            token_budget_as_str(&cp.token_budget),
            cp.generated_at,
            cp.source_revisions.len(),
            rules_fp
        ));
    }
}

fn recompute_overall_status(
    issues: &[review_types::ReviewIssue],
) -> review_types::ReviewOverallStatus {
    if issues
        .iter()
        .any(|i| i.severity == review_types::ReviewSeverity::Block)
    {
        return review_types::ReviewOverallStatus::Block;
    }
    if issues
        .iter()
        .any(|i| i.severity == review_types::ReviewSeverity::Warn)
    {
        return review_types::ReviewOverallStatus::Warn;
    }
    review_types::ReviewOverallStatus::Pass
}

fn recompute_recommended_action(
    status: review_types::ReviewOverallStatus,
    issues: &[review_types::ReviewIssue],
) -> review_types::ReviewRecommendedAction {
    match status {
        review_types::ReviewOverallStatus::Pass => review_types::ReviewRecommendedAction::Accept,
        review_types::ReviewOverallStatus::Warn => {
            if issues
                .iter()
                .any(|i| i.severity == review_types::ReviewSeverity::Warn && !i.auto_fixable)
            {
                review_types::ReviewRecommendedAction::Escalate
            } else {
                review_types::ReviewRecommendedAction::Revise
            }
        }
        review_types::ReviewOverallStatus::Block => {
            if issues
                .iter()
                .any(|i| i.severity == review_types::ReviewSeverity::Block && !i.auto_fixable)
            {
                review_types::ReviewRecommendedAction::Escalate
            } else {
                review_types::ReviewRecommendedAction::Revise
            }
        }
    }
}

fn workflow_kind_as_str(kind: &ChapterWorkflowKind) -> &'static str {
    match kind {
        ChapterWorkflowKind::Micro => "micro",
        ChapterWorkflowKind::Chapter => "chapter",
        ChapterWorkflowKind::Arc => "arc",
        ChapterWorkflowKind::Book => "book",
    }
}

fn apply_micro_review_policy(
    project_path: &std::path::Path,
    mission_id: &str,
    report: &mut review_types::ReviewReport,
) {
    let kind = artifacts::read_layer1_chapter_card(project_path, mission_id)
        .ok()
        .flatten()
        .map(|cc| cc.workflow_kind)
        .unwrap_or(ChapterWorkflowKind::Chapter);

    report
        .evidence_summary
        .push(format!("workflow_kind={}", workflow_kind_as_str(&kind)));

    if kind != ChapterWorkflowKind::Micro {
        return;
    }
    if report.overall_status != review_types::ReviewOverallStatus::Block {
        return;
    }

    report
        .evidence_summary
        .push("policy:micro_block_capped".to_string());

    for issue in &mut report.issues {
        if issue.severity == review_types::ReviewSeverity::Block {
            issue.severity = review_types::ReviewSeverity::Warn;
        }
    }

    report.overall_status = recompute_overall_status(&report.issues);
    report.recommended_action =
        recompute_recommended_action(report.overall_status.clone(), &report.issues);
}

pub(super) async fn run_review_gate_with_p1_policies(
    project_path: &std::path::Path,
    mission_id: &str,
    scope_ref: String,
    target_refs: Vec<String>,
    policy: ReviewGatePolicy,
    token_budget: Option<TokenBudget>,
    run_config: Option<&super::MissionRunConfig>,
) -> Result<(review_types::ReviewReport, ReviewGateRunMeta), AppError> {
    let (cp_opt, staleness, rebuilt) = if let Some(budget) = token_budget {
        ensure_contextpack_for_review_with_budget(project_path, mission_id, &scope_ref, budget)?
    } else {
        ensure_contextpack_for_review(project_path, mission_id, &scope_ref)?
    };

    // DevD: prefer ChapterCard-bound rules for reviews (history not polluted).
    // When binding is missing, resolve via effective_from_chapter-aware selector and persist binding.
    let effective_rules_override: Option<crate::writing_rules::types::EffectiveRules> = {
        let rulesets = crate::writing_rules::loader::load_accepted_rulesets(project_path);
        if rulesets.is_empty() {
            None
        } else {
            let now = chrono::Utc::now().timestamp_millis();
            let card = artifacts::read_layer1_chapter_card(project_path, mission_id)
                .ok()
                .flatten();
            let has_binding = card.as_ref().is_some_and(|cc| !cc.rules_sources.is_empty());

            let effective_rules = if has_binding {
                let bound_sources = card
                    .as_ref()
                    .map(|cc| cc.rules_sources.as_slice())
                    .unwrap_or(&[]);
                crate::writing_rules::chapter_binding::resolve_from_binding(
                    project_path,
                    &scope_ref,
                    bound_sources,
                )
                .or_else(|| {
                    let (effective, _binding) =
                        crate::writing_rules::chapter_binding::resolve_and_bind(
                            project_path,
                            &scope_ref,
                        );
                    Some(effective)
                })
            } else {
                let (effective, binding) = crate::writing_rules::chapter_binding::resolve_and_bind(
                    project_path,
                    &scope_ref,
                );

                if let Some(mut cc) = card {
                    let fp_missing = cc
                        .rules_fingerprint
                        .as_deref()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true);
                    let sources_missing = cc.rules_sources.is_empty();
                    if (fp_missing || sources_missing) && !binding.rules_sources.is_empty() {
                        cc.schema_version = LAYER1_SCHEMA_VERSION;
                        cc.rules_fingerprint = Some(binding.rules_fingerprint.clone());
                        cc.rules_sources = binding.rules_sources.clone();
                        cc.bound_validation_profile_id = binding.validation_profile_id.clone();
                        cc.bound_style_template_id = binding.style_template_id.clone();
                        cc.updated_at = now;

                        if let Err(e) =
                            artifacts::write_layer1_chapter_card(project_path, mission_id, &cc)
                        {
                            tracing::warn!(
                                target: "mission",
                                mission_id = %mission_id,
                                error = %e,
                                "failed to persist chapter_card rule binding"
                            );
                        }
                    }
                }

                Some(effective)
            };

            effective_rules
        }
    };

    let input = review_types::ReviewRunInput {
        scope_ref,
        target_refs,
        branch_id: None,
        review_types: policy.review_types,
        task_card_ref: None,
        context_pack_ref: Some("contextpacks/contextpack.json".to_string()),
        effective_rules_fingerprint: policy.effective_rules_fingerprint,
        severity_threshold: policy.severity_threshold,
    };

    let llm_config = run_config.map(|cfg| ReviewLlmConfig {
        provider: cfg.provider.clone(),
        model: cfg.model.clone(),
        base_url: cfg.base_url.clone(),
        api_key: cfg.api_key.clone(),
    });

    let mut report = review_engine::run_review_with_runtime(
        project_path,
        input,
        review_engine::ReviewRuntimeOptions {
            contextpack: cp_opt.as_ref(),
            llm_config: llm_config.as_ref(),
            effective_rules_override,
        },
    )
    .await?;
    annotate_review_report_with_contextpack(&mut report, &staleness, rebuilt, cp_opt.as_ref());
    apply_micro_review_policy(project_path, mission_id, &mut report);

    Ok((
        report,
        ReviewGateRunMeta {
            contextpack: cp_opt,
            staleness,
            rebuilt,
        },
    ))
}

pub(super) fn upsert_risk_ledger_from_review(
    project_path: &std::path::Path,
    mission_id: &str,
    report: &review_types::ReviewReport,
) -> Result<(), AppError> {
    use serde_json::json;

    fn review_type_as_str(t: &review_types::ReviewType) -> &'static str {
        match t {
            review_types::ReviewType::WordCount => "word_count",
            review_types::ReviewType::Continuity => "continuity",
            review_types::ReviewType::Logic => "logic",
            review_types::ReviewType::Character => "character",
            review_types::ReviewType::Style => "style",
            review_types::ReviewType::Terminology => "terminology",
            review_types::ReviewType::Foreshadow => "foreshadow",
            review_types::ReviewType::ObjectiveCompletion => "objective_completion",
        }
    }

    let now = chrono::Utc::now().timestamp_millis();
    let path = artifacts::layer1_risk_ledger_path(project_path, mission_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ledger: serde_json::Value = if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !ledger.is_object() {
        ledger = json!({});
    }

    let obj = ledger.as_object_mut().expect("object");

    // Ensure schema_version so UI can parse it.
    match obj.get("schema_version").and_then(|v| v.as_i64()) {
        Some(v) if v == LAYER1_SCHEMA_VERSION as i64 => {}
        _ => {
            obj.insert("schema_version".to_string(), json!(LAYER1_SCHEMA_VERSION));
        }
    }

    obj.entry("ref".to_string())
        .or_insert_with(|| json!(format!("risk_ledger:{}", uuid::Uuid::new_v4())));
    obj.insert("scope_ref".to_string(), json!(report.scope_ref.clone()));

    if !obj.get("items").map(|v| v.is_array()).unwrap_or(false) {
        obj.insert("items".to_string(), json!([]));
    }

    let items = obj
        .get_mut("items")
        .and_then(|v| v.as_array_mut())
        .expect("items array");

    let mut key_to_index: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (idx, it) in items.iter().enumerate() {
        let source = it.get("source").and_then(|v| v.as_str()).unwrap_or("");
        if source != "review" {
            continue;
        }

        let summary = it
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if summary.is_empty() {
            continue;
        }

        if let Some(rt) = it
            .get("review_type")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            key_to_index.insert(format!("{rt}|{summary}"), idx);
            continue;
        }

        let sev = it
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if sev.is_empty() {
            continue;
        }
        key_to_index.insert(format!("{sev}|{summary}"), idx);
    }

    let mut current_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut current_legacy_keys: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for issue in &report.issues {
        let (sev_str, status_str) = match issue.severity {
            review_types::ReviewSeverity::Warn => ("warn", "deferred"),
            review_types::ReviewSeverity::Block => ("block", "open"),
            _ => continue,
        };

        let summary = issue.summary.trim();
        if summary.is_empty() {
            continue;
        }

        let rt = review_type_as_str(&issue.review_type);
        let key = format!("{rt}|{summary}");
        let legacy_key = format!("{sev_str}|{summary}");

        current_keys.insert(key.clone());
        current_legacy_keys.insert(legacy_key.clone());

        let (idx_opt, matched_key) = if let Some(&idx) = key_to_index.get(&key) {
            (Some(idx), key.clone())
        } else if let Some(&idx) = key_to_index.get(&legacy_key) {
            (Some(idx), legacy_key.clone())
        } else {
            (None, String::new())
        };

        let mut new_item = json!({
            "risk_id": format!("risk_{}", uuid::Uuid::new_v4()),
            "severity": sev_str,
            "summary": issue.summary,
            "source": "review",
            "status": status_str,
            "review_type": rt,
        });
        if !issue.evidence_refs.is_empty() {
            new_item["evidence_refs"] =
                serde_json::to_value(&issue.evidence_refs).unwrap_or(json!([]));
        }

        if let Some(idx) = idx_opt {
            if let Some(existing) = items.get(idx) {
                if let Some(risk_id) = existing
                    .get("risk_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    new_item["risk_id"] = json!(risk_id);
                }
            }

            if let Some(existing) = items.get_mut(idx) {
                *existing = new_item;
            }

            if matched_key != key {
                key_to_index.remove(&matched_key);
                key_to_index.insert(key.clone(), idx);
            }
        } else {
            let idx = items.len();
            items.push(new_item);
            key_to_index.insert(key.clone(), idx);
        }
    }

    // Resolve any review-sourced items not present in the current report.
    for it in items.iter_mut() {
        let source = it.get("source").and_then(|v| v.as_str()).unwrap_or("");
        if source != "review" {
            continue;
        }
        let status = it.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status == "resolved" {
            continue;
        }

        let summary = it
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if summary.is_empty() {
            continue;
        }

        let is_current = if let Some(rt) = it
            .get("review_type")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            current_keys.contains(&format!("{rt}|{summary}"))
        } else {
            let sev = it
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if sev.is_empty() {
                false
            } else {
                current_legacy_keys.contains(&format!("{sev}|{summary}"))
            }
        };

        if !is_current {
            if let Some(obj) = it.as_object_mut() {
                obj.insert("status".to_string(), json!("resolved"));
                obj.insert("resolved_at".to_string(), json!(now));
            }
        }
    }

    obj.insert("updated_at".to_string(), json!(now));

    crate::utils::atomic_write::atomic_write_json(&path, &ledger)
}

pub(super) fn review_has_non_auto_fixable_block(report: &review_types::ReviewReport) -> bool {
    report
        .issues
        .iter()
        .any(|i| i.severity == review_types::ReviewSeverity::Block && !i.auto_fixable)
}

pub(super) fn review_block_is_auto_fixable(report: &review_types::ReviewReport) -> bool {
    let mut has_block = false;
    for i in &report.issues {
        if i.severity == review_types::ReviewSeverity::Block {
            has_block = true;
            if !i.auto_fixable {
                return false;
            }
        }
    }
    has_block
}

pub(super) fn build_review_decision_request(
    report: &review_types::ReviewReport,
    feature_id: Option<String>,
) -> review_types::ReviewDecisionRequest {
    let mut summaries = report
        .issues
        .iter()
        .filter(|i| i.severity == review_types::ReviewSeverity::Block)
        .map(|i| i.summary.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        summaries.push("review blocked".to_string());
    }

    review_types::ReviewDecisionRequest {
        schema_version: review_types::REVIEW_DECISION_SCHEMA_VERSION,
        review_id: report.review_id.clone(),
        feature_id,
        scope_ref: report.scope_ref.clone(),
        target_refs: Some(report.target_refs.clone()),
        question: format!("ReviewGate 阻断：{}。请选择下一步。", summaries.join("；")),
        options: vec!["manual_fix_then_resume".to_string(), "auto_fix".to_string()],
        context_summary: report.evidence_summary.clone(),
        created_at: chrono::Utc::now().timestamp_millis(),
    }
}

fn build_fixup_instruction_lines(report: &review_types::ReviewReport) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("只修复以下问题，不改变已通过部分：".to_string());

    let mut idx = 0;
    for issue in &report.issues {
        if issue.severity == review_types::ReviewSeverity::Block {
            idx += 1;
            let mut line = format!("{idx}. {}", issue.summary.trim());
            if let Some(sf) = issue
                .suggested_fix
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                line.push_str(&format!("（建议：{}）", sf));
            }
            lines.push(line);
        }
    }

    if idx == 0 {
        lines.push("(no block issues listed)".to_string());
    }

    lines
}

fn build_fixup_feature(
    mut feature: Feature,
    report: &review_types::ReviewReport,
    fixup_attempt: u32,
) -> Feature {
    let targets = report
        .target_refs
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let mut expected = Vec::new();
    expected.push(format!(
        "Fixup attempt {}/{} for review_id={} targets=[{}]",
        fixup_attempt, REVIEW_FIXUP_MAX_ATTEMPTS, report.review_id, targets
    ));
    expected.extend(build_fixup_instruction_lines(report));
    if !feature.expected_behavior.is_empty() {
        expected.push("----".to_string());
        expected.extend(feature.expected_behavior.clone());
    }

    let mut verify = Vec::new();
    verify.push("修复后重新 review，直到 pass 或用户接受 warn".to_string());
    if !feature.verification_steps.is_empty() {
        verify.push("----".to_string());
        verify.extend(feature.verification_steps.clone());
    }

    feature.description = format!(
        "[ReviewFixup {}/{}] {}",
        fixup_attempt,
        REVIEW_FIXUP_MAX_ATTEMPTS,
        feature.description.trim()
    );
    feature.expected_behavior = expected;
    feature.verification_steps = verify;
    feature
}

pub(super) async fn stop_all_workers_for_review_block(
    project_path: &std::path::Path,
    mission_id: &str,
    exclude_worker_id: Option<&str>,
) {
    for wid in cancel_in_process_delegates(mission_id, exclude_worker_id, true) {
        clear_worker_from_state(project_path, mission_id, &wid);
        append_mission_recovery_log(
            project_path,
            mission_id,
            format!("review gate requested in-process delegate stop for {wid}"),
        );
    }

    let wait_deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let remaining = active_in_process_delegate_worker_ids(mission_id)
            .into_iter()
            .filter(|wid| {
                exclude_worker_id
                    .map(|exclude| exclude != wid)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            break;
        }
        if std::time::Instant::now() >= wait_deadline {
            append_mission_recovery_log(
                project_path,
                mission_id,
                format!(
                    "review gate timed out waiting for in-process delegates to stop: {}",
                    remaining.join(", ")
                ),
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    for (wid, handle) in list_worker_handles(mission_id) {
        if exclude_worker_id.is_some_and(|ex| ex == wid.as_str()) {
            continue;
        }

        // Remove handle first so event monitors skip recovery.
        let _ = remove_worker_handle(mission_id, &wid);
        clear_worker_from_state(project_path, mission_id, &wid);

        let worker = handle.worker.lock().await;
        if let Err(e) = worker.kill(Duration::from_secs(2)).await {
            tracing::warn!(
                target: "mission",
                mission_id = %mission_id,
                worker_id = %wid,
                error = %e,
                "failed to stop worker during review block"
            );
        }
    }
}

pub(super) async fn start_review_fixup_attempt(
    app_handle: tauri::AppHandle,
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    start_cfg: &MissionStartConfig,
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
    feature_id: &str,
    report: &review_types::ReviewReport,
) -> Result<(), AppError> {
    let fix_key = review_fixup_key_for_targets(&report.target_refs);

    let fixup_attempt = {
        if let Some(mut entry) = review_fixup_registry().get_mut(mission_id) {
            if entry.key != fix_key {
                entry.key = fix_key;
                entry.attempts = 0;
            }
            if entry.attempts >= REVIEW_FIXUP_MAX_ATTEMPTS {
                return Err(AppError::invalid_argument(
                    "review fixup attempts exhausted",
                ));
            }
            entry.attempts += 1;
            entry.attempts
        } else {
            review_fixup_registry().insert(
                mission_id.to_string(),
                ReviewFixupTracker {
                    key: fix_key,
                    attempts: 1,
                },
            );
            1
        }
    };

    let feature = orch
        .get_features()?
        .features
        .into_iter()
        .find(|f| f.id == feature_id)
        .ok_or_else(|| AppError::not_found(format!("feature not found: {feature_id}")))?;
    let fix_feature = build_fixup_feature(feature, report, fixup_attempt);

    let old_state = orch
        .get_state()
        .ok()
        .map(|s| s.state)
        .unwrap_or(MissionState::Paused);

    let worker_profile = builtin_general_worker_profile();
    let attempt = 0_u32;

    match start_cfg.delegate_transport {
        DelegateTransportMode::Process => {
            let (worker_id, worker_arc) =
                spawn_and_initialize_worker(project_path, project_path_str, mission_id).await?;
            let delegate_worker = {
                let worker = worker_arc.lock().await;
                worker.clone()
            };
            let start_result = {
                let worker = worker_arc.lock().await;
                start_feature_on_worker(
                    orch,
                    &*worker,
                    emitter,
                    project_path,
                    mission_id,
                    fix_feature.clone(),
                    &start_cfg.run_config,
                    &worker_id,
                    attempt,
                    worker_profile.clone(),
                    SessionSource::ReviewGate,
                    start_cfg.parent_session_id.as_deref(),
                    start_cfg.parent_turn_id,
                    true,
                    false,
                )
                .await
            };

            match start_result {
                Ok(_feature_id_started) => {
                    insert_worker_handle(
                        mission_id,
                        worker_id.clone(),
                        MissionWorkerHandle {
                            worker: Arc::clone(&worker_arc),
                            attempt,
                        },
                    );

                    spawn_process_delegate_supervision_task(
                        app_handle,
                        mission_id.to_string(),
                        project_path_str.to_string(),
                        worker_id.clone(),
                        delegate_worker,
                        fix_feature,
                        worker_profile,
                        start_cfg.clone(),
                        attempt,
                    );

                    let old_state_str = serde_json::to_string(&old_state)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string();
                    if old_state != MissionState::Running {
                        let _ = emitter.state_changed(&old_state_str, "running");
                    }

                    let _ = emitter.fixup_progress(fixup_attempt as i32, "auto fixup started");
                    append_mission_recovery_log(
                        project_path,
                        mission_id,
                        format!(
                            "review fixup attempt {fixup_attempt}/{REVIEW_FIXUP_MAX_ATTEMPTS} started"
                        ),
                    );

                    Ok(())
                }
                Err(e) => {
                    clear_worker_from_state(project_path, mission_id, &worker_id);
                    let _ = remove_worker_handle(mission_id, &worker_id);
                    Err(e)
                }
            }
        }
        DelegateTransportMode::InProcess => {
            let worker_id = format!("wk_{}", uuid::Uuid::new_v4());
            let start_result = start_feature_in_process(
                orch,
                emitter,
                project_path,
                mission_id,
                fix_feature.clone(),
                &start_cfg.run_config,
                &worker_id,
                attempt,
                worker_profile.clone(),
                false,
            )
            .await;

            match start_result {
                Ok(_feature_id_started) => {
                    spawn_in_process_delegate_supervision_task(
                        app_handle,
                        mission_id.to_string(),
                        project_path_str.to_string(),
                        worker_id.clone(),
                        fix_feature,
                        worker_profile,
                        start_cfg.clone(),
                        attempt,
                    );

                    let old_state_str = serde_json::to_string(&old_state)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string();
                    if old_state != MissionState::Running {
                        let _ = emitter.state_changed(&old_state_str, "running");
                    }

                    let _ = emitter.fixup_progress(fixup_attempt as i32, "auto fixup started");
                    append_mission_recovery_log(
                        project_path,
                        mission_id,
                        format!(
                            "review fixup attempt {fixup_attempt}/{REVIEW_FIXUP_MAX_ATTEMPTS} started"
                        ),
                    );

                    Ok(())
                }
                Err(e) => {
                    let _ = orch.update_feature_status(&fix_feature.id, FeatureStatus::Pending);
                    clear_worker_from_state(project_path, mission_id, &worker_id);
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
