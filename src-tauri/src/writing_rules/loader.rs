//! Loader for writing rule assets from `.magic_novel/rules/` subdirectories.
//!
//! Reads RuleSet (YAML), StyleTemplateMeta (YAML) + content (MD),
//! and ValidationProfile (YAML) from their respective subdirectories.
//! Returns empty collections when directories don't exist (no panic).

use std::path::{Path, PathBuf};

use crate::services::knowledge_paths::resolve_knowledge_root_for_read;

use super::types::{
    RuleScope, RuleSet, RuleSetStatus, StyleTemplate, StyleTemplateMeta, ValidationProfile,
};

/// Resolved paths for the writing rules asset directories.
struct RulesDirs {
    rulesets: PathBuf,
    style_templates: PathBuf,
    validation_profiles: PathBuf,
}

fn rules_dirs(project_path: &Path) -> RulesDirs {
    let root = resolve_knowledge_root_for_read(project_path);
    let rules = root.join("rules");
    RulesDirs {
        rulesets: rules.join("rulesets"),
        style_templates: rules.join("style_templates"),
        validation_profiles: rules.join("validation_profiles"),
    }
}

// ── RuleSet loading ──────────────────────────────────────────────────────

/// Load all YAML rulesets from `rules/rulesets/`.
pub fn load_all_rulesets(project_path: &Path) -> Vec<RuleSet> {
    let dir = rules_dirs(project_path).rulesets;
    load_yaml_files_from_dir(&dir)
}

/// Load only `accepted` rulesets.
pub fn load_accepted_rulesets(project_path: &Path) -> Vec<RuleSet> {
    load_all_rulesets(project_path)
        .into_iter()
        .filter(|r| r.status == RuleSetStatus::Accepted)
        .collect()
}

/// Select the latest accepted ruleset for a given scope + scope_ref.
/// "Latest" = highest `version` among accepted rulesets with matching scope/scope_ref.
pub fn select_latest_accepted(
    rulesets: &[RuleSet],
    scope: &RuleScope,
    scope_ref: &str,
) -> Option<RuleSet> {
    rulesets
        .iter()
        .filter(|r| {
            r.status == RuleSetStatus::Accepted && &r.scope == scope && r.scope_ref == scope_ref
        })
        .max_by_key(|r| r.version)
        .cloned()
}

/// Select the accepted ruleset version that was active for a specific chapter.
///
/// "Active for chapter" = the highest-versioned accepted ruleset whose
/// `effective_from_chapter` is <= the given `chapter_ref` in lexicographic order,
/// OR has no `effective_from_chapter` set (treats it as active from the beginning).
///
/// This is the key function for Rule-P3 "history must not be polluted":
/// when reviewing an old chapter, pass its bound `ruleset_id+version` via
/// `select_version_for_binding` instead to get an exact match.
pub fn select_accepted_for_chapter(
    rulesets: &[RuleSet],
    scope: &RuleScope,
    scope_ref: &str,
    chapter_ref: &str,
) -> Option<RuleSet> {
    let chapter_norm = normalize_scope_ref(chapter_ref);
    rulesets
        .iter()
        .filter(|r| {
            r.status == RuleSetStatus::Accepted
                && &r.scope == scope
                && r.scope_ref == scope_ref
                && is_effective_for_chapter(r, &chapter_norm)
        })
        .max_by_key(|r| r.version)
        .cloned()
}

/// Check whether a ruleset's `effective_from_chapter` allows it to apply to `chapter_ref`.
/// If `effective_from_chapter` is None, the ruleset applies from the beginning.
/// Otherwise, `effective_from_chapter` <= `chapter_ref` (lexicographic).
fn is_effective_for_chapter(ruleset: &RuleSet, chapter_ref: &str) -> bool {
    match &ruleset.effective_from_chapter {
        None => true,
        Some(from) => {
            let from_norm = normalize_scope_ref(from);
            // Strip .json suffix for stable comparison
            let from_key = from_norm.strip_suffix(".json").unwrap_or(&from_norm);
            let ch_key = chapter_ref.strip_suffix(".json").unwrap_or(chapter_ref);
            from_key <= ch_key
        }
    }
}

