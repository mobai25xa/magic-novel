//! Context loading, injection, and caching for the agent loop.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::commands::agent_engine::AgentEditorState;
use crate::models::{Chapter, VolumeMetadata};
use crate::services::{list_dirs, list_files, read_json};

use super::messages::{AgentMessage, ConversationState, Role};

/// Token budgets for each context injection category.
pub(crate) struct ContextBudgetConfig {
    project_context_chars: usize,
    editor_state_chars: usize,
    writing_rules_chars: usize,
    global_rules_chars: usize,
    skill_prompt_chars: usize,
    total_chars: usize,
}

impl Default for ContextBudgetConfig {
    fn default() -> Self {
        Self {
            project_context_chars: 1600,
            editor_state_chars: 1200,
            writing_rules_chars: 1600,
            global_rules_chars: 1200,
            skill_prompt_chars: 1200,
            total_chars: 7200,
        }
    }
}

/// Fingerprint to detect whether context has changed between turns.
#[derive(Default, Clone)]
pub(crate) struct ContextFingerprint {
    project_structure_hash: u64,
    writing_rules_hash: u64,
    global_rules_hash: u64,
    editor_state_hash: u64,
    active_skill: Option<String>,
    active_chapter: Option<String>,
}

/// Cached context content to avoid recomputing unchanged sections.
pub(crate) struct ContextCache {
    fingerprint: ContextFingerprint,
    merged_text: String,
    cached_at_turn: u32,
    /// Set to true when a `create` tool call invalidates project structure
    project_invalidated: bool,
    /// Set to true when a `.magic_novel/` edit invalidates writing rules
    rules_invalidated: bool,
}

impl ContextCache {
    pub(crate) fn new() -> Self {
        Self {
            fingerprint: ContextFingerprint::default(),
            merged_text: String::new(),
            cached_at_turn: 0,
            project_invalidated: false,
            rules_invalidated: false,
        }
    }

    pub(crate) fn invalidate_project(&mut self) {
        self.project_invalidated = true;
    }

