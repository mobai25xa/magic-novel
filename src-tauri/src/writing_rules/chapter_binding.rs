//! Rule-P3: Chapter binding — bind rule versions to chapters at write time,
//! read bound versions at review time.
//!
//! Contract (from guide.md):
//! - At draft-write time: compute EffectiveRules, write `rules_fingerprint` +
//!   `rules_sources[]` into the chapter_card.
//! - At review/fixup time: read `rules_sources[]` from chapter_card and
//!   reconstruct EffectiveRules from those exact versions (not latest).
//! - "History must not be polluted": changing rules after ch-0001 is written
//!   must not change what ch-0001 is reviewed against.

use std::path::Path;

use crate::mission::layer1_types::BoundRuleSource;
use crate::writing_rules::loader::{load_all_rulesets, select_version_for_binding};
use crate::writing_rules::resolver::resolve_from_rulesets;
use crate::writing_rules::types::{EffectiveRules, RuleSource};

// ── Binding record (written into ChapterCard) ────────────────────────────

/// The subset of EffectiveRules fields that must be recorded in a ChapterCard
/// at draft-write time so that future reviews use the same rule versions.
#[derive(Debug, Clone)]
pub struct ChapterRuleBinding {
    pub rules_fingerprint: String,
    pub rules_sources: Vec<BoundRuleSource>,
    /// Redundant, for debug/DevC reads.
    pub validation_profile_id: Option<String>,
    /// Redundant, for debug/DevC reads.
    pub style_template_id: Option<String>,
}

impl ChapterRuleBinding {
    /// Build a binding from a freshly resolved `EffectiveRules`.
    pub fn from_effective(rules: &EffectiveRules) -> Self {
        let rules_sources = rules.sources.iter().map(rule_source_to_bound).collect();
        ChapterRuleBinding {
            rules_fingerprint: rules.rules_fingerprint.clone(),
            rules_sources,
            validation_profile_id: rules.validation_profile_id.clone(),
            style_template_id: rules.style_template_id.clone(),
        }
    }
}

// ── Write-time: resolve and produce binding ──────────────────────────────

/// Resolve EffectiveRules for `scope_ref` using `select_accepted_for_chapter`
/// (respects `effective_from_chapter`) and return both the rules and the
/// binding evidence to write into the ChapterCard.
///
/// Call this when a chapter draft is first written.
pub fn resolve_and_bind(
    project_path: &Path,
    scope_ref: &str,
) -> (EffectiveRules, ChapterRuleBinding) {
    use crate::writing_rules::loader::{
        derive_chapter_ref, derive_volume_ref, normalize_scope_ref, select_accepted_for_chapter,
    };
    use crate::writing_rules::types::RuleScope;

    let all = load_all_rulesets(project_path);
    let normalized = normalize_scope_ref(scope_ref);
    let volume_ref = derive_volume_ref(&normalized);
    let chapter_ref = derive_chapter_ref(&normalized);

    // Select layers using effective_from_chapter-aware selector
    let global = select_accepted_for_chapter(&all, &RuleScope::Global, "project", &normalized);
    let volume = volume_ref
        .as_deref()
        .and_then(|vr| select_accepted_for_chapter(&all, &RuleScope::Volume, vr, &normalized));
    let chapter = select_accepted_for_chapter(&all, &RuleScope::Chapter, &chapter_ref, &normalized);

    // Build a filtered list to pass to the pure resolver
    let mut selected: Vec<crate::writing_rules::types::RuleSet> = Vec::new();
    if let Some(g) = global {
        selected.push(g);
    }
    if let Some(v) = volume {
        selected.push(v);
    }
    if let Some(c) = chapter {
        selected.push(c);
    }

    let effective = resolve_from_rulesets(&selected, scope_ref);
    let binding = ChapterRuleBinding::from_effective(&effective);
    (effective, binding)
}

// ── Review-time: reconstruct from bound versions ─────────────────────────

/// Reconstruct EffectiveRules from the rule versions bound in a ChapterCard.
///
/// This is the key "history must not be polluted" operation:
/// it loads the exact `ruleset_id + version` pairs recorded when the chapter
/// was first written, ignoring any newer versions that may have been accepted
/// since then.
///
/// Returns `None` if the binding is empty.
pub fn resolve_from_binding(
    project_path: &Path,
    scope_ref: &str,
    bound_sources: &[BoundRuleSource],
) -> Option<EffectiveRules> {
    if bound_sources.is_empty() {
        return None;
    }

    let all = load_all_rulesets(project_path);
    let mut selected = Vec::new();

    for bs in bound_sources {
        if let Some(rs) = select_version_for_binding(&all, &bs.ruleset_id, bs.version) {
            selected.push(rs);
        }
        // If a bound version is missing from disk, we skip it and note the
        // potential staleness via rules_fingerprint mismatch at the call site.
    }

    if selected.is_empty() {
        return None;
    }

    Some(resolve_from_rulesets(&selected, scope_ref))
}

