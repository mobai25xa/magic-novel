//! DevC: EffectiveRules → compact Active Rules summary text.
//!
//! Renders a human-readable, token-efficient summary of the current effective
//! rules for injection into the reminder layer (Layer D of PromptAssembler).
//! Consumed by DevE's reminder builder — DevC provides the raw string.

use crate::writing_rules::types::EffectiveRules;

/// Render a compact Active Rules summary from `EffectiveRules`.
///
/// Output is plain text, ≤12 lines, suitable for injection into
/// `<system-reminder>` after the six standard reminder fields.
///
/// Returns an empty string if there are no meaningful rules to surface.
pub fn render_active_rules_summary(rules: &EffectiveRules) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("[Active Rules: {}]", rules.scope_ref));

    if let Some(cw) = &rules.chapter_words {
        let min_s = cw
            .min
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let max_s = cw
            .max
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let target_s = cw
            .target
            .map(|v| format!(" (target {})", v))
            .unwrap_or_default();
        lines.push(format!("- words: {}–{}{}", min_s, max_s, target_s));
    }

    if let Some(style) = &rules.style_template_id {
        lines.push(format!("- style: {}", style));
    }

    if let Some(pov) = &rules.pov {
        lines.push(format!("- pov: {}", pov));
    }

    if let Some(profile) = &rules.validation_profile_id {
        lines.push(format!("- validation: {}", profile));
    }

    if !rules.forbidden.is_empty() {
        lines.push(format!("- forbidden: {}", rules.forbidden.join(", ")));
    }

    if !rules.writing_notes.is_empty() {
        // Cap at 3 notes to stay within token budget
        let shown: Vec<&str> = rules
            .writing_notes
            .iter()
            .take(3)
            .map(|s| s.as_str())
            .collect();
        lines.push(format!("- notes: {}", shown.join("; ")));
    }

    if !rules.conflicts.is_empty() {
        lines.push(format!(
            "- CONFLICTS({}): {}",
            rules.conflicts.len(),
            rules
                .conflicts
                .iter()
                .map(|c| c.field.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // If only the header line was added and there are no real fields, return empty
    if lines.len() <= 1
        && rules.chapter_words.is_none()
        && rules.style_template_id.is_none()
        && rules.pov.is_none()
        && rules.validation_profile_id.is_none()
        && rules.forbidden.is_empty()
        && rules.writing_notes.is_empty()
    {
        return String::new();
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::types::{ChapterWordsConstraint, Conflict, EffectiveRules};

    fn empty_rules(scope_ref: &str) -> EffectiveRules {
        EffectiveRules {
            scope_ref: scope_ref.to_string(),
            chapter_words: None,
            style_template_id: None,
            pov: None,
            writing_notes: vec![],
            forbidden: vec![],
            validation_profile_id: None,
            rules_fingerprint: "abc123".to_string(),
            sources: vec![],
            conflicts: vec![],
        }
    }

    #[test]
    fn empty_rules_returns_empty_string() {
        let rules = empty_rules("vol1/ch1.json");
        assert_eq!(render_active_rules_summary(&rules), "");
    }

    #[test]
    fn chapter_words_renders_correctly() {
        let mut rules = empty_rules("vol1/ch1.json");
        rules.chapter_words = Some(ChapterWordsConstraint {
            min: Some(2000),
            max: Some(3000),
            target: Some(2400),
        });
        let out = render_active_rules_summary(&rules);
        assert!(out.contains("words: 2000–3000 (target 2400)"), "got: {out}");
        assert!(out.contains("[Active Rules: vol1/ch1.json]"), "got: {out}");
    }

    #[test]
    fn all_fields_render() {
        let mut rules = empty_rules("vol1/ch2.json");
        rules.chapter_words = Some(ChapterWordsConstraint {
            min: Some(1500),
            max: Some(2500),
            target: None,
        });
        rules.style_template_id = Some("style_a_v1".to_string());
        rules.pov = Some("third_limited".to_string());
        rules.validation_profile_id = Some("chapter_gate_v1".to_string());
        rules.forbidden = vec!["OOC".to_string(), "canon_conflict".to_string()];
        rules.writing_notes = vec!["保持克制".to_string()];

        let out = render_active_rules_summary(&rules);
        assert!(out.contains("style: style_a_v1"), "got: {out}");
        assert!(out.contains("pov: third_limited"), "got: {out}");
        assert!(out.contains("validation: chapter_gate_v1"), "got: {out}");
        assert!(out.contains("forbidden: OOC, canon_conflict"), "got: {out}");
        assert!(out.contains("notes: 保持克制"), "got: {out}");
    }

    #[test]
    fn conflicts_surfaced() {
        let mut rules = empty_rules("vol1/ch3.json");
        rules.chapter_words = Some(ChapterWordsConstraint {
            min: Some(4000),
            max: Some(3000),
            target: None,
        });
        rules.conflicts = vec![Conflict {
            field: "chapter_words".to_string(),
            description: "min > max".to_string(),
            sources: vec![],
        }];
        let out = render_active_rules_summary(&rules);
        assert!(out.contains("CONFLICTS(1): chapter_words"), "got: {out}");
    }

    #[test]
    fn writing_notes_capped_at_three() {
        let mut rules = empty_rules("vol1/ch4.json");
        rules.pov = Some("first".to_string());
        rules.writing_notes = vec![
            "note1".to_string(),
            "note2".to_string(),
            "note3".to_string(),
            "note4".to_string(),
        ];
        let out = render_active_rules_summary(&rules);
        assert!(out.contains("note1"), "got: {out}");
        assert!(out.contains("note3"), "got: {out}");
        assert!(!out.contains("note4"), "should be capped at 3; got: {out}");
    }
}