    pub(crate) fn invalidate_rules(&mut self) {
        self.rules_invalidated = true;
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn inject_unified_context(
    state: &mut ConversationState,
    project_path: &str,
    active_chapter_path: &Option<String>,
    active_skill: &Option<String>,
    editor_state: &Option<AgentEditorState>,
    turn_id: u32,
    cache: &mut ContextCache,
) {
    let budget = ContextBudgetConfig::default();

    // Build fingerprint for current state
    let project_text = if cache.project_invalidated || cache.cached_at_turn == 0 {
        build_project_context_text(project_path, active_chapter_path)
    } else {
        // Reuse cached project text by checking hash
        None
    };
    let project_hash = project_text
        .as_deref()
        .map(hash_str)
        .unwrap_or(cache.fingerprint.project_structure_hash);

    let rules_text = if cache.rules_invalidated || cache.cached_at_turn == 0 {
        read_project_guidelines(project_path)
    } else {
        None
    };
    let rules_hash = rules_text
        .as_deref()
        .map(hash_str)
        .unwrap_or(cache.fingerprint.writing_rules_hash);

    let global_rules_text = if cache.cached_at_turn == 0 {
        crate::services::global_config::load_global_rules().map(|r| r.content)
    } else {
        None
    };
    let global_rules_hash = global_rules_text
        .as_deref()
        .map(hash_str)
        .unwrap_or(cache.fingerprint.global_rules_hash);

    let editor_text = build_editor_state_text(editor_state);
    let editor_hash = editor_text.as_deref().map(hash_str).unwrap_or(0);

    let new_fp = ContextFingerprint {
        project_structure_hash: project_hash,
        writing_rules_hash: rules_hash,
        global_rules_hash,
        editor_state_hash: editor_hash,
        active_skill: active_skill.clone(),
        active_chapter: active_chapter_path.clone(),
    };

    // Check if anything changed
    let fp_changed = cache.cached_at_turn == 0
        || cache.fingerprint.project_structure_hash != new_fp.project_structure_hash
        || cache.fingerprint.writing_rules_hash != new_fp.writing_rules_hash
        || cache.fingerprint.global_rules_hash != new_fp.global_rules_hash
        || cache.fingerprint.editor_state_hash != new_fp.editor_state_hash
        || cache.fingerprint.active_skill != new_fp.active_skill
        || cache.fingerprint.active_chapter != new_fp.active_chapter
        || cache.project_invalidated
        || cache.rules_invalidated;

    if !fp_changed && !cache.merged_text.is_empty() {
        // Update only the session line (turn number changes every round)
        let session_line = build_session_line(state, turn_id, active_chapter_path, active_skill);
        let merged = update_session_line_in_cached(&cache.merged_text, &session_line);
        replace_context_message(state, &merged);
        return;
    }

    // Rebuild merged context
    let mut sections = Vec::new();

    // 1. Session (always include, tiny)
    sections.push(build_session_line(
        state,
        turn_id,
        active_chapter_path,
        active_skill,
    ));

    // 2. Skill prompt
    if let Some(ref skill_name) = active_skill {
        if let Some(snippet) = super::skills::get_skill_prompt_snippet(skill_name) {
            let text = truncate_section(&snippet, budget.skill_prompt_chars);
            sections.push(format!("Active skill: {}", text));
        }
    }

    // 3. Project structure
    let project_section =
        project_text.or_else(|| build_project_context_text(project_path, active_chapter_path));
    if let Some(text) = project_section {
        // Strip the old [Project Context] tag if present
        let clean = text
            .trim_start_matches("[Project Context]\n")
            .trim_start_matches("[Project Context]");
        sections.push(format!(
            "Project:\n{}",
            truncate_section(clean, budget.project_context_chars)
        ));
    }

    // 4. Editor state
    if let Some(text) = &editor_text {
        sections.push(format!(
            "Editor:\n{}",
            truncate_section(text, budget.editor_state_chars)
        ));
    }

    // 5. Writing rules
    let rules_section = rules_text.or_else(|| read_project_guidelines(project_path));
    if let Some(text) = rules_section {
        if !text.trim().is_empty() {
            sections.push(format!(
                "Writing rules:\n{}",
                truncate_section(&text, budget.writing_rules_chars)
            ));
        }
    }

    // 6. Global rules (~/.magic/rule.md) — hard constraints
    let global_rules_section = global_rules_text
        .or_else(|| crate::services::global_config::load_global_rules().map(|r| r.content));
    if let Some(text) = global_rules_section {
        if !text.trim().is_empty() {
            sections.push(format!(
                "Global rules (hard constraints):\n{}",
                truncate_section(&text, budget.global_rules_chars)
            ));
        }
    }

    let merged = format!("[Context]\n{}", sections.join("\n\n"));
    let merged = truncate_section(&merged, budget.total_chars);

    replace_context_message(state, &merged);

    // Update cache
    cache.fingerprint = new_fp;
    cache.merged_text = merged;
    cache.cached_at_turn = turn_id;
    cache.project_invalidated = false;
    cache.rules_invalidated = false;
}

fn build_session_line(
    state: &ConversationState,
    turn_id: u32,
    active_chapter_path: &Option<String>,
    active_skill: &Option<String>,
) -> String {
    let chapter = active_chapter_path.as_deref().unwrap_or("none");
    let skill = active_skill.as_deref().unwrap_or("none");
    format!(
        "Session: session_id={} turn={} active_chapter={} active_skill={}",
        state.session_id, turn_id, chapter, skill
    )
}

fn build_editor_state_text(editor_state: &Option<AgentEditorState>) -> Option<String> {
    let es = editor_state.as_ref()?;
    if es.selected_text.is_none() && es.cursor_paragraph.is_none() {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(ref sel) = es.selected_text {
        if !sel.is_empty() {
            let truncated = if sel.chars().count() > 800 {
                super::text_utils::truncate_chars(sel, 800)
            } else {
                sel.clone()
            };
            parts.push(format!("Selected text: \"{}\"", truncated));
        }
    }
    if let Some(ref para) = es.cursor_paragraph {
        if !para.is_empty() {
            let truncated = if para.chars().count() > 500 {
                super::text_utils::truncate_chars(para, 500)
            } else {
                para.clone()
            };
            let idx_info = es
                .cursor_paragraph_index
                .map(|i| format!("Paragraph {}", i + 1))
                .unwrap_or_default();
            parts.push(format!(
                "Cursor paragraph: {} — \"{}\"",
                idx_info, truncated
            ));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn truncate_section(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        format!(
            "{}[...truncated]",
            super::text_utils::truncate_chars(text, max_chars.saturating_sub(15))
        )
    }
}

fn update_session_line_in_cached(cached: &str, new_session_line: &str) -> String {
    // The cached text starts with "[Context]\n<session line>\n..."
    // Replace the session line (second line) with the new one
    let lines: Vec<&str> = cached.splitn(3, '\n').collect();
    if lines.len() >= 2 {
        // lines[0] = "[Context]", lines[1] = old session line, lines[2..] = rest
        let rest = if lines.len() > 2 { lines[2] } else { "" };
        if rest.is_empty() {
            format!("{}\n{}", lines[0], new_session_line)
        } else {
            format!("{}\n{}\n{}", lines[0], new_session_line, rest)
        }
    } else {
        cached.to_string()
    }
}

fn replace_context_message(state: &mut ConversationState, text: &str) {
    // Remove all old-style injection messages
    let old_tags = [
        "[Context]",
        "[Session Context]",
        "[Active Skill]",
        "[Project Context]",
        "[Editor State]",
        "[Writing Rules]",
    ];
    state.messages.retain(|m| {
        if m.role != Role::System {
            return true;
        }
        let content = m.text_content();
        !old_tags.iter().any(|tag| content.starts_with(tag))
    });

    // Insert after the base system prompt
    let insert_pos = state
        .messages
        .iter()
        .position(|m| m.role != Role::System)
        .unwrap_or(state.messages.len());
    state
        .messages
        .insert(insert_pos, AgentMessage::system(text.to_string()));
}

pub(crate) fn build_project_context_text(
    project_path: &str,
    active_chapter_path: &Option<String>,
) -> Option<String> {
    let root = PathBuf::from(project_path);
    let meta_path = root.join("project.json");
    let meta: crate::models::ProjectMetadata = read_json(&meta_path).ok()?;

    let manuscripts_root = root.join("manuscripts");
    let volume_dirs = list_dirs(&manuscripts_root).unwrap_or_default();

    let mut lines = Vec::new();
    lines.push(format!(
        "[Project Context]\nProject: {} | Author: {}",
        meta.name, meta.author
    ));

    let mut total_words: u64 = 0;
    let mut total_chapters: u32 = 0;
    let mut volume_lines = Vec::new();
    let max_volumes = 5;
    let max_chapters_per_vol = 10;

    for (vi, vol_dir) in volume_dirs.iter().enumerate() {
        if vi >= max_volumes {
            volume_lines.push(format!(
                "  ...and {} more volumes",
                volume_dirs.len() - max_volumes
            ));
            break;
        }
        let vol_path = manuscripts_root.join(vol_dir);
        let vol_meta: VolumeMetadata = match read_json(&vol_path.join("volume.json")) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let chapter_files = list_files(&vol_path, ".json").unwrap_or_default();
        let mut ch_lines = Vec::new();
        let mut vol_words: u64 = 0;
        let mut vol_ch_count: u32 = 0;

        for (ci, ch_file) in chapter_files.iter().enumerate() {
            if ch_file == "volume.json" {
                continue;
            }
            vol_ch_count += 1;
            total_chapters += 1;

            if ci >= max_chapters_per_vol {
                ch_lines.push(format!(
                    "      ...and {} more chapters",
                    chapter_files.len() - 1 - max_chapters_per_vol
                ));
                // Still count words for remaining chapters
                for remaining_file in chapter_files.iter().skip(ci + 1) {
                    if remaining_file == "volume.json" {
                        continue;
                    }
                    if let Ok(ch) = read_json::<Chapter>(&vol_path.join(remaining_file)) {
                        vol_words += ch.counts.text_length_no_whitespace.max(0) as u64;
                    }
                }
                break;
            }

            let ch_path_rel = format!("{}/{}", vol_dir, ch_file);
            let ch: Chapter = match read_json(&vol_path.join(ch_file)) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let wc = ch.counts.text_length_no_whitespace.max(0) as u64;
            vol_words += wc;
            let status = ch
                .status
                .as_ref()
                .map(|s| format!(" [{:?}]", s))
                .unwrap_or_default();
            let active_marker = if active_chapter_path.as_deref() == Some(&ch_path_rel) {
                " <- active"
            } else {
                ""
            };
            ch_lines.push(format!(
                "    - {} ({}): {} words{}{}",
                ch.title, ch_path_rel, wc, status, active_marker
            ));
        }

        total_words += vol_words;
        volume_lines.push(format!(
            "  - {} ({}): {} chapters, {} words",
            vol_meta.title, vol_dir, vol_ch_count, vol_words
        ));
        volume_lines.extend(ch_lines);
    }

    lines.push(format!("Volumes ({}):", volume_dirs.len()));
    lines.extend(volume_lines);

    // Knowledge base stats
    let kb_path = root.join(".magic_novel");
    if kb_path.exists() {
        let kb_dirs = list_dirs(&kb_path).unwrap_or_default().len() as u32;
        let kb_files = count_files_recursive(&kb_path);
        lines.push(format!(
            "Knowledge base: {} folders, {} files",
            kb_dirs, kb_files
        ));
    }

    // Active chapter info
    if let Some(ref acp) = active_chapter_path {
        let full_path = manuscripts_root.join(acp);
        if let Ok(ch) = read_json::<Chapter>(&full_path) {
            let wc = ch.counts.text_length_no_whitespace.max(0) as u64;
            lines.push(format!(
                "Active chapter: {} ({}, {} words)",
                acp, ch.title, wc
            ));
        }
    }

    lines.push(format!(
        "Book total: {} chapters, {} words",
        total_chapters, total_words
    ));

    let text = lines.join("\n");
    // Truncate to 2000 chars
    if text.chars().count() > 2000 {
        Some(super::text_utils::truncate_chars(&text, 2000))
    } else {
        Some(text)
    }
}

fn count_files_recursive(path: &std::path::Path) -> u32 {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    count += 1;
                } else if ft.is_dir() {
                    count += count_files_recursive(&entry.path());
                }
            }
        }
    }
    count
}

/// Read project guidelines from `.magic_novel/guidelines.md`.
fn read_project_guidelines(project_path: &str) -> Option<String> {
    let path = PathBuf::from(project_path)
        .join(".magic_novel")
        .join("guidelines.md");
    if !path.exists() {
        return None;
    }
    std::fs::read_to_string(&path).ok()
}
