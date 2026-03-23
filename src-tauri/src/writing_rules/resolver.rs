//! Active Rule Resolution: merges global → volume → chapter rulesets
//! into a single `EffectiveRules` with conflict detection.

use super::loader::{
    derive_chapter_ref, derive_volume_ref, load_accepted_rulesets, normalize_scope_ref,
    select_latest_accepted,
};
use super::types::{
    ChapterWordsConstraint, Conflict, EffectiveRules, RuleScope, RuleSet, RuleSource,
};
use std::collections::HashSet;
use std::path::Path;

/// Resolve effective rules for a given scope_ref (chapter target).
///
/// Loads accepted rulesets from disk, selects global/volume/chapter layers,
/// merges them (chapter > volume > global), detects conflicts, and produces
/// a stable fingerprint.
pub fn resolve_effective_rules(project_path: &Path, scope_ref: &str) -> EffectiveRules {
    let rulesets = load_accepted_rulesets(project_path);
    resolve_from_rulesets(&rulesets, scope_ref)
}

/// Pure-function resolver (testable without filesystem).
pub fn resolve_from_rulesets(rulesets: &[RuleSet], scope_ref: &str) -> EffectiveRules {
    let normalized = normalize_scope_ref(scope_ref);
    let volume_ref = derive_volume_ref(&normalized);
    let chapter_ref = derive_chapter_ref(&normalized);

    // Select layers: global, volume (optional), chapter (optional)
    let global = select_latest_accepted(rulesets, &RuleScope::Global, "project");
    let volume = volume_ref
        .as_deref()
        .and_then(|vr| select_latest_accepted(rulesets, &RuleScope::Volume, vr));
    let chapter = select_latest_accepted(rulesets, &RuleScope::Chapter, &chapter_ref);

    // Collect participating layers (low → high priority)
    let layers: Vec<&RuleSet> = [global.as_ref(), volume.as_ref(), chapter.as_ref()]
        .into_iter()
        .flatten()
        .collect();

    let sources: Vec<RuleSource> = layers.iter().map(|r| to_rule_source(r)).collect();
    let mut conflicts = Vec::new();

    // ── Merge chapter_words (scalar: high priority wins) ─────────────
    let chapter_words = merge_chapter_words(&layers, &sources, &mut conflicts);

    // ── Merge scalar overrides (high priority wins) ──────────────────
    let style_template_id =
        merge_scalar_string(&layers, |r| r.constraints.style_template_id.as_deref());
    let pov = merge_scalar_string(&layers, |r| r.constraints.pov.as_deref());
    let validation_profile_id =
        merge_scalar_string(&layers, |r| r.validation_profile_id.as_deref());

    // ── Merge lists (set union, dedupe, low→high append order) ───────
    let writing_notes = merge_string_list(&layers, |r| &r.constraints.writing_notes);
    let forbidden = merge_string_list(&layers, |r| &r.constraints.forbidden);

    // ── Fingerprint ──────────────────────────────────────────────────
    let rules_fingerprint = compute_fingerprint(&normalized, &sources, &chapter_words, &pov);

    EffectiveRules {
        scope_ref: normalized,
        chapter_words,
        style_template_id,
        pov,
        writing_notes,
        forbidden,
        validation_profile_id,
        rules_fingerprint,
        sources,
        conflicts,
    }
}

// ── Merge helpers ────────────────────────────────────────────────────────

fn merge_chapter_words(
    layers: &[&RuleSet],
    sources: &[RuleSource],
    conflicts: &mut Vec<Conflict>,
) -> Option<ChapterWordsConstraint> {
    // Take the highest-priority layer that defines chapter_words
    let mut result: Option<ChapterWordsConstraint> = None;
    for layer in layers.iter() {
        if let Some(cw) = &layer.constraints.chapter_words {
            result = Some(match result {
                None => cw.clone(),
                Some(prev) => ChapterWordsConstraint {
                    min: cw.min.or(prev.min),
                    max: cw.max.or(prev.max),
                    target: cw.target.or(prev.target),
                },
            });
        }
    }

    // Validate: min must not exceed max
    if let Some(ref cw) = result {
        if let (Some(min), Some(max)) = (cw.min, cw.max) {
            if min > max {
                conflicts.push(Conflict {
                    field: "chapter_words".to_string(),
                    description: format!(
                        "min ({}) > max ({}): invalid range after merge",
                        min, max
                    ),
                    sources: sources.to_vec(),
                });
            }
        }
    }

    result
}

fn merge_scalar_string<F>(layers: &[&RuleSet], extractor: F) -> Option<String>
where
    F: Fn(&RuleSet) -> Option<&str>,
{
    // Last (highest priority) layer with a value wins
    layers
        .iter()
        .rev()
        .find_map(|r| extractor(r).map(|s| s.to_string()))
}

