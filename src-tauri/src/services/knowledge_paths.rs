use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::agent_tools::contracts::FaultDomain;
use crate::agent_tools::tools::r#ref::{normalize_project_relative_path, RefError};
use crate::models::PlanningDocId;
use crate::models::AppError;
use crate::services::{ensure_dir, write_file};

pub const KNOWLEDGE_ROOT_PRIMARY: &str = ".magic_novel";
pub const KNOWLEDGE_GUIDELINES_FILE: &str = "guidelines.md";
pub const KNOWLEDGE_SCAFFOLD_DIRS: &[&str] = &[
    "characters",
    "terms",
    "settings",
    "planning",
    "foreshadow",
    "index",
    "system",
    "task",
];

const KNOWLEDGE_SHORTHAND_ROOTS: &[&str] = &[
    "characters",
    "locations",
    "organizations",
    "rules",
    "terms",
    "plotlines",
    "style_rules",
    "sources",
    "chapter_summaries",
    "recent_facts",
    "foreshadow",
    "planning",
    "settings",
];
const KNOWLEDGE_SHORTHAND_FILES: &[&str] = &[KNOWLEDGE_GUIDELINES_FILE, "branch_state.json"];
const DEFAULT_GUIDELINES_TEMPLATE: &str = "# Guidelines\n";

pub fn knowledge_read_roots(project_path: &Path) -> Vec<PathBuf> {
    let primary = project_path.join(KNOWLEDGE_ROOT_PRIMARY);
    if primary.exists() {
        return vec![primary];
    }
    Vec::new()
}

pub fn resolve_knowledge_root_for_read(project_path: &Path) -> PathBuf {
    project_path.join(KNOWLEDGE_ROOT_PRIMARY)
}

pub fn resolve_knowledge_root_for_write(project_path: &Path) -> Result<PathBuf, AppError> {
    let primary = project_path.join(KNOWLEDGE_ROOT_PRIMARY);
    ensure_dir(&primary)?;
    Ok(primary)
}

pub fn ensure_project_knowledge_scaffold(project_path: &Path) -> Result<(), AppError> {
    let primary = resolve_knowledge_root_for_write(project_path)?;

    for dir_name in KNOWLEDGE_SCAFFOLD_DIRS {
        ensure_dir(&primary.join(dir_name))?;
    }

    let guidelines_path = primary.join(KNOWLEDGE_GUIDELINES_FILE);
    if !guidelines_path.exists() {
        write_file(&guidelines_path, DEFAULT_GUIDELINES_TEMPLATE)?;
    }

    Ok(())
}

pub fn looks_like_knowledge_input(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }

    if let Some(path) = trimmed.strip_prefix("knowledge:") {
        return looks_like_knowledge_path(path);
    }

    looks_like_knowledge_path(trimmed)
}

pub fn normalize_knowledge_virtual_path(raw: &str) -> Result<String, RefError> {
    let trimmed = raw.trim();
    let path = trimmed.strip_prefix("knowledge:").unwrap_or(trimmed);
    let normalized = normalize_project_relative_path(path, false)?;

    if normalized == KNOWLEDGE_ROOT_PRIMARY
        || normalized.starts_with(&format!("{KNOWLEDGE_ROOT_PRIMARY}/"))
    {
        return Ok(normalized);
    }

    if is_known_knowledge_shorthand(&normalized) {
        return normalize_project_relative_path(
            &format!("{KNOWLEDGE_ROOT_PRIMARY}/{normalized}"),
            false,
        );
    }

    Err(RefError {
        code: "E_REF_INVALID",
        fault_domain: FaultDomain::Validation,
        message: "knowledge ref path must be under .magic_novel/ or a known knowledge shorthand"
            .to_string(),
    })
}

pub fn resolve_knowledge_physical_path(project_path: &Path, virtual_path: &str) -> PathBuf {
    let rel = knowledge_rel_path(virtual_path);
    project_path.join(KNOWLEDGE_ROOT_PRIMARY).join(rel)
}