/// Select a specific accepted ruleset by `ruleset_id` and `version`.
/// Used during chapter review to load the exact bound version (history binding).
pub fn select_version_for_binding(
    rulesets: &[RuleSet],
    ruleset_id: &str,
    version: i32,
) -> Option<RuleSet> {
    rulesets
        .iter()
        .find(|r| {
            r.ruleset_id == ruleset_id
                && r.version == version
                && r.status == RuleSetStatus::Accepted
        })
        .cloned()
}

/// Load all accepted versions of a ruleset_id (sorted ascending by version).
pub fn load_ruleset_version_chain(rulesets: &[RuleSet], ruleset_id: &str) -> Vec<RuleSet> {
    let mut chain: Vec<RuleSet> = rulesets
        .iter()
        .filter(|r| r.ruleset_id == ruleset_id && r.status == RuleSetStatus::Accepted)
        .cloned()
        .collect();
    chain.sort_by_key(|r| r.version);
    chain
}

// ── ValidationProfile loading ────────────────────────────────────────────

/// Load all validation profiles from `rules/validation_profiles/`.
pub fn load_all_validation_profiles(project_path: &Path) -> Vec<ValidationProfile> {
    let dir = rules_dirs(project_path).validation_profiles;
    load_yaml_files_from_dir(&dir)
}

/// Find a validation profile by id.
pub fn load_validation_profile(project_path: &Path, profile_id: &str) -> Option<ValidationProfile> {
    load_all_validation_profiles(project_path)
        .into_iter()
        .find(|p| p.validation_profile_id == profile_id)
}

// ── StyleTemplate loading ────────────────────────────────────────────────

/// Load all style templates (meta YAML + content MD) from `rules/style_templates/`.
pub fn load_all_style_templates(project_path: &Path) -> Vec<StyleTemplate> {
    let dir = rules_dirs(project_path).style_templates;
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.ends_with(".meta.yaml") {
            continue;
        }
        let meta: StyleTemplateMeta = match read_yaml_file(&path) {
            Some(m) => m,
            None => continue,
        };
        // Derive content file: `{template_id}.md` in the same directory
        let content_path = dir.join(format!("{}.md", meta.template_id));
        let content = std::fs::read_to_string(&content_path).unwrap_or_default();
        out.push(StyleTemplate { meta, content });
    }
    out
}

/// Find a style template by id.
pub fn load_style_template(project_path: &Path, template_id: &str) -> Option<StyleTemplate> {
    load_all_style_templates(project_path)
        .into_iter()
        .find(|t| t.meta.template_id == template_id)
}

// ── Generic YAML helpers ─────────────────────────────────────────────────

fn load_yaml_files_from_dir<T: serde::de::DeserializeOwned>(dir: &Path) -> Vec<T> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_yaml_file(&path) {
            continue;
        }
        if let Some(val) = read_yaml_file(&path) {
            out.push(val);
        }
    }
    out
}

fn read_yaml_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&text).ok()
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml" | "yml")
    )
}

// ── ScopeRef normalization ───────────────────────────────────────────────

/// Normalize a scope_ref to a canonical form.
///
/// Accepted inputs:
/// - `chapter:vol1/ch1.json`
/// - `chapter:manuscripts/vol1/ch1.json`
/// - `vol1/ch1.json`
///
/// Output: the manuscripts-relative path without prefix, e.g. `vol1/ch1.json`.
pub fn normalize_scope_ref(raw: &str) -> String {
    let s = raw.trim().replace('\\', "/");
    let s = s.strip_prefix("chapter:").unwrap_or(&s);
    let s = s.strip_prefix("manuscripts/").unwrap_or(s);
    s.trim_start_matches('/').to_string()
}

