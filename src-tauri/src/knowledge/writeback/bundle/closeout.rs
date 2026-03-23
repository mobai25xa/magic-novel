use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::knowledge::types::{KnowledgeAcceptPolicy, KnowledgeOp, KnowledgeProposalBundle, KnowledgeProposalItem};
use crate::mission::artifacts;
use crate::models::{AppError, Chapter};

use super::super::branch::resolve_active_branch_id;
use super::super::path::{infer_chapter_locator_from_write_paths, normalize_path, slugify_locator};
use super::super::storage::{read_stored_object, stored_object_path};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Layer1ActiveForeshadowing {
    #[serde(default)]
    pub r#ref: String,
    #[serde(default)]
    pub scope_ref: String,
    #[serde(default)]
    pub items: Vec<Layer1ActiveForeshadowingItem>,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Layer1ActiveForeshadowingItem {
    #[serde(default)]
    pub foreshadow_ref: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
}

fn split_current_state(summary: &str) -> Vec<String> {
    let summary = summary.trim();
    if summary.is_empty() {
        return Vec::new();
    }

    let normalized = summary
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('；', ";");

    let mut candidates: Vec<String> = Vec::new();
    if normalized.contains('\n') {
        candidates.extend(normalized.lines().map(|l| l.to_string()));
    } else if normalized.contains(';') {
        candidates.extend(normalized.split(';').map(|s| s.to_string()));
    } else {
        candidates.push(normalized);
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for c in candidates {
        let mut t = c.trim();
        if t.is_empty() {
            continue;
        }

        t = t.trim_start_matches(['-', '*', '•', '·', '—']).trim();
        if t.is_empty() {
            continue;
        }

        let t = t.to_string();
        if seen.insert(t.clone()) {
            out.push(t);
        }
        if out.len() >= 12 {
            break;
        }
    }

    out
}

fn character_state_target_ref(character_ref: &str) -> Option<String> {
    let character_ref = character_ref.trim();
    if character_ref.is_empty() {
        return None;
    }
    let slug = slugify_locator(character_ref);
    if slug.is_empty() {
        return None;
    }
    Some(format!("characters/{slug}.state.json"))
}

fn foreshadow_target_ref(foreshadow_ref: &str) -> Option<String> {
    let foreshadow_ref = foreshadow_ref.trim();
    if foreshadow_ref.is_empty() {
        return None;
    }
    let slug = slugify_locator(foreshadow_ref);
    if slug.is_empty() {
        return None;
    }
    Some(format!("foreshadow/{slug}.json"))
}

pub(super) fn generate_proposal_bundle_after_closeout(
    project_path: &Path,
    mission_id: &str,
    scope_ref: String,
    write_paths: Vec<String>,
    source_session_id: String,
    source_review_id: Option<String>,
) -> Result<KnowledgeProposalBundle, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let bundle_id = format!("kbundle_{}", uuid::Uuid::new_v4());

    let layer1_cc = artifacts::read_layer1_chapter_card(project_path, mission_id)
        .ok()
        .flatten();
    let locator = infer_chapter_locator_from_write_paths(&write_paths)
        .or_else(|| layer1_cc.as_ref().and_then(|cc| cc.scope_locator.clone()))
        .map(|s| normalize_path(&s));

    let chapter_locator = locator
        .clone()
        .unwrap_or_else(|| format!("unknown/{bundle_id}.json"));
    let chapter_slug = slugify_locator(&chapter_locator);

    // ── Chapter summary proposal ───────────────────────────────
    let chapter_target_ref = format!("chapter_summaries/{chapter_locator}");
    let chapter_obj_path = stored_object_path(project_path, &chapter_target_ref);
    let existing = read_stored_object(&chapter_obj_path)?;

    let (chapter_op, chapter_target_revision) = match existing {
        Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
        None => (KnowledgeOp::Create, None),
    };

    let chapter_summary_fields = load_chapter_summary_fields(project_path, &chapter_locator)
        .unwrap_or_else(|| {
            json!({
                "chapter_locator": chapter_locator,
                "summary": "",
                "key_events": [],
                "state_changes": []
            })
        });

    let mut proposal_items: Vec<KnowledgeProposalItem> = vec![KnowledgeProposalItem {
        item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
        kind: "chapter_summary".to_string(),
        op: chapter_op,
        target_ref: Some(chapter_target_ref),
        target_revision: chapter_target_revision,
        fields: chapter_summary_fields,
        evidence_refs: write_paths
            .iter()
            .map(|p| normalize_path(p))
            .filter(|p| !p.is_empty())
            .take(8)
            .collect(),
        source_refs: vec![format!("scope_ref:{scope_ref}")],
        change_reason: "chapter closeout: update chapter summary".to_string(),
        accept_policy: KnowledgeAcceptPolicy::AutoIfPass,
    }];

    // ── Recent facts (from Layer1) ─────────────────────────────
    if let Ok(Some(rf)) = artifacts::read_layer1_recent_facts(project_path, mission_id) {
        for (idx, fact) in rf.facts.iter().enumerate() {
            let summary = fact.summary.trim();
            if summary.is_empty() {
                continue;
            }
            let target_ref = format!("recent_facts/{chapter_slug}/fact_{}.json", idx + 1);
            let obj_path = stored_object_path(project_path, &target_ref);
            let existing = read_stored_object(&obj_path)?;
            let (op, target_revision) = match existing {
                Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
                None => (KnowledgeOp::Create, None),
            };

            let confidence = match fact.confidence {
                crate::mission::layer1_types::FactConfidence::Accepted => "accepted",
                crate::mission::layer1_types::FactConfidence::Proposed => "proposed",
            };

            let mut evidence = Vec::new();
            if !fact.source_ref.trim().is_empty() {
                evidence.push(fact.source_ref.trim().to_string());
            }
            if evidence.is_empty() {
                evidence.extend(
                    write_paths
                        .iter()
                        .map(|p| normalize_path(p))
                        .filter(|p| !p.is_empty())
                        .take(4),
                );
            }
            if evidence.is_empty() {
                evidence.push(format!("scope_ref:{scope_ref}"));
            }

            proposal_items.push(KnowledgeProposalItem {
                item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
                kind: "recent_fact".to_string(),
                op,
                target_ref: Some(target_ref),
                target_revision,
                fields: json!({
                    "scope_ref": rf.scope_ref,
                    "summary": summary,
                    "source_ref": fact.source_ref,
                    "fact_scope": "chapter",
                    "confidence": confidence,
                    "subject_refs": []
                }),
                evidence_refs: evidence.clone(),
                source_refs: vec![format!("scope_ref:{scope_ref}")],
                change_reason: "chapter closeout: recent fact".to_string(),
                accept_policy: KnowledgeAcceptPolicy::AutoIfPass,
            });
        }
    }

    // ── Active cast (from Layer1) ─────────────────────────────
    if let Ok(Some(ac)) = artifacts::read_layer1_active_cast(project_path, mission_id) {
        for entry in ac.cast.iter().take(20) {
            let char_ref = entry.character_ref.trim();
            if char_ref.is_empty() {
                continue;
            }

            let current_state = split_current_state(&entry.current_state_summary);
            if current_state.is_empty() {
                continue;
            }

            let Some(target_ref) = character_state_target_ref(char_ref) else {
                continue;
            };
            let obj_path = stored_object_path(project_path, &target_ref);
            let existing = read_stored_object(&obj_path)?;
            let (op, target_revision) = match existing {
                Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
                None => (KnowledgeOp::Create, None),
            };

            let mut evidence = Vec::new();
            evidence.push("layer1:active_cast".to_string());
            evidence.push(format!("character:{char_ref}"));
            evidence.extend(
                write_paths
                    .iter()
                    .map(|p| normalize_path(p))
                    .filter(|p| !p.is_empty())
                    .take(2),
            );
            if evidence.is_empty() {
                evidence.push(format!("scope_ref:{scope_ref}"));
            }

            proposal_items.push(KnowledgeProposalItem {
                item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
                kind: "character".to_string(),
                op,
                target_ref: Some(target_ref),
                target_revision,
                fields: json!({
                    "character_ref": char_ref,
                    "current_state": current_state,
                }),
                evidence_refs: evidence,
                source_refs: vec![
                    format!("scope_ref:{scope_ref}"),
                    "layer1:active_cast".to_string(),
                ],
                change_reason: format!(
                    "chapter closeout: update character current_state ({})",
                    char_ref
                ),
                // Spec: character updates require explicit decision (no auto accept).
                accept_policy: KnowledgeAcceptPolicy::Manual,
            });
        }
    }

    // ── Active foreshadowing (from Layer1) ─────────────────────
    if let Ok(Some(raw_af)) = artifacts::read_layer1_active_foreshadowing(project_path, mission_id)
    {
        match serde_json::from_value::<Layer1ActiveForeshadowing>(raw_af) {
            Ok(af) => {
                for item in af.items.iter().take(50) {
                    let foreshadow_ref = item.foreshadow_ref.trim();
                    let status_label = item.status.trim();
                    if foreshadow_ref.is_empty() || status_label.is_empty() {
                        continue;
                    }

                    let Some(target_ref) = foreshadow_target_ref(foreshadow_ref) else {
                        continue;
                    };
                    let obj_path = stored_object_path(project_path, &target_ref);
                    let existing = read_stored_object(&obj_path)?;
                    let (op, target_revision) = match existing {
                        Some(obj) => (KnowledgeOp::Update, Some(obj.revision)),
                        None => (KnowledgeOp::Create, None),
                    };

                    let mut evidence = Vec::new();
                    if let Some(er) = item
                        .evidence_ref
                        .as_ref()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    {
                        evidence.push(er.to_string());
                    }
                    evidence.push("layer1:active_foreshadowing".to_string());
                    evidence.extend(
                        write_paths
                            .iter()
                            .map(|p| normalize_path(p))
                            .filter(|p| !p.is_empty())
                            .take(2),
                    );
                    if evidence.is_empty() {
                        evidence.push(format!("scope_ref:{scope_ref}"));
                    }

                    let notes = item
                        .required_action
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    proposal_items.push(KnowledgeProposalItem {
                        item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
                        kind: "foreshadow".to_string(),
                        op,
                        target_ref: Some(target_ref),
                        target_revision,
                        fields: json!({
                            "seed_ref": foreshadow_ref,
                            "status_label": status_label,
                            "current_notes": notes,
                        }),
                        evidence_refs: evidence,
                        source_refs: vec![
                            format!("scope_ref:{scope_ref}"),
                            "layer1:active_foreshadowing".to_string(),
                        ],
                        change_reason: format!(
                            "chapter closeout: update foreshadow status ({})",
                            foreshadow_ref
                        ),
                        accept_policy: KnowledgeAcceptPolicy::AutoIfPass,
                    });
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "knowledge",
                    error = %e,
                    "active_foreshadowing parse error; skipping foreshadow proposals"
                );
            }
        }
    }

    // Ensure every item has at least one source_ref.
    for it in &mut proposal_items {
        if it.source_refs.is_empty() {
            it.source_refs.push(format!("scope_ref:{scope_ref}"));
        }
        if !it.fields.is_object() {
            it.fields = json!({});
        }
    }

    Ok(KnowledgeProposalBundle {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id,
        scope_ref,
        branch_id: Some(resolve_active_branch_id(project_path)),
        source_session_id,
        source_review_id,
        generated_at: now,
        proposal_items,
    })
}

fn load_chapter_summary_fields(project_path: &Path, chapter_locator: &str) -> Option<serde_json::Value> {
    let p = project_path.join("manuscripts").join(chapter_locator);
    let ch: Chapter = crate::services::read_json(&p).ok()?;
    let summary = ch.summary.unwrap_or_default();
    Some(json!({
        "chapter_locator": chapter_locator,
        "chapter_id": ch.id,
        "chapter_title": ch.title,
        "summary": summary,
        "key_events": [],
        "state_changes": []
    }))
}

