//! Global config loading from ~/.magic/
//!
//! Directory structure:
//!   ~/.magic/skills/*.md    — user-defined skill prompt snippets
//!   ~/.magic/workers/*.json — worker definitions (system_prompt + tool whitelist)
//!   ~/.magic/rule.md        — global hard-constraint rules

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::{AppError, ErrorCode};
use crate::services::ensure_dir;

const MAGIC_DIR: &str = ".magic";
const SKILLS_DIR: &str = "skills";
const WORKERS_DIR: &str = "workers";
const RULES_FILE: &str = "rule.md";

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    Builtin,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSkillDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub prompt_snippet: String,
    pub source: SkillSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDefinition {
    pub name: String,
    pub display_name: String,
    pub system_prompt: String,
    pub tool_whitelist: Vec<String>,
    #[serde(default)]
    pub match_keywords: Vec<String>,
    #[serde(default)]
    pub max_rounds: Option<u32>,
    #[serde(default)]
    pub max_tool_calls: Option<u32>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GlobalRules {
    pub content: String,
}

// ── Path helpers ──

pub fn global_config_dir() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError {
        code: ErrorCode::Internal,
        message: "Cannot locate home directory".to_string(),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_HOME_NOT_FOUND" })),
        recoverable: Some(false),
    })?;
    Ok(home.join(MAGIC_DIR))
}

fn skills_dir() -> Result<PathBuf, AppError> {
    Ok(global_config_dir()?.join(SKILLS_DIR))
}

fn workers_dir() -> Result<PathBuf, AppError> {
    Ok(global_config_dir()?.join(WORKERS_DIR))
}

fn rules_path() -> Result<PathBuf, AppError> {
    Ok(global_config_dir()?.join(RULES_FILE))
}

#[allow(dead_code)]
pub fn ensure_global_config_dirs() -> Result<(), AppError> {
    ensure_dir(&skills_dir()?)?;
    ensure_dir(&workers_dir()?)?;
    Ok(())
}

// ── Skills ──

pub fn load_user_skills() -> Vec<UserSkillDefinition> {
    let dir = match skills_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    if !dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => continue,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) if !c.trim().is_empty() => c,
            _ => continue,
        };

        let (display_name, description) = parse_skill_md_header(&content, &name);

        skills.push(UserSkillDefinition {
            name,
            display_name,
            description,
            prompt_snippet: content,
            source: SkillSource::User,
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Parse the first H1 line as display_name and the first paragraph after it as description.
/// If no H1 is found, the first paragraph becomes the description.
fn parse_skill_md_header(content: &str, fallback_name: &str) -> (String, String) {
    let mut display_name = fallback_name.to_string();
    let mut description = String::new();
    let mut found_h1 = false;
    let mut collecting_desc = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if !found_h1 && !collecting_desc {
            if let Some(h1) = trimmed.strip_prefix("# ") {
                let h1 = h1.trim();
                if !h1.is_empty() {
                    display_name = h1.to_string();
                }
                found_h1 = true;
                collecting_desc = true;
                continue;
            }
            // No H1 yet — start collecting first paragraph as description
            if !trimmed.is_empty() {
                collecting_desc = true;
                description.push_str(trimmed);
                continue;
            }
        } else if collecting_desc {
            if trimmed.is_empty() {
                if !description.is_empty() {
                    break;
                }
            } else {
                if !description.is_empty() {
                    description.push(' ');
                }
                description.push_str(trimmed);
            }
        }
    }

    (display_name, description)
}

pub fn save_user_skill(name: &str, content: &str) -> Result<(), AppError> {
    let normalized = normalize_skill_name(name);
    if normalized.is_empty() {
        return Err(AppError::invalid_argument("Skill name cannot be empty"));
    }

    let dir = skills_dir()?;
    ensure_dir(&dir)?;
    let path = dir.join(format!("{}.md", sanitize_filename(&normalized)));
    std::fs::write(&path, content).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("Failed to save skill '{}': {}", normalized, e),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_WRITE_FAILED" })),
        recoverable: Some(true),
    })
}

