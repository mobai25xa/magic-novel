//! Mission system - ContextPack staleness detection (P1)

use std::collections::HashMap;
use std::path::Path;

use crate::models::{AppError, Chapter};
use crate::services::{global_config, read_json};

use super::artifacts;
use super::contextpack_types::{ContextPack, SourceRevision};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ContextPackStalenessStatus {
    pub present: bool,
    pub stale: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub current_source_revisions: Vec<SourceRevision>,
}

/// Compute a stable rules fingerprint for staleness detection.
///
/// Uses `.magic_novel/guidelines.md` (project rules) and `~/.magic/rule.md` (global rules) if present.
pub fn compute_rules_fingerprint(project_path: &Path) -> i64 {
    let guidelines_path = project_path.join(".magic_novel").join("guidelines.md");
    let guidelines = std::fs::read_to_string(&guidelines_path).unwrap_or_default();
    let global = global_config::load_global_rules()
        .map(|r| r.content)
        .unwrap_or_default();
    let combined = format!("{guidelines}\n\n---\n\n{global}");
    u64_to_revision(fnv1a_64(combined.as_bytes()))
}

pub fn compute_current_source_revisions(
    project_path: &Path,
    mission_id: &str,
) -> Result<Vec<SourceRevision>, AppError> {
    let mut out = Vec::new();

    let cc = artifacts::read_layer1_chapter_card(project_path, mission_id)?;
    if let Some(cc) = cc.as_ref() {
        if cc.updated_at > 0 {
            out.push(SourceRevision {
                r#ref: "layer1:chapter_card".to_string(),
                revision: cc.updated_at,
            });
        }
    }

    let rf = artifacts::read_layer1_recent_facts(project_path, mission_id)?;
    if let Some(rf) = rf.as_ref() {
        if rf.updated_at > 0 {
            out.push(SourceRevision {
                r#ref: "layer1:recent_facts".to_string(),
                revision: rf.updated_at,
            });
        }
    }

    let ac = artifacts::read_layer1_active_cast(project_path, mission_id)?;
    if let Some(ac) = ac.as_ref() {
        if ac.updated_at > 0 {
            out.push(SourceRevision {
                r#ref: "layer1:active_cast".to_string(),
                revision: ac.updated_at,
            });
        }
    }

    out.push(SourceRevision {
        r#ref: "rules:fingerprint".to_string(),
        revision: compute_rules_fingerprint(project_path),
    });

    if let Some(locator) = cc
        .and_then(|cc| cc.scope_locator)
        .map(|s| s.trim().to_string())
    {
        if !locator.is_empty() {
            let rel = locator.replace('\\', "/");
            let full = project_path.join("manuscripts").join(rel);
            if let Ok(ch) = read_json::<Chapter>(&full) {
                out.push(SourceRevision {
                    r#ref: format!("chapter:{}", ch.id),
                    revision: ch.updated_at,
                });
            }
        }
    }

    Ok(out)
}

pub fn check_contextpack_staleness(
    project_path: &Path,
    mission_id: &str,
    contextpack: Option<&ContextPack>,
) -> Result<ContextPackStalenessStatus, AppError> {
    let current = compute_current_source_revisions(project_path, mission_id)?;
    let current_map = revisions_to_map(&current);

    let Some(cp) = contextpack else {
        return Ok(ContextPackStalenessStatus {
            present: false,
            stale: true,
            reasons: vec!["missing_contextpack".to_string()],
            current_source_revisions: current,
        });
    };

    let mut reasons = Vec::new();
    let mut stale = false;

    let cp_map = revisions_to_map(&cp.source_revisions);

    // If current sources include revisions that the pack didn't track, treat as stale.
    for (r, cur) in &current_map {
        match cp_map.get(*r) {
            Some(old) => {
                if old != cur {
                    stale = true;
                    reasons.push(format!("revision_changed:{r}:{old}->{cur}"));
                }
            }
            None => {
                stale = true;
                reasons.push(format!("contextpack_missing_revision:{r}"));
            }
        }
    }

    // If pack references sources we can no longer resolve, treat as stale.
    for (r, _) in &cp_map {
        if !current_map.contains_key(*r) {
            stale = true;
            reasons.push(format!("source_missing_or_unresolvable:{r}"));
        }
    }

    // Limit reason verbosity.
    if reasons.len() > 12 {
        reasons.truncate(12);
        reasons.push("...more".to_string());
    }

    Ok(ContextPackStalenessStatus {
        present: true,
        stale,
        reasons,
        current_source_revisions: current,
    })
}

fn revisions_to_map(revs: &[SourceRevision]) -> HashMap<&str, i64> {
    let mut map = HashMap::new();
    for r in revs {
        let key = r.r#ref.trim();
        if key.is_empty() {
            continue;
        }
        map.insert(key, r.revision);
    }
    map
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn u64_to_revision(v: u64) -> i64 {
    (v & 0x7fff_ffff_ffff_ffff) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::contextpack_builder::{
        build_and_persist_contextpack, BuildContextPackInput,
    };
    use crate::mission::layer1_types::{
        ChapterCard, ChapterCardStatus, ChapterWorkflowKind, LAYER1_SCHEMA_VERSION,
    };
    use crate::utils::atomic_write::atomic_write_json;
    use std::fs;
    use std::path::PathBuf;

    fn temp_project_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn staleness_detects_chapter_revision_change() {
        let project = temp_project_dir();
        let mission_id = "mis_stale_1";
        fs::create_dir_all(artifacts::mission_dir(&project, mission_id)).unwrap();

        // Guidelines used for fingerprint.
        fs::create_dir_all(project.join(".magic_novel")).unwrap();
        fs::write(project.join(".magic_novel").join("guidelines.md"), "Rule A").unwrap();

        // Layer1: chapter card
        let cc = ChapterCard {
            schema_version: LAYER1_SCHEMA_VERSION,
            scope_ref: "chapter:ch_1".to_string(),
            scope_locator: Some("vol1/ch1.json".to_string()),
            objective: "Test".to_string(),
            workflow_kind: ChapterWorkflowKind::Chapter,
            hard_constraints: Vec::new(),
            success_criteria: Vec::new(),
            status: ChapterCardStatus::Active,
            updated_at: 10,
        };
        artifacts::write_layer1_chapter_card(&project, mission_id, &cc).unwrap();

        // Chapter file
        let manuscripts = project.join("manuscripts").join("vol1");
        fs::create_dir_all(&manuscripts).unwrap();
        let mut ch = Chapter::new("Ch1".to_string());
        ch.id = "ch_1".to_string();
        ch.updated_at = 100;
        atomic_write_json(&manuscripts.join("ch1.json"), &ch).unwrap();

        // Build first pack
        let cp = build_and_persist_contextpack(
            &project,
            mission_id,
            BuildContextPackInput {
                scope_ref: None,
                token_budget: None,
                active_chapter_path: None,
                selected_text: None,
            },
        )
        .unwrap();
        let st0 = check_contextpack_staleness(&project, mission_id, Some(&cp)).unwrap();
        assert!(!st0.stale, "fresh pack should not be stale");

        // Update chapter revision
        let mut ch2 = ch.clone();
        ch2.updated_at = 101;
        atomic_write_json(&manuscripts.join("ch1.json"), &ch2).unwrap();

        let st1 = check_contextpack_staleness(&project, mission_id, Some(&cp)).unwrap();
        assert!(st1.stale);

        let _ = fs::remove_dir_all(&project);
    }
}
