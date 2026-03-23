//! Rule-P3: RuleSet versioning — diff, accept, rollback.
//!
//! - `version` monotonically increasing within a `ruleset_id`.
//! - `previous_version` forms a linked chain for traceability.
//! - A `ruleset_id` must be bound to a single `(scope, scope_ref)` pair
//!   (do not reuse the same id across different layers).
//! - `accept` promotes Draft→Accepted as a new version file on disk.
//! - `rollback` re-applies an old version's constraints as a *new* version
//!   (history chain is never deleted).
//! - `effective_from_chapter` controls which chapters a version applies to.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::models::AppError;
use crate::services::knowledge_paths::resolve_knowledge_root_for_read;
use crate::utils::atomic_write::atomic_write;

use super::loader::load_all_rulesets;
use super::types::{RuleConstraints, RuleScope, RuleSet, RuleSetStatus};

// ── RuleSetDiff ──────────────────────────────────────────────────────────

/// A single field-level change between two RuleSet versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Diff between two RuleSet versions (human-readable, stable field order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSetDiff {
    pub ruleset_id: String,
    pub from_version: i32,
    pub to_version: i32,
    pub changes: Vec<FieldChange>,
}

impl RuleSetDiff {
    /// Returns true when no fields changed.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Compute a field-level diff between `from` and `to` RuleSet versions.
pub fn diff_rulesets(from: &RuleSet, to: &RuleSet) -> RuleSetDiff {
    let mut changes = Vec::new();

    diff_option_i32(
        &mut changes,
        "chapter_words.min",
        from.constraints.chapter_words.as_ref().and_then(|c| c.min),
        to.constraints.chapter_words.as_ref().and_then(|c| c.min),
    );
    diff_option_i32(
        &mut changes,
        "chapter_words.max",
        from.constraints.chapter_words.as_ref().and_then(|c| c.max),
        to.constraints.chapter_words.as_ref().and_then(|c| c.max),
    );
    diff_option_i32(
        &mut changes,
        "chapter_words.target",
        from.constraints
            .chapter_words
            .as_ref()
            .and_then(|c| c.target),
        to.constraints.chapter_words.as_ref().and_then(|c| c.target),
    );
    diff_option_str(
        &mut changes,
        "style_template_id",
        from.constraints.style_template_id.as_deref(),
        to.constraints.style_template_id.as_deref(),
    );
    diff_option_str(
        &mut changes,
        "pov",
        from.constraints.pov.as_deref(),
        to.constraints.pov.as_deref(),
    );
    diff_option_str(
        &mut changes,
        "validation_profile_id",
        from.validation_profile_id.as_deref(),
        to.validation_profile_id.as_deref(),
    );
    diff_vec_str(
        &mut changes,
        "writing_notes",
        &from.constraints.writing_notes,
        &to.constraints.writing_notes,
    );
    diff_vec_str(
        &mut changes,
        "forbidden",
        &from.constraints.forbidden,
        &to.constraints.forbidden,
    );
    diff_option_str(
        &mut changes,
        "effective_from_chapter",
        from.effective_from_chapter.as_deref(),
        to.effective_from_chapter.as_deref(),
    );

    RuleSetDiff {
        ruleset_id: from.ruleset_id.clone(),
        from_version: from.version,
        to_version: to.version,
        changes,
    }
}

// ── Accept (Draft -> Accepted new version) ───────────────────────────────

/// Parameters for accepting a ruleset as a new version.
pub struct AcceptRuleSetParams {
    /// The base ruleset_id to create a new version for.
    pub ruleset_id: String,
    /// Scope of the ruleset.
    pub scope: RuleScope,
    /// Scope reference (e.g. "project", "vol1", "vol1/ch1").
    pub scope_ref: String,
    /// The new constraints to accept.
    pub constraints: RuleConstraints,
    /// Optional validation profile override.
    pub validation_profile_id: Option<String>,
    /// Chapter from which this version takes effect (None = from the beginning).
    pub effective_from_chapter: Option<String>,
    /// Human-readable changelog entry.
    pub changelog: Option<String>,
}

/// Accept a ruleset change, writing a new Accepted version file to disk.
///
/// - Loads all existing rulesets to determine `previous_version`.
/// - Assigns `version = previous_version + 1`.
/// - Writes file as `{id}.v{version:04}.yaml`.
/// - Returns the newly created `RuleSet`.
pub fn accept_ruleset(
    project_path: &Path,
    params: AcceptRuleSetParams,
) -> Result<RuleSet, AppError> {
    let all = load_all_rulesets(project_path);
    validate_version_chain_semantics(&all, &params.ruleset_id, &params.scope, &params.scope_ref)?;
    let previous_version = all
        .iter()
        .filter(|r| r.ruleset_id == params.ruleset_id)
        .map(|r| r.version)
        .max()
        .unwrap_or(0);
    let new_version = previous_version + 1;
    let now = now_secs();

    let ruleset = RuleSet {
        schema_version: 1,
        ruleset_id: params.ruleset_id.clone(),
        version: new_version,
        status: RuleSetStatus::Accepted,
        scope: params.scope,
        scope_ref: params.scope_ref,
        constraints: params.constraints,
        validation_profile_id: params.validation_profile_id,
        previous_version,
        effective_from_chapter: params.effective_from_chapter,
        changelog: params.changelog,
        created_at: now,
        updated_at: now,
    };

    write_ruleset_file(project_path, &ruleset)?;
    Ok(ruleset)
}

// ── Rollback (reapply old version as new version) ────────────────────────

/// Rollback to a specific version of a ruleset by creating a *new* version
/// that copies the old version's constraints.
///
/// The full history chain is preserved — no files are deleted.
/// `previous_version` points to the current latest before rollback.
/// `effective_from_chapter` controls when the rolled-back rules take effect.
pub fn rollback_ruleset(
    project_path: &Path,
    ruleset_id: &str,
    target_version: i32,
    effective_from_chapter: Option<String>,
    changelog: Option<String>,
) -> Result<RuleSet, AppError> {
    let all = load_all_rulesets(project_path);

    let target = all
        .iter()
        .find(|r| r.ruleset_id == ruleset_id && r.version == target_version)
        .ok_or_else(|| {
            AppError::not_found(format!(
                "Ruleset '{}' version {} not found",
                ruleset_id, target_version
            ))
        })?;

    validate_version_chain_semantics(&all, ruleset_id, &target.scope, &target.scope_ref)?;

    let previous_version = all
        .iter()
        .filter(|r| r.ruleset_id == ruleset_id)
        .map(|r| r.version)
        .max()
        .unwrap_or(0);
    let new_version = previous_version + 1;
    let now = now_secs();

    let changelog_msg = changelog.unwrap_or_else(|| format!("rollback to v{}", target_version));

    let rolled_back = RuleSet {
        schema_version: 1,
        ruleset_id: ruleset_id.to_string(),
        version: new_version,
        status: RuleSetStatus::Accepted,
        scope: target.scope.clone(),
        scope_ref: target.scope_ref.clone(),
        constraints: target.constraints.clone(),
        validation_profile_id: target.validation_profile_id.clone(),
        previous_version,
        effective_from_chapter,
        changelog: Some(changelog_msg),
        created_at: now,
        updated_at: now,
    };

    write_ruleset_file(project_path, &rolled_back)?;
    Ok(rolled_back)
}

fn validate_version_chain_semantics(
    all: &[RuleSet],
    ruleset_id: &str,
    scope: &RuleScope,
    scope_ref: &str,
) -> Result<(), AppError> {
    // 1) A ruleset_id must not be reused across different (scope, scope_ref).
    if let Some(mismatch) = all
        .iter()
        .find(|r| r.ruleset_id == ruleset_id && (&r.scope != scope || r.scope_ref != scope_ref))
    {
        return Err(AppError::invalid_argument(format!(
            "ruleset_id '{}' is already bound to {:?}/'{}' (found version {}), cannot accept/rollback as {:?}/'{}'",
            ruleset_id,
            mismatch.scope,
            mismatch.scope_ref,
            mismatch.version,
            scope,
            scope_ref
        )));
    }

    // 2) A (scope, scope_ref) must not have multiple ruleset_id chains.
    let mut ids: HashSet<&str> = all
        .iter()
        .filter(|r| &r.scope == scope && r.scope_ref == scope_ref)
        .map(|r| r.ruleset_id.as_str())
        .collect();
    ids.remove(ruleset_id);
    if !ids.is_empty() {
        let mut other: Vec<&str> = ids.into_iter().collect();
        other.sort_unstable();
        return Err(AppError::invalid_argument(format!(
            "{:?}/'{}' already has ruleset_id chain(s): [{}]; refusing to create/use a second chain '{}'",
            scope,
            scope_ref,
            other.join(", "),
            ruleset_id
        )));
    }

    Ok(())
}

// ── Storage helpers ──────────────────────────────────────────────────────

/// Write a RuleSet to disk as `{rulesets_dir}/{id}.v{version:04}.yaml`.
fn write_ruleset_file(project_path: &Path, ruleset: &RuleSet) -> Result<(), AppError> {
    let dir = rulesets_dir(project_path);
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{}.v{:04}.yaml", ruleset.ruleset_id, ruleset.version);
    let path = dir.join(filename);
    let yaml = serde_yaml::to_string(ruleset)
        .map_err(|e| AppError::internal(format!("yaml serialize: {}", e)))?;
    atomic_write(&path, &yaml)
}

fn rulesets_dir(project_path: &Path) -> PathBuf {
    let root = resolve_knowledge_root_for_read(project_path);
    root.join("rules").join("rulesets")
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Diff helpers ─────────────────────────────────────────────────────────

fn diff_option_i32(
    changes: &mut Vec<FieldChange>,
    field: &str,
    before: Option<i32>,
    after: Option<i32>,
) {
    if before != after {
        changes.push(FieldChange {
            field: field.to_string(),
            before: before.map(|v| v.to_string()),
            after: after.map(|v| v.to_string()),
        });
    }
}

fn diff_option_str(
    changes: &mut Vec<FieldChange>,
    field: &str,
    before: Option<&str>,
    after: Option<&str>,
) {
    if before != after {
        changes.push(FieldChange {
            field: field.to_string(),
            before: before.map(|s| s.to_string()),
            after: after.map(|s| s.to_string()),
        });
    }
}

fn diff_vec_str(changes: &mut Vec<FieldChange>, field: &str, before: &[String], after: &[String]) {
    if before != after {
        changes.push(FieldChange {
            field: field.to_string(),
            before: Some(before.join(", ")),
            after: Some(after.join(", ")),
        });
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::types::*;
    use std::fs;

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wr_version_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn base_constraints() -> RuleConstraints {
        RuleConstraints {
            chapter_words: Some(ChapterWordsConstraint {
                min: Some(2000),
                max: Some(3000),
                target: Some(2400),
            }),
            style_template_id: None,
            pov: Some("third_limited".to_string()),
            writing_notes: vec![],
            forbidden: vec!["OOC".to_string()],
        }
    }

    fn make_ruleset(version: i32, constraints: RuleConstraints) -> RuleSet {
        RuleSet {
            schema_version: 1,
            ruleset_id: "wc".into(),
            version,
            status: RuleSetStatus::Accepted,
            scope: RuleScope::Global,
            scope_ref: "project".into(),
            constraints,
            validation_profile_id: None,
            previous_version: version - 1,
            effective_from_chapter: None,
            changelog: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    // ── diff_rulesets ────────────────────────────────────────────────

    #[test]
    fn diff_identical_rulesets_is_empty() {
        let r1 = make_ruleset(1, base_constraints());
        let r2 = make_ruleset(2, base_constraints());
        let diff = diff_rulesets(&r1, &r2);
        assert!(
            diff.is_empty(),
            "expected no changes, got {:?}",
            diff.changes
        );
    }

    #[test]
    fn diff_detects_chapter_words_change() {
        let r1 = make_ruleset(1, base_constraints());
        let mut c2 = base_constraints();
        c2.chapter_words = Some(ChapterWordsConstraint {
            min: Some(1500),
            max: Some(2500),
            target: None,
        });
        let r2 = make_ruleset(2, c2);
        let diff = diff_rulesets(&r1, &r2);
        assert!(!diff.is_empty());
        let fields: Vec<&str> = diff.changes.iter().map(|c| c.field.as_str()).collect();
        assert!(fields.contains(&"chapter_words.min"));
        assert!(fields.contains(&"chapter_words.max"));
        assert!(fields.contains(&"chapter_words.target"));
        assert!(!fields.contains(&"pov"));
    }

    #[test]
    fn diff_detects_pov_and_forbidden_changes() {
        let r1 = make_ruleset(1, base_constraints());
        let mut c2 = base_constraints();
        c2.pov = Some("first".to_string());
        c2.forbidden = vec!["OOC".to_string(), "canon_conflict".to_string()];
        let r2 = make_ruleset(2, c2);
        let diff = diff_rulesets(&r1, &r2);
        let fields: Vec<&str> = diff.changes.iter().map(|c| c.field.as_str()).collect();
        assert!(fields.contains(&"pov"));
        assert!(fields.contains(&"forbidden"));
    }

    // ── accept_ruleset ───────────────────────────────────────────────

    #[test]
    fn accept_creates_first_version() {
        let project = temp_project();
        let result = accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "global_wc".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("init".into()),
            },
        )
        .unwrap();
        assert_eq!(result.version, 1);
        assert_eq!(result.previous_version, 0);
        assert_eq!(result.status, RuleSetStatus::Accepted);
        let file = rulesets_dir(&project).join("global_wc.v0001.yaml");
        assert!(file.exists(), "expected file {:?}", file);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn accept_increments_version_and_sets_previous() {
        let project = temp_project();
        for i in 0..3 {
            let r = accept_ruleset(
                &project,
                AcceptRuleSetParams {
                    ruleset_id: "global_wc".into(),
                    scope: RuleScope::Global,
                    scope_ref: "project".into(),
                    constraints: base_constraints(),
                    validation_profile_id: None,
                    effective_from_chapter: None,
                    changelog: Some(format!("v{}", i + 1)),
                },
            )
            .unwrap();
            assert_eq!(r.version, i + 1);
            assert_eq!(r.previous_version, i);
        }
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn accept_rejects_ruleset_id_reuse_across_scope_ref() {
        let project = temp_project();

        accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "global".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("init".into()),
            },
        )
        .unwrap();

        // Same id, different layer => invalid
        let err = accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "global".into(),
                scope: RuleScope::Volume,
                scope_ref: "vol-01".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("should fail".into()),
            },
        );
        assert!(err.is_err());

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn accept_rejects_multiple_chains_for_same_scope_ref() {
        let project = temp_project();

        accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "global".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("init".into()),
            },
        )
        .unwrap();

        // Same (scope, scope_ref), different id => invalid
        let err = accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "global_alt".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("should fail".into()),
            },
        );
        assert!(err.is_err());

        let _ = fs::remove_dir_all(&project);
    }

    // ── rollback_ruleset ─────────────────────────────────────────────

    #[test]
    fn rollback_creates_new_version_preserving_history() {
        let project = temp_project();

        // v1: 2000-3000
        let v1 = accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "wc".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: base_constraints(),
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("init".into()),
            },
        )
        .unwrap();

        // v2: 500-1000
        let mut c2 = base_constraints();
        c2.chapter_words = Some(ChapterWordsConstraint {
            min: Some(500),
            max: Some(1000),
            target: None,
        });
        accept_ruleset(
            &project,
            AcceptRuleSetParams {
                ruleset_id: "wc".into(),
                scope: RuleScope::Global,
                scope_ref: "project".into(),
                constraints: c2,
                validation_profile_id: None,
                effective_from_chapter: None,
                changelog: Some("changed".into()),
            },
        )
        .unwrap();

        // rollback to v1 => creates v3
        let rolled = rollback_ruleset(
            &project,
            "wc",
            v1.version,
            Some("vol1/ch0010.json".into()),
            None,
        )
        .unwrap();

        assert_eq!(rolled.version, 3);
        assert_eq!(rolled.previous_version, 2);
        assert_eq!(
            rolled.effective_from_chapter.as_deref(),
            Some("vol1/ch0010.json")
        );
        assert_eq!(
            rolled.constraints.chapter_words.as_ref().unwrap().min,
            Some(2000)
        );

        // all three files still exist
        assert!(rulesets_dir(&project).join("wc.v0001.yaml").exists());
        assert!(rulesets_dir(&project).join("wc.v0002.yaml").exists());
        assert!(rulesets_dir(&project).join("wc.v0003.yaml").exists());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn rollback_nonexistent_version_returns_error() {
        let project = temp_project();
        let err = rollback_ruleset(&project, "wc", 99, None, None);
        assert!(err.is_err());
        let _ = fs::remove_dir_all(&project);
    }
}