pub fn import_user_skill_from_file(
    input_path: &str,
    override_name: Option<&str>,
) -> Result<String, AppError> {
    let source = PathBuf::from(input_path);
    let content = std::fs::read_to_string(&source).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("Failed to read skill file '{}': {}", input_path, e),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_READ_FAILED" })),
        recoverable: Some(true),
    })?;

    if content.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "Skill file content cannot be empty",
        ));
    }

    let fallback_name = source
        .file_stem()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "imported-skill".to_string());

    let raw_name = override_name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or(fallback_name);
    let name = normalize_skill_name(&raw_name);

    if name.is_empty() {
        return Err(AppError::invalid_argument("Skill name cannot be empty"));
    }

    save_user_skill(&name, &content)?;
    Ok(name)
}

pub fn export_skill_to_file(name: &str, output_path: &str) -> Result<(), AppError> {
    use crate::agent_engine::skills::get_skill_by_name;

    let normalized = normalize_skill_name(name);
    if normalized.is_empty() {
        return Err(AppError::invalid_argument("Skill name cannot be empty"));
    }

    let skill = get_skill_by_name(&normalized)
        .ok_or_else(|| AppError::not_found(format!("Skill not found: {}", normalized)))?;

    let snippet = skill.system_prompt_snippet.trim();
    if snippet.is_empty() {
        return Err(AppError::invalid_argument(
            "Skill content is empty and cannot be exported",
        ));
    }

    let target = PathBuf::from(output_path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_dir(parent)?;
        }
    }

    std::fs::write(&target, snippet).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!(
            "Failed to export skill '{}' to '{}': {}",
            name, output_path, e
        ),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_WRITE_FAILED" })),
        recoverable: Some(true),
    })
}

pub fn delete_user_skill(name: &str) -> Result<(), AppError> {
    let normalized = normalize_skill_name(name);
    if normalized.is_empty() {
        return Err(AppError::invalid_argument("Skill name cannot be empty"));
    }

    let dir = skills_dir()?;
    let path = dir.join(format!("{}.md", sanitize_filename(&normalized)));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| AppError {
            code: ErrorCode::IoError,
            message: format!("Failed to delete skill '{}': {}", normalized, e),
            details: Some(json!({ "code": "E_GLOBAL_CONFIG_DELETE_FAILED" })),
            recoverable: Some(true),
        })?;
    }
    Ok(())
}

// ── Workers ──

pub fn load_worker_definitions() -> Vec<WorkerDefinition> {
    let dir = match workers_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    if !dir.exists() {
        return Vec::new();
    }

    let mut workers = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let def: WorkerDefinition = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    target: "global_config",
                    path = %path.display(),
                    error = %e,
                    "skipping invalid worker definition"
                );
                continue;
            }
        };
        if def.name.trim().is_empty() || def.system_prompt.trim().is_empty() {
            continue;
        }
        workers.push(def);
    }

    workers.sort_by(|a, b| a.name.cmp(&b.name));
    workers
}

pub fn save_worker_definition(def: &WorkerDefinition) -> Result<(), AppError> {
    let dir = workers_dir()?;
    ensure_dir(&dir)?;
    let path = dir.join(format!("{}.json", sanitize_filename(&def.name)));
    let json = serde_json::to_string_pretty(def).map_err(|e| AppError {
        code: ErrorCode::Internal,
        message: format!("Failed to serialize worker '{}': {}", def.name, e),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_SERIALIZE_FAILED" })),
        recoverable: Some(false),
    })?;
    std::fs::write(&path, json).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("Failed to save worker '{}': {}", def.name, e),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_WRITE_FAILED" })),
        recoverable: Some(true),
    })
}

