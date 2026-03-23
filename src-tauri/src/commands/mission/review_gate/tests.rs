use super::*;

use crate::mission::artifacts;
use crate::mission::layer1_types::{ChapterCard, ChapterCardStatus};
use crate::review::types::{
    ReviewConfidence, ReviewIssue, ReviewOverallStatus, ReviewRecommendedAction, ReviewReport,
    ReviewSeverity, ReviewType,
};
use crate::utils::atomic_write::atomic_write_json;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn write_chapter_card_with_locator(
    project: &Path,
    mission_id: &str,
    scope_ref: &str,
    scope_locator: &str,
) {
    let card = ChapterCard {
        schema_version: LAYER1_SCHEMA_VERSION,
        scope_ref: scope_ref.to_string(),
        scope_locator: Some(scope_locator.to_string()),
        objective: "Test objective".to_string(),
        workflow_kind: ChapterWorkflowKind::Chapter,
        hard_constraints: Vec::new(),
        success_criteria: Vec::new(),
        status: ChapterCardStatus::Active,
        updated_at: 10,
        rules_fingerprint: None,
        rules_sources: vec![],
        bound_validation_profile_id: None,
        bound_style_template_id: None,
    };
    artifacts::write_layer1_chapter_card(project, mission_id, &card).unwrap();
}

fn temp_project_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn init_test_mission(project: &Path, mission_id: &str) {
    fs::create_dir_all(artifacts::mission_dir(project, mission_id)).unwrap();
}

fn write_chapter_card(
    project: &Path,
    mission_id: &str,
    workflow_kind: ChapterWorkflowKind,
    hard_constraints: Vec<String>,
    success_criteria: Vec<String>,
) {
    let card = ChapterCard {
        schema_version: LAYER1_SCHEMA_VERSION,
        scope_ref: "chapter:ch_1".to_string(),
        scope_locator: Some("vol1/ch1.json".to_string()),
        objective: "Test objective".to_string(),
        workflow_kind,
        hard_constraints,
        success_criteria,
        status: ChapterCardStatus::Active,
        updated_at: 10,
        rules_fingerprint: None,
        rules_sources: vec![],
        bound_validation_profile_id: None,
        bound_style_template_id: None,
    };
    artifacts::write_layer1_chapter_card(project, mission_id, &card).unwrap();
}

fn write_chapter(project: &Path, rel: &str) {
    let full = project.join("manuscripts").join(rel);
    fs::create_dir_all(full.parent().unwrap()).unwrap();

    let mut ch = crate::models::Chapter::new("Ch1".to_string());
    ch.id = "ch_1".to_string();
    ch.updated_at = 20;
    ch.target_words = Some(100);
    ch.content = serde_json::Value::String((0..80).map(|_| "w").collect::<Vec<_>>().join(" "));
    atomic_write_json(&full, &ch).unwrap();
}

fn sample_report(status: ReviewOverallStatus, issues: Vec<ReviewIssue>) -> ReviewReport {
    ReviewReport {
        schema_version: 1,
        review_id: "rev_test".to_string(),
        scope_ref: "chapter:ch_1".to_string(),
        target_refs: vec!["vol1/ch1.json".to_string()],
        review_types: vec![ReviewType::WordCount],
        overall_status: status,
        issues,
        evidence_summary: Vec::new(),
        recommended_action: ReviewRecommendedAction::Revise,
        generated_at: 1,
    }
}

fn sample_issue(review_type: ReviewType, severity: ReviewSeverity, summary: &str) -> ReviewIssue {
    ReviewIssue {
        issue_id: format!("iss_{}", uuid::Uuid::new_v4()),
        review_type,
        severity,
        summary: summary.to_string(),
        subject_refs: vec!["vol1/ch1.json".to_string()],
        evidence_refs: vec!["target:vol1/ch1.json#word_count=80".to_string()],
        confidence: ReviewConfidence::High,
        suggested_fix: None,
        auto_fixable: true,
    }
}