/// Check whether the bound fingerprint matches a freshly resolved one.
/// Returns true when the chapter's bound rules are still current.
pub fn is_binding_current(project_path: &Path, scope_ref: &str, bound_fingerprint: &str) -> bool {
    use crate::writing_rules::resolver::resolve_effective_rules;
    let current = resolve_effective_rules(project_path, scope_ref);
    current.rules_fingerprint == bound_fingerprint
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn rule_source_to_bound(rs: &RuleSource) -> BoundRuleSource {
    use crate::writing_rules::types::RuleScope;
    BoundRuleSource {
        scope: match rs.scope {
            RuleScope::Global => "global".to_string(),
            RuleScope::Volume => "volume".to_string(),
            RuleScope::Chapter => "chapter".to_string(),
        },
        scope_ref: rs.scope_ref.clone(),
        ruleset_id: rs.ruleset_id.clone(),
        version: rs.version,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::{
        types::*,
        versioning::{accept_ruleset, AcceptRuleSetParams},
    };
    use std::{fs, path::PathBuf};

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wr_binding_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn accept(
        project: &Path,
        id: &str,
        scope: RuleScope,
        scope_ref: &str,
        min: i32,
        max: i32,
        from_ch: Option<&str>,
    ) -> RuleSet {
        accept_ruleset(
            project,
            AcceptRuleSetParams {
                ruleset_id: id.to_string(),
                scope,
                scope_ref: scope_ref.to_string(),
                constraints: RuleConstraints {
                    chapter_words: Some(ChapterWordsConstraint {
                        min: Some(min),
                        max: Some(max),
                        target: None,
                    }),
                    style_template_id: None,
                    pov: None,
                    writing_notes: vec![],
                    forbidden: vec![],
                },
                validation_profile_id: None,
                effective_from_chapter: from_ch.map(|s| s.to_string()),
                changelog: None,
            },
        )
        .unwrap()
    }

    // ── smoke-rules-04: history not polluted ─────────────────────────
    //
    // Setup: global v1 (2000-3000) effective from ch-0001
    //        global v2 (500-1000)  effective from ch-0002
    //
    // ch-0001 was written under v1 => review must use v1 (2000-3000)
    // ch-0002 written under v2      => review must use v2 (500-1000)

    #[test]
    fn resolve_and_bind_uses_effective_from_chapter() {
        let project = temp_project();

        // v1 effective from ch-0001
        accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            2000,
            3000,
            Some("vol1/ch-0001.json"),
        );
        // v2 effective from ch-0002
        accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            500,
            1000,
            Some("vol1/ch-0002.json"),
        );

        // Writing ch-0001 => should bind v1 (v2 not yet effective)
        let (eff_ch1, binding_ch1) = resolve_and_bind(&project, "chapter:vol1/ch-0001.json");
        let cw1 = eff_ch1.chapter_words.as_ref().unwrap();
        assert_eq!(cw1.min, Some(2000), "ch-0001 should use v1 (2000-3000)");
        assert_eq!(cw1.max, Some(3000));
        assert_eq!(binding_ch1.rules_sources.len(), 1);
        assert_eq!(binding_ch1.rules_sources[0].version, 1);

        // Writing ch-0002 => should bind v2
        let (eff_ch2, binding_ch2) = resolve_and_bind(&project, "chapter:vol1/ch-0002.json");
        let cw2 = eff_ch2.chapter_words.as_ref().unwrap();
        assert_eq!(cw2.min, Some(500), "ch-0002 should use v2 (500-1000)");
        assert_eq!(cw2.max, Some(1000));
        assert_eq!(binding_ch2.rules_sources[0].version, 2);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn resolve_from_binding_reconstructs_exact_version() {
        let project = temp_project();

        // v1 2000-3000
        let v1 = accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            2000,
            3000,
            None,
        );
        // v2 500-1000
        accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            500,
            1000,
            None,
        );

        // Simulate ch-0001 was bound to v1
        let bound = vec![BoundRuleSource {
            scope: "global".to_string(),
            scope_ref: "project".to_string(),
            ruleset_id: v1.ruleset_id.clone(),
            version: v1.version,
        }];

        let eff =
            resolve_from_binding(&project, "vol1/ch-0001.json", &bound).expect("should resolve");
        let cw = eff.chapter_words.as_ref().unwrap();
        assert_eq!(cw.min, Some(2000), "review of ch-0001 must use v1, not v2");
        assert_eq!(cw.max, Some(3000));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn resolve_from_binding_returns_none_for_empty_sources() {
        let project = temp_project();
        let result = resolve_from_binding(&project, "vol1/ch1.json", &[]);
        assert!(result.is_none());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn is_binding_current_detects_stale_fingerprint() {
        let project = temp_project();
        accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            2000,
            3000,
            None,
        );

        // Get current fingerprint
        let (eff, _) = resolve_and_bind(&project, "vol1/ch-0001.json");
        let fp = eff.rules_fingerprint.clone();

        // Still current
        assert!(is_binding_current(&project, "vol1/ch-0001.json", &fp));

        // Add new version => fingerprint changes
        accept(
            &project,
            "global_wc",
            RuleScope::Global,
            "project",
            1000,
            2000,
            None,
        );
        assert!(
            !is_binding_current(&project, "vol1/ch-0001.json", &fp),
            "fingerprint should be stale after rule update"
        );

        let _ = fs::remove_dir_all(&project);
    }
}