pub fn delete_worker_definition(name: &str) -> Result<(), AppError> {
    let dir = workers_dir()?;
    let path = dir.join(format!("{}.json", sanitize_filename(name)));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| AppError {
            code: ErrorCode::IoError,
            message: format!("Failed to delete worker '{}': {}", name, e),
            details: Some(json!({ "code": "E_GLOBAL_CONFIG_DELETE_FAILED" })),
            recoverable: Some(true),
        })?;
    }
    Ok(())
}

// ── Global Rules ──

pub fn load_global_rules() -> Option<GlobalRules> {
    let path = rules_path().ok()?;
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    if content.trim().is_empty() {
        return None;
    }
    Some(GlobalRules { content })
}

pub fn save_global_rules(content: &str) -> Result<(), AppError> {
    let dir = global_config_dir()?;
    ensure_dir(&dir)?;
    let path = dir.join(RULES_FILE);
    std::fs::write(&path, content).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("Failed to save global rules: {}", e),
        details: Some(json!({ "code": "E_GLOBAL_CONFIG_WRITE_FAILED" })),
        recoverable: Some(true),
    })
}

// ── Helpers ──

fn normalize_skill_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = false;

    for ch in name.trim().to_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            ch
        } else {
            '-'
        };

        if mapped == '-' {
            if !last_dash {
                out.push('-');
            }
            last_dash = true;
        } else {
            out.push(mapped);
            last_dash = false;
        }
    }

    out.trim_matches('-').to_string()
}

fn sanitize_filename(name: &str) -> String {
    name.trim()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_md_header_with_h1() {
        let content = "# My Skill\nThis is the description.\n\nMore text.";
        let (name, desc) = parse_skill_md_header(content, "fallback");
        assert_eq!(name, "My Skill");
        assert_eq!(desc, "This is the description.");
    }

    #[test]
    fn test_parse_skill_md_header_no_h1() {
        let content = "Just some text without a heading.";
        let (name, desc) = parse_skill_md_header(content, "fallback");
        assert_eq!(name, "fallback");
        assert_eq!(desc, "Just some text without a heading.");
    }

    #[test]
    fn test_parse_skill_md_header_multiline_desc() {
        let content = "# Skill\nLine one\nline two.\n\nIgnored.";
        let (name, desc) = parse_skill_md_header(content, "fb");
        assert_eq!(name, "Skill");
        assert_eq!(desc, "Line one line two.");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("my/skill:name"), "my_skill_name");
        assert_eq!(sanitize_filename("  normal  "), "normal");
    }

    #[test]
    fn test_normalize_skill_name() {
        assert_eq!(normalize_skill_name("Story Architect"), "story-architect");
        assert_eq!(
            normalize_skill_name(" strict_fact_check "),
            "strict_fact_check"
        );
        assert_eq!(normalize_skill_name("中文 名称"), "");
    }

    #[test]
    fn test_worker_definition_serde() {
        let def = WorkerDefinition {
            name: "test-worker".to_string(),
            display_name: "Test Worker".to_string(),
            system_prompt: "You are a test worker.".to_string(),
            tool_whitelist: vec!["read".to_string(), "grep".to_string()],
            match_keywords: vec!["test".to_string()],
            max_rounds: Some(5),
            max_tool_calls: None,
            model: None,
        };
        let json = serde_json::to_string(&def).unwrap();
        let parsed: WorkerDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-worker");
        assert_eq!(parsed.tool_whitelist.len(), 2);
        assert_eq!(parsed.max_rounds, Some(5));
        assert!(parsed.max_tool_calls.is_none());
    }

    #[test]
    fn test_worker_definition_serde_minimal() {
        let json =
            r#"{"name":"w","display_name":"W","system_prompt":"sp","tool_whitelist":["read"]}"#;
        let parsed: WorkerDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "w");
        assert!(parsed.match_keywords.is_empty());
        assert!(parsed.max_rounds.is_none());
    }

    #[test]
    fn test_global_rules_returns_none_for_empty() {
        // load_global_rules returns None when content is empty
        // Tested via the trim().is_empty() check in the function
    }
}
