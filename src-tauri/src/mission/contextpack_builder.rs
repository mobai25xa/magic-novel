//! Mission system - ContextPackBuilder (M2)

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::kernel::search::corpus_extract::extract_tiptap_text;
use crate::models::{AppError, Chapter};
use crate::services::{global_config, read_json};

use super::artifacts;
use super::contextpack_staleness::compute_rules_fingerprint;
use super::contextpack_types::*;
use super::layer1_types::*;

#[derive(Debug, Clone, Default)]
pub struct BuildContextPackInput {
    pub scope_ref: Option<String>,
    pub token_budget: Option<TokenBudget>,
    pub active_chapter_path: Option<String>,
    pub selected_text: Option<String>,
}

pub fn build_and_persist_contextpack(
    project_path: &Path,
    mission_id: &str,
    input: BuildContextPackInput,
) -> Result<ContextPack, AppError> {
    let cp = build_contextpack(project_path, mission_id, input)?;
    artifacts::write_latest_contextpack(project_path, mission_id, &cp)?;
    Ok(cp)
}

pub fn build_contextpack(
    project_path: &Path,
    mission_id: &str,
    input: BuildContextPackInput,
) -> Result<ContextPack, AppError> {
    let now = chrono::Utc::now().timestamp_millis();

    let chapter_card =
        artifacts::read_layer1_chapter_card(project_path, mission_id)?.ok_or_else(|| AppError {
            code: crate::models::ErrorCode::InvalidArgument,
            message: "Layer1 chapter_card is missing (mission requires chapter_card.json)"
                .to_string(),
            details: Some(serde_json::json!({ "code": "E_LAYER1_CHAPTER_CARD_MISSING" })),
            recoverable: Some(true),
        })?;

    let scope_ref = input
        .scope_ref
        .clone()
        .or_else(|| Some(chapter_card.scope_ref.clone()))
        .unwrap_or_default()
        .trim()
        .to_string();
    if scope_ref.is_empty() {
        return Err(AppError::invalid_argument(
            "contextpack build requires non-empty scope_ref",
        ));
    }

    let effective_budget =
        input
            .token_budget
            .clone()
            .unwrap_or_else(|| match chapter_card.workflow_kind {
                ChapterWorkflowKind::Micro => TokenBudget::Small,
                ChapterWorkflowKind::Chapter => TokenBudget::Medium,
                ChapterWorkflowKind::Arc | ChapterWorkflowKind::Book => TokenBudget::Large,
            });

    let recent_facts = artifacts::read_layer1_recent_facts(project_path, mission_id)?;
    let active_cast = artifacts::read_layer1_active_cast(project_path, mission_id)?;

    let key_facts = recent_facts
        .as_ref()
        .map(|rf| {
            rf.facts
                .iter()
                .map(|f| f.summary.trim())
                .filter(|s| !s.is_empty())
                .take(20)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let cast_notes = active_cast
        .as_ref()
        .map(|ac| {
            ac.cast
                .iter()
                .filter(|c| {
                    !c.character_ref.trim().is_empty() || !c.current_state_summary.trim().is_empty()
                })
                .take(20)
                .map(|c| ContextPackCastNote {
                    character_ref: c.character_ref.trim().to_string(),
                    summary: c.current_state_summary.trim().to_string(),
                    voice_signals: c.must_keep_voice_signals.as_ref().map(|v| {
                        v.iter()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    }),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut must_keep = chapter_card
        .hard_constraints
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let mut active_constraints = must_keep.clone();
    if let Some(global) = global_config::load_global_rules().map(|r| r.content) {
        let extracted = extract_nonempty_rule_lines(&global, 8);
        for line in extracted {
            active_constraints.push(format!("GLOBAL: {line}"));
        }
    }

    must_keep = dedupe_preserve_order(must_keep);
    active_constraints = dedupe_preserve_order(active_constraints);

    let style_rules = read_project_guidelines_rules(project_path, 16);

    let review_targets = chapter_card
        .success_criteria
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(20)
        .collect::<Vec<_>>();

    let mut evidence_snippets = Vec::new();
    if let Some(sel) = input
        .selected_text
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        evidence_snippets.push(EvidenceSnippet {
            source_ref: "editor:selection".to_string(),
            snippet: truncate_chars(sel, 900),
            reason: "Selected text".to_string(),
            score: 1.0,
        });
    }

    let mut risk_flags = Vec::new();
    let mut source_revisions = Vec::new();
    if chapter_card.updated_at > 0 {
        source_revisions.push(SourceRevision {
            r#ref: "layer1:chapter_card".to_string(),
            revision: chapter_card.updated_at,
        });
    }
    if let Some(rf) = recent_facts.as_ref() {
        if rf.updated_at > 0 {
            source_revisions.push(SourceRevision {
                r#ref: "layer1:recent_facts".to_string(),
                revision: rf.updated_at,
            });
        }
    } else {
        risk_flags.push("missing_layer1:recent_facts".to_string());
    }
    if let Some(ac) = active_cast.as_ref() {
        if ac.updated_at > 0 {
            source_revisions.push(SourceRevision {
                r#ref: "layer1:active_cast".to_string(),
                revision: ac.updated_at,
            });
        }
    } else {
        risk_flags.push("missing_layer1:active_cast".to_string());
    }

    source_revisions.push(SourceRevision {
        r#ref: "rules:fingerprint".to_string(),
        revision: compute_rules_fingerprint(project_path),
    });

    let chapter_rel = input
        .active_chapter_path
        .clone()
        .or_else(|| chapter_card.scope_locator.clone())
        .map(|p| p.trim().replace('\\', "/"))
        .filter(|p| !p.is_empty());

    if let Some(rel) = chapter_rel.as_deref() {
        let full = manuscripts_root(project_path).join(rel);
        match read_json::<Chapter>(&full) {
            Ok(ch) => {
                let raw_text = extract_tiptap_text(&ch.content);
                let tail = tail_chars(&raw_text, 1100);
                if !tail.trim().is_empty() {
                    evidence_snippets.push(EvidenceSnippet {
                        source_ref: format!("chapter:{}", ch.id),
                        snippet: truncate_chars(&tail, 1200),
                        reason: "Active chapter excerpt (tail)".to_string(),
                        score: 0.7,
                    });
                }
                source_revisions.push(SourceRevision {
                    r#ref: format!("chapter:{}", ch.id),
                    revision: ch.updated_at,
                });
            }
            Err(_) => {
                risk_flags.push(format!("missing_chapter_source:{rel}"));
            }
        }
    }

    evidence_snippets.truncate(8);
    source_revisions = dedupe_revisions(source_revisions);

    Ok(ContextPack {
        schema_version: CONTEXTPACK_SCHEMA_VERSION,
        scope_ref,
        token_budget: effective_budget,
        objective_summary: chapter_card.objective.trim().to_string(),
        must_keep,
        active_constraints,
        key_facts,
        cast_notes,
        evidence_snippets,
        style_rules,
        review_targets,
        risk_flags,
        source_revisions,
        generated_at: now,
    })
}

fn manuscripts_root(project_path: &Path) -> PathBuf {
    project_path.join("manuscripts")
}

fn read_project_guidelines_rules(project_path: &Path, max_lines: usize) -> Vec<String> {
    let path = project_path.join(".magic_novel").join("guidelines.md");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    extract_nonempty_rule_lines(&content, max_lines)
}

fn extract_nonempty_rule_lines(text: &str, max_lines: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip markdown headings to keep rules list compact.
        if trimmed.starts_with('#') {
            continue;
        }
        out.push(trimmed.to_string());
        if out.len() >= max_lines {
            break;
        }
    }
    out
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(15);
    let mut out: String = text.chars().take(keep).collect();
    out.push_str("[...truncated]");
    out
}

fn tail_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let skip = count - max_chars;
    let tail: String = text.chars().skip(skip).collect();
    format!("[...omitted]\n{tail}")
}

fn dedupe_preserve_order(mut items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items.retain(|s| {
        let key = s.trim().to_string();
        if key.is_empty() {
            return false;
        }
        seen.insert(key)
    });
    items
}

fn dedupe_revisions(items: Vec<SourceRevision>) -> Vec<SourceRevision> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let key = item.r#ref.trim();
        if key.is_empty() {
            continue;
        }
        if seen.insert(key.to_string()) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::layer1_types::{ChapterCardStatus, ChapterWorkflowKind};
    use crate::utils::atomic_write::atomic_write_json;
    use std::fs;

    fn temp_project_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn build_contextpack_minimal() {
        let project = temp_project_dir();
        let mission_id = "mis_test_cp";
        let mission_dir = artifacts::mission_dir(&project, mission_id);
        fs::create_dir_all(&mission_dir).unwrap();

        // Write minimal Layer1
        let cc = ChapterCard {
            schema_version: LAYER1_SCHEMA_VERSION,
            scope_ref: "chapter:ch_1".to_string(),
            scope_locator: Some("vol1/ch1.json".to_string()),
            objective: "Rewrite ending".to_string(),
            workflow_kind: ChapterWorkflowKind::Chapter,
            hard_constraints: vec!["Keep POV".to_string()],
            success_criteria: vec!["Ends with hook".to_string()],
            status: ChapterCardStatus::Active,
            updated_at: 123,
        };
        artifacts::write_layer1_chapter_card(&project, mission_id, &cc).unwrap();

        // Write a chapter source
        let manuscripts = project.join("manuscripts").join("vol1");
        fs::create_dir_all(&manuscripts).unwrap();
        let mut ch = Chapter::new("Ch1".to_string());
        ch.id = "ch_1".to_string();
        ch.updated_at = 456;
        ch.content = serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":"Hello world"}]}]
        });
        atomic_write_json(&manuscripts.join("ch1.json"), &ch).unwrap();

        let cp = build_contextpack(
            &project,
            mission_id,
            BuildContextPackInput {
                scope_ref: None,
                token_budget: None,
                active_chapter_path: None,
                selected_text: Some("sel".to_string()),
            },
        )
        .unwrap();

        assert_eq!(cp.schema_version, CONTEXTPACK_SCHEMA_VERSION);
        assert_eq!(cp.scope_ref, "chapter:ch_1");
        assert_eq!(cp.token_budget, TokenBudget::Medium);
        assert!(cp.objective_summary.contains("Rewrite"));
        assert!(!cp.evidence_snippets.is_empty());

        let _ = fs::remove_dir_all(&project);
    }
}