fn merge_string_list<F>(layers: &[&RuleSet], extractor: F) -> Vec<String>
where
    F: Fn(&RuleSet) -> &Vec<String>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for layer in layers.iter() {
        for item in extractor(layer) {
            let trimmed = item.trim().to_string();
            if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
                out.push(trimmed);
            }
        }
    }
    out
}

// ── Fingerprint ──────────────────────────────────────────────────────────

fn compute_fingerprint(
    scope_ref: &str,
    sources: &[RuleSource],
    chapter_words: &Option<ChapterWordsConstraint>,
    pov: &Option<String>,
) -> String {
    use std::hash::{Hash, Hasher};

    struct FnvHasher(u64);
    impl FnvHasher {
        fn new() -> Self {
            Self(0xcbf29ce484222325)
        }
    }
    impl Hasher for FnvHasher {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 ^= b as u64;
                self.0 = self.0.wrapping_mul(0x100000001b3);
            }
        }
    }

    let mut h = FnvHasher::new();
    scope_ref.hash(&mut h);
    for s in sources {
        s.ruleset_id.hash(&mut h);
        s.version.hash(&mut h);
        s.scope_ref.hash(&mut h);
    }
    if let Some(cw) = chapter_words {
        cw.min.hash(&mut h);
        cw.max.hash(&mut h);
        cw.target.hash(&mut h);
    }
    if let Some(p) = pov {
        p.hash(&mut h);
    }

    format!("{:016x}", h.finish())
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn to_rule_source(r: &RuleSet) -> RuleSource {
    RuleSource {
        scope: r.scope.clone(),
        scope_ref: r.scope_ref.clone(),
        ruleset_id: r.ruleset_id.clone(),
        version: r.version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::types::*;

    fn make_ruleset(
        id: &str,
        version: i32,
        scope: RuleScope,
        scope_ref: &str,
        constraints: RuleConstraints,
    ) -> RuleSet {
        RuleSet {
            schema_version: 1,
            ruleset_id: id.to_string(),
            version,
            status: RuleSetStatus::Accepted,
            scope,
            scope_ref: scope_ref.to_string(),
            constraints,
            validation_profile_id: None,
            previous_version: 0,
            effective_from_chapter: None,
            changelog: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn empty_constraints() -> RuleConstraints {
        RuleConstraints {
            chapter_words: None,
            style_template_id: None,
            pov: None,
            writing_notes: Vec::new(),
            forbidden: Vec::new(),
        }
    }

    // ── Empty rulesets ───────────────────────────────────────────────

    #[test]
    fn resolve_empty_rulesets_returns_empty_effective() {
        let result = resolve_from_rulesets(&[], "chapter:vol1/ch1.json");
        assert_eq!(result.scope_ref, "vol1/ch1.json");
        assert!(result.chapter_words.is_none());
        assert!(result.style_template_id.is_none());
        assert!(result.pov.is_none());
        assert!(result.writing_notes.is_empty());
        assert!(result.forbidden.is_empty());
        assert!(result.sources.is_empty());
        assert!(result.conflicts.is_empty());
    }

    // ── Global only ──────────────────────────────────────────────────

    #[test]
    fn resolve_global_only() {
        let global = make_ruleset(
            "wc",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: Some(2400),
                }),
                pov: Some("third_limited".to_string()),
                forbidden: vec!["OOC".to_string()],
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global], "chapter:vol1/ch1.json");
        let cw = result.chapter_words.unwrap();
        assert_eq!(cw.min, Some(2000));
        assert_eq!(cw.max, Some(3000));
        assert_eq!(cw.target, Some(2400));
        assert_eq!(result.pov.as_deref(), Some("third_limited"));
        assert_eq!(result.forbidden, vec!["OOC"]);
        assert_eq!(result.sources.len(), 1);
        assert!(result.conflicts.is_empty());
    }

    // ── Chapter overrides volume overrides global ────────────────────

    #[test]
    fn resolve_chapter_overrides_volume_overrides_global() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: None,
                }),
                pov: Some("first".to_string()),
                style_template_id: Some("style_a".to_string()),
                writing_notes: vec!["note_global".to_string()],
                forbidden: vec!["OOC".to_string()],
            },
        );
        let volume = make_ruleset(
            "v",
            1,
            RuleScope::Volume,
            "vol1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(1500),
                    max: Some(2500),
                    target: None,
                }),
                pov: Some("third_limited".to_string()),
                writing_notes: vec!["note_vol".to_string()],
                ..empty_constraints()
            },
        );
        let chapter = make_ruleset(
            "c",
            1,
            RuleScope::Chapter,
            "vol1/ch1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(3000),
                    max: Some(3500),
                    target: Some(3200),
                }),
                forbidden: vec!["canon_conflict".to_string()],
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global, volume, chapter], "chapter:vol1/ch1.json");

        // chapter_words: chapter wins (all fields specified)
        let cw = result.chapter_words.unwrap();
        assert_eq!(cw.min, Some(3000));
        assert_eq!(cw.max, Some(3500));
        assert_eq!(cw.target, Some(3200));

        // scalar: pov = volume wins (chapter didn't set it)
        assert_eq!(result.pov.as_deref(), Some("third_limited"));

        // scalar: style_template_id = global (nobody else set it)
        assert_eq!(result.style_template_id.as_deref(), Some("style_a"));

        // list merge: writing_notes = union (global + volume)
        assert_eq!(result.writing_notes, vec!["note_global", "note_vol"]);

        // list merge: forbidden = union (global + chapter, deduplicated)
        assert_eq!(result.forbidden, vec!["OOC", "canon_conflict"]);

        // 3 sources
        assert_eq!(result.sources.len(), 3);
        assert!(result.conflicts.is_empty());
    }

    // ── Switching scope: chapter without override falls back to volume ─

    #[test]
    fn resolve_falls_back_to_volume_when_no_chapter_override() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        let volume = make_ruleset(
            "v",
            1,
            RuleScope::Volume,
            "vol1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(1500),
                    max: Some(2500),
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        // ch-0003 has an override
        let ch3 = make_ruleset(
            "c3",
            1,
            RuleScope::Chapter,
            "vol1/ch3",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(3000),
                    max: Some(3500),
                    target: None,
                }),
                ..empty_constraints()
            },
        );

        // ch-0004 has NO override → should get volume rules
        let result = resolve_from_rulesets(
            &[global.clone(), volume.clone(), ch3],
            "chapter:vol1/ch4.json",
        );
        let cw = result.chapter_words.unwrap();
        assert_eq!(cw.min, Some(1500));
        assert_eq!(cw.max, Some(2500));
        assert_eq!(result.sources.len(), 2); // global + volume only
    }

    // ── Conflict detection: min > max ────────────────────────────────

    #[test]
    fn resolve_detects_min_greater_than_max_conflict() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        // Chapter sets min=4000, max=3500 → conflict
        let chapter = make_ruleset(
            "c",
            1,
            RuleScope::Chapter,
            "vol1/ch1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(4000),
                    max: Some(3500),
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global, chapter], "chapter:vol1/ch1.json");
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].field, "chapter_words");
        assert!(result.conflicts[0].description.contains("min"));
        assert!(result.conflicts[0].description.contains("max"));
    }

    // ── Conflict: cross-layer min > max after merge ──────────────────

    #[test]
    fn resolve_detects_cross_layer_min_max_conflict() {
        // Global sets only min=5000, volume sets only max=3000
        // After merge: min=5000 > max=3000 → conflict
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(5000),
                    max: None,
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        let volume = make_ruleset(
            "v",
            1,
            RuleScope::Volume,
            "vol1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: None,
                    max: Some(3000),
                    target: None,
                }),
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global, volume], "chapter:vol1/ch1.json");
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].field, "chapter_words");
    }

    // ── List deduplication ───────────────────────────────────────────

    #[test]
    fn resolve_deduplicates_list_items() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                forbidden: vec!["OOC".to_string(), "canon_conflict".to_string()],
                writing_notes: vec!["keep concise".to_string()],
                ..empty_constraints()
            },
        );
        let volume = make_ruleset(
            "v",
            1,
            RuleScope::Volume,
            "vol1",
            RuleConstraints {
                forbidden: vec!["OOC".to_string(), "info_dump".to_string()],
                writing_notes: vec!["keep concise".to_string(), "show don't tell".to_string()],
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global, volume], "chapter:vol1/ch1.json");
        assert_eq!(result.forbidden, vec!["OOC", "canon_conflict", "info_dump"]);
        assert_eq!(
            result.writing_notes,
            vec!["keep concise", "show don't tell"]
        );
    }

    // ── Empty/whitespace list items are skipped ──────────────────────

    #[test]
    fn resolve_skips_empty_list_items() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                forbidden: vec!["OOC".to_string(), "".to_string(), "  ".to_string()],
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global], "chapter:vol1/ch1.json");
        assert_eq!(result.forbidden, vec!["OOC"]);
    }

    // ── Fingerprint stability ────────────────────────────────────────

    #[test]
    fn fingerprint_is_stable_across_calls() {
        let rulesets = vec![make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: None,
                }),
                pov: Some("first".to_string()),
                ..empty_constraints()
            },
        )];
        let r1 = resolve_from_rulesets(&rulesets, "vol1/ch1.json");
        let r2 = resolve_from_rulesets(&rulesets, "vol1/ch1.json");
        assert_eq!(r1.rules_fingerprint, r2.rules_fingerprint);
    }

    #[test]
    fn fingerprint_changes_when_version_changes() {
        let mk = |v| {
            vec![make_ruleset(
                "g",
                v,
                RuleScope::Global,
                "project",
                RuleConstraints {
                    chapter_words: Some(ChapterWordsConstraint {
                        min: Some(2000),
                        max: Some(3000),
                        target: None,
                    }),
                    ..empty_constraints()
                },
            )]
        };
        let r1 = resolve_from_rulesets(&mk(1), "vol1/ch1.json");
        let r2 = resolve_from_rulesets(&mk(2), "vol1/ch1.json");
        assert_ne!(r1.rules_fingerprint, r2.rules_fingerprint);
    }

    #[test]
    fn fingerprint_changes_when_scope_ref_changes() {
        let rulesets = vec![make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            empty_constraints(),
        )];
        let r1 = resolve_from_rulesets(&rulesets, "vol1/ch1.json");
        let r2 = resolve_from_rulesets(&rulesets, "vol1/ch2.json");
        assert_ne!(r1.rules_fingerprint, r2.rules_fingerprint);
    }

    // ── validation_profile_id scalar override ────────────────────────

    #[test]
    fn resolve_validation_profile_id_override() {
        let mut global = make_ruleset("g", 1, RuleScope::Global, "project", empty_constraints());
        global.validation_profile_id = Some("default_gate".to_string());

        let mut chapter = make_ruleset("c", 1, RuleScope::Chapter, "vol1/ch1", empty_constraints());
        chapter.validation_profile_id = Some("strict_gate".to_string());

        let result = resolve_from_rulesets(&[global, chapter], "chapter:vol1/ch1.json");
        assert_eq!(result.validation_profile_id.as_deref(), Some("strict_gate"));
    }

    // ── Only accepted rulesets participate ────────────────────────────

    #[test]
    fn resolve_ignores_draft_rulesets() {
        let mut draft = make_ruleset(
            "g",
            2,
            RuleScope::Global,
            "project",
            RuleConstraints {
                pov: Some("omniscient".to_string()),
                ..empty_constraints()
            },
        );
        draft.status = RuleSetStatus::Draft;

        let accepted = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                pov: Some("first".to_string()),
                ..empty_constraints()
            },
        );

        // Draft has higher version but should be ignored
        let result = resolve_from_rulesets(&[draft, accepted], "vol1/ch1.json");
        assert_eq!(result.pov.as_deref(), Some("first"));
    }

    // ── Partial chapter_words merge (fill gaps from lower layer) ─────

    #[test]
    fn resolve_chapter_words_partial_override() {
        let global = make_ruleset(
            "g",
            1,
            RuleScope::Global,
            "project",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: Some(2000),
                    max: Some(3000),
                    target: Some(2400),
                }),
                ..empty_constraints()
            },
        );
        // Chapter only overrides target, keeps min/max from global
        let chapter = make_ruleset(
            "c",
            1,
            RuleScope::Chapter,
            "vol1/ch1",
            RuleConstraints {
                chapter_words: Some(ChapterWordsConstraint {
                    min: None,
                    max: None,
                    target: Some(2800),
                }),
                ..empty_constraints()
            },
        );
        let result = resolve_from_rulesets(&[global, chapter], "chapter:vol1/ch1.json");
        let cw = result.chapter_words.unwrap();
        assert_eq!(cw.min, Some(2000)); // from global
        assert_eq!(cw.max, Some(3000)); // from global
        assert_eq!(cw.target, Some(2800)); // from chapter (overrides global's 2400)
        assert!(result.conflicts.is_empty());
    }

    // ── Filesystem-based resolve ─────────────────────────────────────

    #[test]
    fn resolve_effective_rules_from_disk() {
        use std::fs;
        let project =
            std::env::temp_dir().join(format!("wr_resolve_test_{}", uuid::Uuid::new_v4()));
        let dir = project.join(".magic_novel").join("rules").join("rulesets");
        fs::create_dir_all(&dir).unwrap();

        let global_yaml = r#"
schema_version: 1
ruleset_id: wc
version: 1
status: accepted
scope: global
scope_ref: project
constraints:
  chapter_words:
    min: 2000
    max: 3000
  pov: third_limited
  forbidden:
    - OOC
"#;
        fs::write(dir.join("global.v0001.yaml"), global_yaml).unwrap();

        let result = resolve_effective_rules(&project, "chapter:vol1/ch1.json");
        assert_eq!(result.scope_ref, "vol1/ch1.json");
        let cw = result.chapter_words.unwrap();
        assert_eq!(cw.min, Some(2000));
        assert_eq!(cw.max, Some(3000));
        assert_eq!(result.pov.as_deref(), Some("third_limited"));
        assert_eq!(result.forbidden, vec!["OOC"]);
        assert!(!result.rules_fingerprint.is_empty());
        let _ = fs::remove_dir_all(&project);
    }
}
