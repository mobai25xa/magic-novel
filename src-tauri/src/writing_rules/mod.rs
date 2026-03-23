//! Writing Rules System (Rule-P1 + Rule-P3)
//!
//! Structured writing rule assets: RuleSet, StyleTemplate, ValidationProfile.
//! Active rule resolution: global → volume → chapter merge with conflict detection.
//! Produces `EffectiveRules` for Prompt injection and ReviewGate validation.
//!
//! Rule-P3 additions:
//! - `versioning`: accept/rollback write new version files; diff between versions.
//! - `chapter_binding`: bind rule versions at draft-write time; reconstruct at
//!   review time so history is never polluted by later rule changes.

use std::path::Path;

pub mod chapter_binding;
pub mod loader;
pub mod resolver;
pub mod types;
pub mod versioning;

pub use resolver::resolve_effective_rules;
pub use types::EffectiveRules;

/// Resolve EffectiveRules only when the structured rulesets system is present.
///
/// Returns `None` when there are no accepted structured rulesets on disk.
pub fn resolve_effective_rules_if_available(
    project_path: &Path,
    scope_ref: &str,
) -> Option<EffectiveRules> {
    let rulesets = loader::load_accepted_rulesets(project_path);
    if rulesets.is_empty() {
        return None;
    }
    Some(resolver::resolve_from_rulesets(&rulesets, scope_ref))
}

/// Returns true when there is at least one accepted structured ruleset on disk.
pub fn has_accepted_rulesets(project_path: &Path) -> bool {
    !loader::load_accepted_rulesets(project_path).is_empty()
}