pub fn builtin_knowledge_display_name(virtual_path: &str) -> Option<&'static str> {
    let normalized = virtual_path.trim().trim_end_matches('/');

    if let Some(doc_id) = PlanningDocId::from_relative_path(normalized) {
        return Some(doc_id.display_name());
    }

    match normalized {
        ".magic_novel" => Some("知识库"),
        ".magic_novel/guidelines.md" => Some("创作准则"),
        ".magic_novel/planning" => Some("规划合同"),
        ".magic_novel/characters" => Some("角色资料"),
        ".magic_novel/terms" => Some("术语库"),
        ".magic_novel/settings" => Some("设定资料"),
        ".magic_novel/foreshadow" => Some("伏笔资料"),
        ".magic_novel/index" => Some("索引"),
        ".magic_novel/system" => Some("系统"),
        ".magic_novel/task" => Some("任务"),
        _ => None,
    }
}

/// Map a virtual `.magic_novel/...` path used by tools/UI into the physical knowledge root.
#[allow(dead_code)]
pub fn map_virtual_magic_novel_path(project_path: &Path, virtual_path: &str) -> PathBuf {
    resolve_knowledge_physical_path(project_path, virtual_path)
}

fn looks_like_knowledge_path(raw: &str) -> bool {
    let candidate = raw.trim().replace('\\', "/");
    let candidate = candidate.trim_start_matches("./").trim_end_matches('/');
    if candidate.is_empty() {
        return false;
    }

    if candidate == KNOWLEDGE_ROOT_PRIMARY
        || candidate.starts_with(&format!("{KNOWLEDGE_ROOT_PRIMARY}/"))
    {
        return true;
    }

    is_known_knowledge_shorthand(candidate)
}

fn is_known_knowledge_shorthand(path: &str) -> bool {
    let normalized = path.trim().trim_start_matches("./").trim_end_matches('/');
    if normalized.is_empty() {
        return false;
    }

    let first_segment = normalized.split('/').next().unwrap_or_default();
    KNOWLEDGE_SHORTHAND_ROOTS.contains(&first_segment)
        || KNOWLEDGE_SHORTHAND_FILES.contains(&first_segment)
}

fn knowledge_rel_path(virtual_path: &str) -> String {
    virtual_path
        .trim()
        .trim_start_matches("./")
        .trim_start_matches(KNOWLEDGE_ROOT_PRIMARY)
        .trim_start_matches('/')
        .to_string()
}

pub fn knowledge_top_level_dirs(project_path: &Path) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for root in knowledge_read_roots(project_path) {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };

        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    dirs.insert(name.to_string());
                }
            }
        }
    }

    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scaffold_creates_primary_layout() {
        let dir = tempdir().expect("temp");
        ensure_project_knowledge_scaffold(dir.path()).expect("scaffold");

        let primary = dir.path().join(KNOWLEDGE_ROOT_PRIMARY);
        assert!(primary.join(KNOWLEDGE_GUIDELINES_FILE).exists());
        assert!(primary.join("characters").is_dir());
        assert!(primary.join("terms").is_dir());
        assert!(primary.join("settings").is_dir());
        assert!(primary.join("planning").is_dir());
        assert!(primary.join("foreshadow").is_dir());
    }

    #[test]
    fn normalize_accepts_canonical_and_shorthand_inputs() {
        assert_eq!(
            normalize_knowledge_virtual_path("characters/alice.md").expect("ref"),
            ".magic_novel/characters/alice.md"
        );
        assert_eq!(
            normalize_knowledge_virtual_path("guidelines.md").expect("ref"),
            ".magic_novel/guidelines.md"
        );
    }

    #[test]
    fn write_root_always_resolves_to_primary() {
        let dir = tempdir().expect("temp");
        let write_root = resolve_knowledge_root_for_write(dir.path()).expect("write root");
        assert_eq!(write_root, dir.path().join(KNOWLEDGE_ROOT_PRIMARY));
        assert!(write_root.exists());
    }

    #[test]
    fn physical_path_always_resolves_to_primary_root() {
        let dir = tempdir().expect("temp");
        let physical =
            resolve_knowledge_physical_path(dir.path(), ".magic_novel/characters/alice.md");
        assert_eq!(
            physical,
            dir.path()
                .join(KNOWLEDGE_ROOT_PRIMARY)
                .join("characters")
                .join("alice.md")
        );
    }

    #[test]
    fn builtin_display_names_cover_scaffold_and_planning_contracts() {
        assert_eq!(
            builtin_knowledge_display_name(".magic_novel/planning"),
            Some("规划合同")
        );
        assert_eq!(
            builtin_knowledge_display_name(".magic_novel/planning/narrative_contract.md"),
            Some("叙事合同")
        );
        assert_eq!(
            builtin_knowledge_display_name(".magic_novel/guidelines.md"),
            Some("创作准则")
        );
    }
}
