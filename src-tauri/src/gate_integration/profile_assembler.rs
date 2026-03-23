//! DevC: ValidationProfile → ReviewRunInput assembler.
//!
//! Maps a `ValidationProfile` (from the writing rules system) to the
//! `review_types` and threshold settings expected by `ReviewRunInput`.
//! This replaces the hard-coded default review type set in the gate.

use crate::review::types::{ReviewRunInput, ReviewType};
use crate::writing_rules::types::{
    CheckType, SeverityThreshold as ProfileSeverity, ValidationProfile,
};

/// Assemble a `ReviewRunInput` from a `ValidationProfile` and a scope/target.
///
/// - `scope_ref`: e.g. `"chapter:vol1/ch1.json"`
/// - `target_refs`: manuscript-relative paths for the chapter(s) to review
/// - `profile`: the ValidationProfile loaded for this scope
/// - `effective_rules_fingerprint`: optional fingerprint for evidence tracing
pub fn assemble_review_input(
    scope_ref: impl Into<String>,
    target_refs: Vec<String>,
    profile: &ValidationProfile,
    effective_rules_fingerprint: Option<String>,
) -> ReviewRunInput {
    let review_types = map_checks_to_review_types(&profile.checks);

    let severity_threshold = match profile.severity_threshold {
        ProfileSeverity::Block => Some("block".to_string()),
        ProfileSeverity::Warn => Some("warn".to_string()),
        ProfileSeverity::None => None,
    };

    ReviewRunInput {
        scope_ref: scope_ref.into(),
        target_refs,
        branch_id: None,
        review_types,
        task_card_ref: None,
        context_pack_ref: None,
        effective_rules_fingerprint,
        severity_threshold,
    }
}

/// Map `CheckType` entries from a ValidationProfile to `ReviewType` values.
///
/// Unknown / unimplemented checks are silently skipped (V1 scope).
fn map_checks_to_review_types(checks: &[CheckType]) -> Vec<ReviewType> {
    let mut out = Vec::new();
    for check in checks {
        let rt = match check {
            CheckType::WordCountCheck => Some(ReviewType::WordCount),
            CheckType::ContinuityCheck => Some(ReviewType::Continuity),
            CheckType::LogicCheck => Some(ReviewType::Logic),
            CheckType::CharacterVoiceCheck => Some(ReviewType::Character),
            CheckType::StyleTemplateCheck => Some(ReviewType::Style),
            CheckType::TerminologyCheck => Some(ReviewType::Terminology),
            CheckType::ForeshadowCheck => Some(ReviewType::Foreshadow),
            CheckType::ObjectiveCompletionCheck => Some(ReviewType::ObjectiveCompletion),
            // ForbiddenPatternCheck: V1 lightweight scan handled separately; skip here
            CheckType::ForbiddenPatternCheck => None,
        };
        if let Some(rt) = rt {
            if !out.contains(&rt) {
                out.push(rt);
            }
        }
    }
    // If the profile produced no review types, fall back to WordCount so the gate is never empty
    if out.is_empty() {
        out.push(ReviewType::WordCount);
    }
    out
}

/// Returns true if the review result should block mission progress given this profile.
///
/// - `overall_status`: the string from `ReviewReport.overall_status` ("pass"/"warn"/"block")
/// - `strict_warn`: from the profile; if true, "warn" also blocks
pub fn should_block(overall_status: &str, strict_warn: bool) -> bool {
    match overall_status {
        "block" => true,
        "warn" if strict_warn => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing_rules::types::{CheckType, SeverityThreshold, ValidationProfile};

    fn make_profile(checks: Vec<CheckType>, threshold: SeverityThreshold) -> ValidationProfile {
        ValidationProfile {
            schema_version: 1,
            validation_profile_id: "test_profile".to_string(),
            checks,
            severity_threshold: threshold,
            strict_warn: false,
            auto_fix_on_block: false,
        }
    }

    #[test]
    fn maps_word_count_check() {
        let profile = make_profile(vec![CheckType::WordCountCheck], SeverityThreshold::Warn);
        let input = assemble_review_input(
            "chapter:vol1/ch1.json",
            vec!["vol1/ch1.json".to_string()],
            &profile,
            None,
        );
        assert_eq!(input.review_types, vec![ReviewType::WordCount]);
        assert_eq!(input.severity_threshold.as_deref(), Some("warn"));
    }

    #[test]
    fn maps_multiple_checks_deduped() {
        let profile = make_profile(
            vec![
                CheckType::WordCountCheck,
                CheckType::ContinuityCheck,
                CheckType::CharacterVoiceCheck,
                CheckType::WordCountCheck, // duplicate
            ],
            SeverityThreshold::Block,
        );
        let input = assemble_review_input(
            "chapter:vol1/ch1.json",
            vec!["vol1/ch1.json".to_string()],
            &profile,
            None,
        );
        assert_eq!(
            input.review_types,
            vec![
                ReviewType::WordCount,
                ReviewType::Continuity,
                ReviewType::Character
            ]
        );
        assert_eq!(input.severity_threshold.as_deref(), Some("block"));
    }

    #[test]
    fn empty_checks_fallback_to_word_count() {
        let profile = make_profile(vec![], SeverityThreshold::Warn);
        let input = assemble_review_input(
            "chapter:vol1/ch1.json",
            vec!["vol1/ch1.json".to_string()],
            &profile,
            None,
        );
        assert_eq!(input.review_types, vec![ReviewType::WordCount]);
    }

    #[test]
    fn forbidden_pattern_check_skipped() {
        let profile = make_profile(
            vec![CheckType::ForbiddenPatternCheck],
            SeverityThreshold::None,
        );
        let input = assemble_review_input(
            "chapter:vol1/ch1.json",
            vec!["vol1/ch1.json".to_string()],
            &profile,
            None,
        );
        // ForbiddenPatternCheck skipped → fallback WordCount
        assert_eq!(input.review_types, vec![ReviewType::WordCount]);
        assert!(input.severity_threshold.is_none());
    }

    #[test]
    fn should_block_on_block_status() {
        assert!(should_block("block", false));
        assert!(should_block("block", true));
    }

    #[test]
    fn should_block_warn_only_when_strict() {
        assert!(!should_block("warn", false));
        assert!(should_block("warn", true));
    }

    #[test]
    fn should_not_block_on_pass() {
        assert!(!should_block("pass", false));
        assert!(!should_block("pass", true));
    }
}