/// Derive volume scope_ref from a chapter scope_ref.
/// e.g. `vol1/ch1.json` → `vol1`
pub fn derive_volume_ref(chapter_ref: &str) -> Option<String> {
    let normalized = normalize_scope_ref(chapter_ref);
    let parts: Vec<&str> = normalized.split('/').collect();
    if parts.len() >= 2 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Derive chapter scope_ref (without extension) for ruleset matching.
/// e.g. `vol1/ch1.json` → `vol1/ch1` (also matches `vol-01__ch-0001` style)
pub fn derive_chapter_ref(chapter_ref: &str) -> String {
    let normalized = normalize_scope_ref(chapter_ref);
    normalized
        .strip_suffix(".json")
        .unwrap_or(&normalized)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::types::*;
    use std::fs;

    fn temp_project() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wr_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_rulesets_dir(project: &Path) -> PathBuf {
        let dir = project.join(".magic_novel").join("rules").join("rulesets");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_profiles_dir(project: &Path) -> PathBuf {
        let dir = project
            .join(".magic_novel")
            .join("rules")
            .join("validation_profiles");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_templates_dir(project: &Path) -> PathBuf {
        let dir = project
            .join(".magic_novel")
            .join("rules")
            .join("style_templates");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── normalize_scope_ref ──────────────────────────────────────────

    #[test]
    fn normalize_scope_ref_strips_chapter_prefix() {
        assert_eq!(
            normalize_scope_ref("chapter:vol1/ch1.json"),
            "vol1/ch1.json"
        );
    }

    #[test]
    fn normalize_scope_ref_strips_manuscripts_prefix() {
        assert_eq!(
            normalize_scope_ref("chapter:manuscripts/vol1/ch1.json"),
            "vol1/ch1.json"
        );
    }

    #[test]
    fn normalize_scope_ref_bare_path() {
        assert_eq!(normalize_scope_ref("vol1/ch1.json"), "vol1/ch1.json");
    }

    #[test]
    fn normalize_scope_ref_backslash() {
        assert_eq!(
            normalize_scope_ref("chapter:vol1\\ch1.json"),
            "vol1/ch1.json"
        );
    }

    // ── derive_volume_ref ────────────────────────────────────────────

    #[test]
    fn derive_volume_ref_extracts_first_segment() {
        assert_eq!(derive_volume_ref("vol1/ch1.json"), Some("vol1".to_string()));
    }

    #[test]
    fn derive_volume_ref_none_for_single_segment() {
        assert_eq!(derive_volume_ref("ch1.json"), None);
    }

    // ── derive_chapter_ref ───────────────────────────────────────────

    #[test]
    fn derive_chapter_ref_strips_json() {
        assert_eq!(derive_chapter_ref("vol1/ch1.json"), "vol1/ch1");
    }

    #[test]
    fn derive_chapter_ref_no_extension() {
        assert_eq!(derive_chapter_ref("vol1/ch1"), "vol1/ch1");
    }

    // ── load_all_rulesets ────────────────────────────────────────────

    #[test]
    fn load_rulesets_empty_when_no_dir() {
        let project = temp_project();
        let result = load_all_rulesets(&project);
        assert!(result.is_empty());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_rulesets_reads_yaml() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);
        let yaml = r#"
schema_version: 1
ruleset_id: writing_constraints
version: 1
status: accepted
scope: global
scope_ref: project
constraints:
  chapter_words:
    min: 2000
    max: 3000
    target: 2400
  pov: third_limited
"#;
        fs::write(dir.join("global.v0001.yaml"), yaml).unwrap();
        let result = load_all_rulesets(&project);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ruleset_id, "writing_constraints");
        assert_eq!(result[0].scope, RuleScope::Global);
        let cw = result[0].constraints.chapter_words.as_ref().unwrap();
        assert_eq!(cw.min, Some(2000));
        assert_eq!(cw.max, Some(3000));
        assert_eq!(cw.target, Some(2400));
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_rulesets_ignores_non_yaml_files() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);
        fs::write(dir.join("readme.md"), "# not a ruleset").unwrap();
        fs::write(dir.join("legacy.json"), "{}").unwrap();
        let result = load_all_rulesets(&project);
        assert!(result.is_empty());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_rulesets_skips_malformed_yaml() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);
        fs::write(dir.join("bad.yaml"), "not: valid: {yaml: [").unwrap();
        let result = load_all_rulesets(&project);
        assert!(result.is_empty());
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_accepted_rulesets_filters_draft() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);

        let accepted = r#"
schema_version: 1
ruleset_id: r1
version: 1
status: accepted
scope: global
scope_ref: project
constraints: {}
"#;
        let draft = r#"
schema_version: 1
ruleset_id: r2
version: 2
status: draft
scope: global
scope_ref: project
constraints: {}
"#;
        fs::write(dir.join("r1.v0001.yaml"), accepted).unwrap();
        fs::write(dir.join("r2.v0002.yaml"), draft).unwrap();

        let result = load_accepted_rulesets(&project);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ruleset_id, "r1");
        let _ = fs::remove_dir_all(&project);
    }

    // ── select_latest_accepted ───────────────────────────────────────

    #[test]
    fn select_latest_accepted_picks_highest_version() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);

        for v in 1..=3 {
            let yaml = format!(
                r#"
schema_version: 1
ruleset_id: wc
version: {}
status: accepted
scope: global
scope_ref: project
constraints: {{}}
"#,
                v
            );
            fs::write(dir.join(format!("global.v{:04}.yaml", v)), yaml).unwrap();
        }

        let all = load_all_rulesets(&project);
        let latest = select_latest_accepted(&all, &RuleScope::Global, "project").unwrap();
        assert_eq!(latest.version, 3);
        let _ = fs::remove_dir_all(&project);
    }

    // ── ValidationProfile loading ────────────────────────────────────

    #[test]
    fn load_validation_profile_by_id() {
        let project = temp_project();
        let dir = setup_profiles_dir(&project);
        let yaml = r#"
schema_version: 1
validation_profile_id: chapter_gate_v1
checks:
  - word_count_check
  - continuity_check
severity_threshold: block
strict_warn: false
auto_fix_on_block: true
"#;
        fs::write(dir.join("chapter_gate_v1.yaml"), yaml).unwrap();
        let p = load_validation_profile(&project, "chapter_gate_v1").unwrap();
        assert_eq!(p.validation_profile_id, "chapter_gate_v1");
        assert_eq!(p.checks.len(), 2);
        assert_eq!(p.severity_threshold, SeverityThreshold::Block);
        assert!(p.auto_fix_on_block);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_validation_profile_missing_returns_none() {
        let project = temp_project();
        assert!(load_validation_profile(&project, "nonexistent").is_none());
        let _ = fs::remove_dir_all(&project);
    }

    // ── StyleTemplate loading ────────────────────────────────────────

    #[test]
    fn load_style_template_reads_meta_and_content() {
        let project = temp_project();
        let dir = setup_templates_dir(&project);

        let meta_yaml = r#"
schema_version: 1
template_id: style_a_v1
status: accepted
summary: "克制、清冷、少解释、重氛围"
source_ref: style_templates/style_a_v1.md
"#;
        fs::write(dir.join("style_a_v1.meta.yaml"), meta_yaml).unwrap();
        fs::write(dir.join("style_a_v1.md"), "保持克制冷清的文风").unwrap();

        let t = load_style_template(&project, "style_a_v1").unwrap();
        assert_eq!(t.meta.template_id, "style_a_v1");
        assert_eq!(t.meta.summary, "克制、清冷、少解释、重氛围");
        assert_eq!(t.content, "保持克制冷清的文风");
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn load_style_template_missing_content_is_empty() {
        let project = temp_project();
        let dir = setup_templates_dir(&project);

        let meta_yaml = r#"
schema_version: 1
template_id: orphan
status: accepted
summary: "no content file"
"#;
        fs::write(dir.join("orphan.meta.yaml"), meta_yaml).unwrap();

        let t = load_style_template(&project, "orphan").unwrap();
        assert!(t.content.is_empty());
        let _ = fs::remove_dir_all(&project);
    }

    // ── Coexistence with legacy rules/*.json ─────────────────────────

    #[test]
    fn loader_ignores_legacy_json_in_rules_root() {
        let project = temp_project();
        let rules_root = project.join(".magic_novel").join("rules");
        fs::create_dir_all(&rules_root).unwrap();
        fs::write(rules_root.join("legacy_knowledge.json"), "{}").unwrap();

        // No rulesets/ subdirectory → empty
        let result = load_all_rulesets(&project);
        assert!(result.is_empty());

        // Now add a real ruleset
        let dir = rules_root.join("rulesets");
        fs::create_dir_all(&dir).unwrap();
        let yaml = r#"
schema_version: 1
ruleset_id: test
version: 1
status: accepted
scope: global
scope_ref: project
constraints: {}
"#;
        fs::write(dir.join("global.v0001.yaml"), yaml).unwrap();
        let result = load_all_rulesets(&project);
        assert_eq!(result.len(), 1);
        let _ = fs::remove_dir_all(&project);
    }

    // ── select_accepted_for_chapter ──────────────────────────────────

    fn make_accepted_yaml(id: &str, version: i32, from_ch: Option<&str>) -> String {
        let from_line = match from_ch {
            Some(ch) => format!("effective_from_chapter: {}\n", ch),
            None => String::new(),
        };
        format!(
            "schema_version: 1\nruleset_id: {}\nversion: {}\nstatus: accepted\n\
             scope: global\nscope_ref: project\nconstraints: {{}}\n{}",
            id, version, from_line
        )
    }

    #[test]
    fn select_accepted_for_chapter_no_effective_from_always_matches() {
        // A ruleset with no effective_from_chapter applies to any chapter.
        let yaml = make_accepted_yaml("wc", 1, None);
        let r: RuleSet = serde_yaml::from_str(&yaml).unwrap();
        let result =
            select_accepted_for_chapter(&[r], &RuleScope::Global, "project", "vol1/ch-0005.json");
        assert!(result.is_some());
    }

    #[test]
    fn select_accepted_for_chapter_respects_effective_from() {
        let project = temp_project();
        let dir = setup_rulesets_dir(&project);

        // v1: effective from ch-0001 (applies to ch-0001 and later)
        fs::write(
            dir.join("wc.v0001.yaml"),
            make_accepted_yaml("wc", 1, Some("vol1/ch-0001")),
        )
        .unwrap();
        // v2: effective from ch-0003 (should NOT apply to ch-0001 or ch-0002)
        fs::write(
            dir.join("wc.v0002.yaml"),
            make_accepted_yaml("wc", 2, Some("vol1/ch-0003")),
        )
        .unwrap();

        let all = load_all_rulesets(&project);

        // ch-0001: only v1 is effective
        let r =
            select_accepted_for_chapter(&all, &RuleScope::Global, "project", "vol1/ch-0001.json")
                .unwrap();
        assert_eq!(r.version, 1, "ch-0001 should pick v1");

        // ch-0002: only v1 is effective (v2 starts from ch-0003)
        let r =
            select_accepted_for_chapter(&all, &RuleScope::Global, "project", "vol1/ch-0002.json")
                .unwrap();
        assert_eq!(r.version, 1, "ch-0002 should pick v1");

        // ch-0003: both v1 and v2 are effective; v2 wins (higher version)
        let r =
            select_accepted_for_chapter(&all, &RuleScope::Global, "project", "vol1/ch-0003.json")
                .unwrap();
        assert_eq!(r.version, 2, "ch-0003 should pick v2");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn select_version_for_binding_finds_exact_version() {
        let yaml1 = make_accepted_yaml("wc", 1, None);
        let yaml2 = make_accepted_yaml("wc", 2, None);
        let r1: RuleSet = serde_yaml::from_str(&yaml1).unwrap();
        let r2: RuleSet = serde_yaml::from_str(&yaml2).unwrap();
        let all = vec![r1, r2];
        let found = select_version_for_binding(&all, "wc", 1).unwrap();
        assert_eq!(found.version, 1);
        let found2 = select_version_for_binding(&all, "wc", 2).unwrap();
        assert_eq!(found2.version, 2);
        assert!(select_version_for_binding(&all, "wc", 99).is_none());
    }

    // ── load_ruleset_version_chain ───────────────────────────────────

    #[test]
    fn load_ruleset_version_chain_returns_sorted_accepted() {
        let v3 = make_accepted_yaml("wc", 3, None);
        let v1 = make_accepted_yaml("wc", 1, None);
        let v2 = make_accepted_yaml("wc", 2, None);
        let r3: RuleSet = serde_yaml::from_str(&v3).unwrap();
        let r1: RuleSet = serde_yaml::from_str(&v1).unwrap();
        let r2: RuleSet = serde_yaml::from_str(&v2).unwrap();
        let chain = load_ruleset_version_chain(&[r3, r1, r2], "wc");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].version, 1);
        assert_eq!(chain[1].version, 2);
        assert_eq!(chain[2].version, 3);
    }
}
