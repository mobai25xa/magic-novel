//! Mission Layer1 / ContextPack Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::mission::artifacts;
use crate::mission::contextpack_builder::{self, BuildContextPackInput};
use crate::mission::contextpack_staleness::{
    check_contextpack_staleness, ContextPackStalenessStatus,
};
use crate::mission::contextpack_types::{ContextPack, TokenBudget};
use crate::mission::events::MissionEventEmitter;
use crate::mission::layer1_types::{
    ActiveCast, ChapterCard, Layer1ArtifactKind, Layer1Snapshot, RecentFacts, LAYER1_SCHEMA_VERSION,
};
use crate::models::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionLayer1GetInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_layer1_get(input: MissionLayer1GetInput) -> Result<Layer1Snapshot, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::read_layer1_snapshot(project_path, &input.mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionLayer1UpsertInput {
    pub project_path: String,
    pub mission_id: String,
    pub kind: Layer1ArtifactKind,
    pub doc: serde_json::Value,
}

fn infer_layer1_scope_ref(project_path: &std::path::Path, mission_id: &str) -> Option<String> {
    artifacts::read_layer1_chapter_card(project_path, mission_id)
        .ok()
        .flatten()
        .map(|cc| cc.scope_ref)
        .filter(|s| !s.trim().is_empty())
}

#[command]
pub async fn mission_layer1_upsert(
    app_handle: tauri::AppHandle,
    input: MissionLayer1UpsertInput,
) -> Result<(), AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let now = chrono::Utc::now().timestamp_millis();
    let fallback_scope_ref = infer_layer1_scope_ref(project_path, &input.mission_id);
    let kind_str = serde_json::to_string(&input.kind)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    match input.kind {
        Layer1ArtifactKind::ChapterCard => {
            let mut doc: ChapterCard = serde_json::from_value(input.doc)?;
            doc.schema_version = LAYER1_SCHEMA_VERSION;
            doc.updated_at = now;
            if doc.scope_ref.trim().is_empty() {
                return Err(AppError::invalid_argument(
                    "chapter_card.scope_ref cannot be empty",
                ));
            }
            artifacts::write_layer1_chapter_card(project_path, &input.mission_id, &doc)?;
        }
        Layer1ArtifactKind::RecentFacts => {
            let mut doc: RecentFacts = serde_json::from_value(input.doc)?;
            doc.schema_version = LAYER1_SCHEMA_VERSION;
            doc.updated_at = now;
            if doc.scope_ref.trim().is_empty() {
                if let Some(sr) = fallback_scope_ref {
                    doc.scope_ref = sr;
                }
            }
            if doc.scope_ref.trim().is_empty() {
                return Err(AppError::invalid_argument(
                    "recent_facts.scope_ref cannot be empty",
                ));
            }
            artifacts::write_layer1_recent_facts(project_path, &input.mission_id, &doc)?;
        }
        Layer1ArtifactKind::ActiveCast => {
            let mut doc: ActiveCast = serde_json::from_value(input.doc)?;
            doc.schema_version = LAYER1_SCHEMA_VERSION;
            doc.updated_at = now;
            if doc.scope_ref.trim().is_empty() {
                if let Some(sr) = fallback_scope_ref {
                    doc.scope_ref = sr;
                }
            }
            if doc.scope_ref.trim().is_empty() {
                return Err(AppError::invalid_argument(
                    "active_cast.scope_ref cannot be empty",
                ));
            }
            artifacts::write_layer1_active_cast(project_path, &input.mission_id, &doc)?;
        }
        Layer1ArtifactKind::ActiveForeshadowing => {
            let path = artifacts::layer1_active_foreshadowing_path(project_path, &input.mission_id);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::utils::atomic_write::atomic_write_json(&path, &input.doc)?;
        }
        Layer1ArtifactKind::PreviousSummary => {
            let path = artifacts::layer1_previous_summary_path(project_path, &input.mission_id);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::utils::atomic_write::atomic_write_json(&path, &input.doc)?;
        }
        Layer1ArtifactKind::RiskLedger => {
            let path = artifacts::layer1_risk_ledger_path(project_path, &input.mission_id);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::utils::atomic_write::atomic_write_json(&path, &input.doc)?;
        }
    }

    // Best-effort event emission (non-fatal)
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    if let Err(e) = emitter.layer1_updated(&kind_str) {
        tracing::warn!(target: "mission", error = %e, "failed to emit MISSION_LAYER1_UPDATED");
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackGetLatestInput {
    pub project_path: String,
    pub mission_id: String,
}

#[command]
pub async fn mission_contextpack_get_latest(
    input: MissionContextPackGetLatestInput,
) -> Result<Option<ContextPack>, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    artifacts::read_latest_contextpack(project_path, &input.mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackBuildInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub scope_ref: Option<String>,
    #[serde(default)]
    pub token_budget: Option<TokenBudget>,
    #[serde(default)]
    pub active_chapter_path: Option<String>,
    #[serde(default)]
    pub selected_text: Option<String>,
}

#[command]
pub async fn mission_contextpack_build(
    app_handle: tauri::AppHandle,
    input: MissionContextPackBuildInput,
) -> Result<ContextPack, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let build_input = BuildContextPackInput {
        scope_ref: input.scope_ref,
        token_budget: input.token_budget,
        active_chapter_path: input.active_chapter_path,
        selected_text: input.selected_text,
    };
    let cp = contextpack_builder::build_and_persist_contextpack(
        project_path,
        &input.mission_id,
        build_input,
    )?;

    // Best-effort event emission (non-fatal)
    let token_budget = serde_json::to_string(&cp.token_budget)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    if let Err(e) = emitter.contextpack_built(&cp.scope_ref, &token_budget, cp.generated_at) {
        tracing::warn!(target: "mission", error = %e, "failed to emit MISSION_CONTEXTPACK_BUILT");
    }

    Ok(cp)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackStatusInput {
    pub project_path: String,
    pub mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackStatusOutput {
    pub contextpack: Option<ContextPack>,
    pub status: ContextPackStalenessStatus,
}

#[command]
pub async fn mission_contextpack_status(
    input: MissionContextPackStatusInput,
) -> Result<MissionContextPackStatusOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let cp = artifacts::read_latest_contextpack(project_path, &input.mission_id)?;
    let status = check_contextpack_staleness(project_path, &input.mission_id, cp.as_ref())?;
    Ok(MissionContextPackStatusOutput {
        contextpack: cp,
        status,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackRebuildIfStaleInput {
    pub project_path: String,
    pub mission_id: String,
    #[serde(default)]
    pub scope_ref: Option<String>,
    #[serde(default)]
    pub token_budget: Option<TokenBudget>,
    #[serde(default)]
    pub active_chapter_path: Option<String>,
    #[serde(default)]
    pub selected_text: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextPackRebuildIfStaleOutput {
    pub rebuilt: bool,
    #[serde(default)]
    pub stale_reasons: Vec<String>,
    pub contextpack: ContextPack,
}

#[command]
pub async fn mission_contextpack_rebuild_if_stale(
    app_handle: tauri::AppHandle,
    input: MissionContextPackRebuildIfStaleInput,
) -> Result<MissionContextPackRebuildIfStaleOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);
    let existing = artifacts::read_latest_contextpack(project_path, &input.mission_id)?;
    let status = check_contextpack_staleness(project_path, &input.mission_id, existing.as_ref())?;

    let should_rebuild = input.force || !status.present || status.stale;
    if !should_rebuild {
        let Some(cp) = existing else {
            return Err(AppError::internal(
                "contextpack staleness status inconsistent (present=true but no pack)".to_string(),
            ));
        };
        return Ok(MissionContextPackRebuildIfStaleOutput {
            rebuilt: false,
            stale_reasons: Vec::new(),
            contextpack: cp,
        });
    }

    let build_input = BuildContextPackInput {
        scope_ref: input.scope_ref,
        token_budget: input.token_budget,
        active_chapter_path: input.active_chapter_path,
        selected_text: input.selected_text,
    };
    let rebuilt = contextpack_builder::build_and_persist_contextpack(
        project_path,
        &input.mission_id,
        build_input,
    )?;

    // Best-effort event emission (non-fatal)
    let token_budget = serde_json::to_string(&rebuilt.token_budget)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let emitter = MissionEventEmitter::new(app_handle, input.mission_id.clone());
    if let Err(e) =
        emitter.contextpack_built(&rebuilt.scope_ref, &token_budget, rebuilt.generated_at)
    {
        tracing::warn!(target: "mission", error = %e, "failed to emit MISSION_CONTEXTPACK_BUILT");
    }

    Ok(MissionContextPackRebuildIfStaleOutput {
        rebuilt: true,
        stale_reasons: status.reasons,
        contextpack: rebuilt,
    })
}
