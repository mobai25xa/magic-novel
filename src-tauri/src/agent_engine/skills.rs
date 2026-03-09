//! Agent Engine - Skill definitions and lookup
//!
//! Skills are loaded from two sources:
//! 1. Built-in defaults (hardcoded fallback)
//! 2. User-defined skills from ~/.magic/skills/*.md
//!
//! User skills override built-in skills with the same name.

use serde::{Deserialize, Serialize};

use crate::services::global_config::{self, SkillSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub system_prompt_snippet: String,
    pub enabled: bool,
    pub source: SkillSource,
}

fn builtin_skill_defaults() -> Vec<SkillDefinition> {
    vec![
        SkillDefinition {
            name: "story-architect".to_string(),
            display_name: "Story Architect".to_string(),
            description: "Favor structured planning, scene goals, and consistency checks before drafting prose.".to_string(),
            system_prompt_snippet: "Skill activated: story-architect. Prioritize structured execution steps (goal → constraints → steps → outcome). Before any writing changes, explain structural impact and continuity checkpoints.".to_string(),
            enabled: true,
            source: SkillSource::Builtin,
        },
        SkillDefinition {
            name: "strict-fact-check".to_string(),
            display_name: "Strict Fact Check (Preview)".to_string(),
            description: "Reserved skill entry for strict citation flows. Disabled in current build.".to_string(),
            system_prompt_snippet: String::new(),
            enabled: false,
            source: SkillSource::Builtin,
        },
    ]
}

/// Get all skill definitions, merging built-in defaults with user-defined skills.
/// User skills override built-in skills with the same name.
pub fn get_skill_definitions() -> Vec<SkillDefinition> {
    let mut skills = builtin_skill_defaults();
    let user_skills = global_config::load_user_skills();

    for user_skill in user_skills {
        if let Some(existing) = skills.iter_mut().find(|s| s.name == user_skill.name) {
            // User overrides built-in
            existing.display_name = user_skill.display_name;
            existing.description = user_skill.description;
            existing.system_prompt_snippet = user_skill.prompt_snippet;
            existing.enabled = true;
            existing.source = SkillSource::User;
        } else {
            skills.push(SkillDefinition {
                name: user_skill.name,
                display_name: user_skill.display_name,
                description: user_skill.description,
                system_prompt_snippet: user_skill.prompt_snippet,
                enabled: true,
                source: SkillSource::User,
            });
        }
    }

    skills
}

pub fn get_skill_by_name(name: &str) -> Option<SkillDefinition> {
    let normalized = name.trim().to_lowercase();
    get_skill_definitions()
        .into_iter()
        .find(|s| s.name == normalized)
}

pub fn get_skill_prompt_snippet(name: &str) -> Option<String> {
    let skill = get_skill_by_name(name)?;
    if !skill.enabled {
        return None;
    }
    let snippet = skill.system_prompt_snippet.trim().to_string();
    if snippet.is_empty() {
        None
    } else {
        Some(snippet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_skill_by_name() {
        assert!(get_skill_by_name("story-architect").is_some());
        assert!(get_skill_by_name("Story-Architect").is_some());
        assert!(get_skill_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_get_skill_prompt_snippet_disabled() {
        assert!(get_skill_prompt_snippet("strict-fact-check").is_none());
    }

    #[test]
    fn test_get_skill_prompt_snippet_enabled() {
        let snippet = get_skill_prompt_snippet("story-architect");
        assert!(snippet.is_some());
        assert!(snippet.as_deref().unwrap().contains("story-architect"));
    }

    #[test]
    fn test_get_skill_definitions_returns_at_least_builtins() {
        let defs = get_skill_definitions();
        assert!(defs.len() >= 2);
    }

    #[test]
    fn test_builtin_defaults_have_correct_source() {
        let defs = builtin_skill_defaults();
        for d in &defs {
            assert_eq!(d.source, SkillSource::Builtin);
        }
    }
}
