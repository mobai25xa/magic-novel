//! DevC: Gate Integration — Rule-P2 + Prompt-P4 wiring.
//!
//! Provides:
//! - `profile_assembler`: ValidationProfile → ReviewRunInput mapping
//! - `rules_summary`: EffectiveRules → compact reminder text
//! - `canon_version`: `.magic_novel/_meta/canon_version.json` read/bump

use std::path::Path;

pub mod canon_version;
pub mod profile_assembler;
pub mod rules_summary;

pub use canon_version::{bump_canon_version, read_canon_version, CanonVersion};
pub use profile_assembler::{assemble_review_input, should_block};
pub use rules_summary::render_active_rules_summary;

/// DevC: Best-effort blocker signal for reminders.
///
/// Returns true when *any* mission under this project has a pending review decision.
/// This is intentionally conservative for UI reminders (we don't assume the caller knows a mission_id).
pub fn has_pending_review_blocker(project_path: &Path) -> bool {
    let root = crate::mission::artifacts::missions_root(project_path);
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let mission_id_os = entry.file_name();
        let Some(mission_id) = mission_id_os.to_str() else {
            continue;
        };
        if crate::mission::artifacts::pending_review_decision_path(project_path, mission_id)
            .exists()
        {
            return true;
        }
    }

    false
}