#[test]
fn review_gate_uses_chapter_card_bound_rules_not_latest_accepted() {
    use crate::writing_rules::types::{ChapterWordsConstraint, RuleConstraints, RuleScope};
    use crate::writing_rules::versioning::{accept_ruleset, AcceptRuleSetParams};

    let project = temp_project_dir();
    let mission_id = "mis_rules_binding";
    init_test_mission(&project, mission_id);
    write_chapter_card_with_locator(&project, mission_id, "chapter:ch-0001", "vol1/ch-0001.json");
    write_chapter(&project, "vol1/ch-0001.json");

    // Global v1: 2000-3000, effective from ch-0001.
    accept_ruleset(
        &project,
        AcceptRuleSetParams {
            ruleset_id: "global_wc".to_string(),
            scope: RuleScope::Global,
            scope_ref: "project".to_string(),
            constraints: RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: None,
                }),
                style_template_id: None,
                pov: None,
                writing_notes: vec![],
                forbidden: vec![],
            },
            validation_profile_id: None,
            effective_from_chapter: Some("vol1/ch-0001.json".to_string()),
            changelog: Some("v1".to_string()),
        },
    )
    .unwrap();

    let scope_ref = infer_review_scope_ref(&project, mission_id);
    assert_eq!(scope_ref, "chapter:vol1/ch-0001.json");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // First review binds v1 and blocks (80 words < 2000 min).
    let (report1, _meta1) = rt
        .block_on(run_review_gate_with_p1_policies(
            &project,
            mission_id,
            scope_ref.clone(),
            vec!["vol1/ch-0001.json".to_string()],
            ReviewGatePolicy {
                review_types: vec![review_types::ReviewType::WordCount],
                severity_threshold: None,
                strict_warn: false,
                auto_fix_on_block: true,
                effective_rules_fingerprint: None,
            },
            None,
            None,
        ))
        .unwrap();
    assert_eq!(report1.overall_status, ReviewOverallStatus::Block);

    let cc1 = artifacts::read_layer1_chapter_card(&project, mission_id)
        .unwrap()
        .unwrap();
    assert!(cc1
        .rules_fingerprint
        .as_deref()
        .is_some_and(|fp| !fp.trim().is_empty()));
    assert_eq!(cc1.rules_sources.len(), 1);
    assert_eq!(cc1.rules_sources[0].version, 1);

    // Global v2: 50-100 would allow the chapter to pass if latest were used.
    accept_ruleset(
        &project,
        AcceptRuleSetParams {
            ruleset_id: "global_wc".to_string(),
            scope: RuleScope::Global,
            scope_ref: "project".to_string(),
            constraints: RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(50),
                    max: Some(100),
                    target: None,
                }),
                style_template_id: None,
                pov: None,
                writing_notes: vec![],
                forbidden: vec![],
            },
            validation_profile_id: None,
            effective_from_chapter: Some("vol1/ch-0002.json".to_string()),
            changelog: Some("v2".to_string()),
        },
    )
    .unwrap();

    // Re-review ch-0001: must still use bound v1 (history not polluted) => still blocks.
    let (report2, _meta2) = rt
        .block_on(run_review_gate_with_p1_policies(
            &project,
            mission_id,
            scope_ref.clone(),
            vec!["vol1/ch-0001.json".to_string()],
            ReviewGatePolicy {
                review_types: vec![review_types::ReviewType::WordCount],
                severity_threshold: None,
                strict_warn: false,
                auto_fix_on_block: true,
                effective_rules_fingerprint: None,
            },
            None,
            None,
        ))
        .unwrap();
    assert_eq!(
        report2.overall_status,
        ReviewOverallStatus::Block,
        "must use bound v1 rules; latest v2 would allow 80 words"
    );

    let cc2 = artifacts::read_layer1_chapter_card(&project, mission_id)
        .unwrap()
        .unwrap();
    assert_eq!(cc2.rules_sources.len(), 1);
    assert_eq!(
        cc2.rules_sources[0].version, 1,
        "binding must not be overwritten"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn filter_chapter_write_targets_only_keeps_valid_chapter_jsons() {
    let project = temp_project_dir();
    write_chapter(&project, "vol1/ch1.json");
    fs::create_dir_all(project.join("manuscripts").join("vol1")).unwrap();
    fs::write(
        project.join("manuscripts").join("vol1").join("notes.txt"),
        "x",
    )
    .unwrap();
    fs::write(
        project.join("manuscripts").join("vol1").join("broken.json"),
        "{}",
    )
    .unwrap();

    let targets = filter_chapter_write_targets(
        &project,
        &[
            "manuscripts/vol1/ch1.json".to_string(),
            "vol1/ch1.json".to_string(),
            "manuscripts/vol1/notes.txt".to_string(),
            "manuscripts/vol1/broken.json".to_string(),
            "../escape.json".to_string(),
        ],
    );

    assert_eq!(targets, vec!["vol1/ch1.json".to_string()]);

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn default_review_types_enable_conditional_gates_from_risk_ledger() {
    let project = temp_project_dir();
    let mission_id = "mis_review_types";
    init_test_mission(&project, mission_id);
    write_chapter_card(
        &project,
        mission_id,
        ChapterWorkflowKind::Chapter,
        Vec::new(),
        Vec::new(),
    );

    let ledger = json!({
        "schema_version": LAYER1_SCHEMA_VERSION,
        "items": [
            {"review_type": "terminology", "summary": "术语需要统一"},
            {"review_type": "foreshadow", "summary": "伏笔需要回收"}
        ]
    });
    atomic_write_json(
        &artifacts::layer1_risk_ledger_path(&project, mission_id),
        &ledger,
    )
    .unwrap();

    let types = default_chapter_review_types(&project, mission_id);

    assert!(types.contains(&ReviewType::Terminology));
    assert!(types.contains(&ReviewType::Foreshadow));

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn run_review_gate_with_p1_policies_rebuilds_contextpack_when_missing() {
    let project = temp_project_dir();
    let mission_id = "mis_contextpack_rebuild";
    init_test_mission(&project, mission_id);
    write_chapter_card(
        &project,
        mission_id,
        ChapterWorkflowKind::Chapter,
        vec!["Keep POV".to_string()],
        vec!["Finish scene".to_string()],
    );
    write_chapter(&project, "vol1/ch1.json");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (report, meta) = rt
        .block_on(run_review_gate_with_p1_policies(
            &project,
            mission_id,
            "chapter:ch_1".to_string(),
            vec!["vol1/ch1.json".to_string()],
            ReviewGatePolicy {
                review_types: vec![review_types::ReviewType::WordCount],
                severity_threshold: None,
                strict_warn: false,
                auto_fix_on_block: true,
                effective_rules_fingerprint: None,
            },
            None,
            None,
        ))
        .unwrap();

    assert!(meta.rebuilt);
    assert!(meta.contextpack.is_some());
    assert!(meta.staleness.stale);
    assert!(artifacts::latest_contextpack_path(&project, mission_id).exists());
    assert!(report
        .evidence_summary
        .iter()
        .any(|line| line.contains("contextpack: present=false stale=true rebuilt=true")));

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn apply_micro_review_policy_caps_block_to_warn() {
    let project = temp_project_dir();
    let mission_id = "mis_micro_policy";
    init_test_mission(&project, mission_id);
    write_chapter_card(
        &project,
        mission_id,
        ChapterWorkflowKind::Micro,
        Vec::new(),
        Vec::new(),
    );

    let mut report = sample_report(
        ReviewOverallStatus::Block,
        vec![sample_issue(
            ReviewType::WordCount,
            ReviewSeverity::Block,
            "too short",
        )],
    );

    apply_micro_review_policy(&project, mission_id, &mut report);

    assert_eq!(report.overall_status, ReviewOverallStatus::Warn);
    assert_eq!(report.issues[0].severity, ReviewSeverity::Warn);
    assert!(report
        .evidence_summary
        .iter()
        .any(|line| line == "policy:micro_block_capped"));

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn upsert_risk_ledger_resolves_missing_review_items() {
    let project = temp_project_dir();
    let mission_id = "mis_risk_ledger";
    init_test_mission(&project, mission_id);

    let first = sample_report(
        ReviewOverallStatus::Block,
        vec![sample_issue(
            ReviewType::Continuity,
            ReviewSeverity::Block,
            "旧问题",
        )],
    );
    upsert_risk_ledger_from_review(&project, mission_id, &first).unwrap();

    let second = sample_report(ReviewOverallStatus::Pass, Vec::new());
    upsert_risk_ledger_from_review(&project, mission_id, &second).unwrap();

    let ledger: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(artifacts::layer1_risk_ledger_path(&project, mission_id)).unwrap(),
    )
    .unwrap();

    let items = ledger.get("items").and_then(|v| v.as_array()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("status").and_then(|v| v.as_str()),
        Some("resolved")
    );
    assert!(items[0]
        .get("resolved_at")
        .and_then(|v| v.as_i64())
        .is_some());

    let _ = fs::remove_dir_all(&project);
}
