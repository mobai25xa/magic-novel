//! Macro workflow Tauri commands (M5)
//!
//! C3: mission_macro_create, mission_macro_get_state
//! C4: Feature pipeline generation (per-chapter context + draft)
//! C5: Macro state progression helpers
//! C6: Crash recovery (rebuild state from features)

use serde::{Deserialize, Serialize};
use tauri::command;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::macro_types::*;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::models::AppError;

// ── DTOs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroCreateInput {
    pub project_path: String,
    pub objective: String,
    pub workflow_kind: WorkflowKind,
    pub chapter_targets: Vec<ChapterTarget>,
    #[serde(default)]
    pub strict_review: bool,
    #[serde(default)]
    pub auto_fix_on_block: bool,
    #[serde(default = "default_token_budget")]
    pub token_budget: TokenBudget,
}

fn default_token_budget() -> TokenBudget {
    TokenBudget::Medium
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroCreateOutput {
    pub mission_id: String,
    pub macro_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroGetStateInput {
    pub project_path: String,
    pub mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroGetStateOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<MacroWorkflowConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<MacroWorkflowState>,
}

// ── C4: Feature pipeline generation ────────────────────────────

fn generate_macro_features(targets: &[ChapterTarget]) -> Vec<Feature> {
    let mut features = Vec::new();
    let mut prev_id: Option<String> = None;

    for (i, target) in targets.iter().enumerate() {
        let idx = i + 1;
        let ctx_id = format!("ch{}_context", idx);
        let draft_id = format!("ch{}_draft", idx);

        let ctx_desc = format!(
            "Build ContextPack for chapter {} ({})",
            idx,
            target.display_title.as_deref().unwrap_or(&target.chapter_ref)
        );
        let draft_desc = format!(
            "Draft chapter {} to {}",
            idx, target.write_path
        );

        // context depends on previous chapter's last feature
        let ctx_deps = match &prev_id {
            Some(id) => vec![id.clone()],
            None => Vec::new(),
        };

        features.push(Feature {
            id: ctx_id.clone(),
            status: FeatureStatus::Pending,
            description: ctx_desc,
            skill: "context".to_string(),
            preconditions: Vec::new(),
            depends_on: ctx_deps,
            expected_behavior: vec![
                format!("Generate Layer1 artifacts for {}", target.chapter_ref),
                "Build ContextPack with minimal necessary context".to_string(),
            ],
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        });

        // draft depends on its own context
        features.push(Feature {
            id: draft_id.clone(),
            status: FeatureStatus::Pending,
            description: draft_desc,
            skill: "draft".to_string(),
            preconditions: Vec::new(),
            depends_on: vec![ctx_id],
            expected_behavior: vec![
                format!("Write chapter content to {}", target.write_path),
            ],
            verification_steps: Vec::new(),
            write_paths: vec![target.write_path.clone()],
        });

        prev_id = Some(draft_id);
    }

    features
}

// ── C5: Initial state builder ──────────────────────────────────

fn build_initial_macro_state(
    config: &MacroWorkflowConfig,
) -> MacroWorkflowState {
    let now = chrono::Utc::now().timestamp_millis();
    let chapters = config
        .chapter_targets
        .iter()
        .map(|t| ChapterRunState {
            chapter_ref: t.chapter_ref.clone(),
            write_path: t.write_path.clone(),
            display_title: t.display_title.clone(),
            status: ChapterRunStatus::Pending,
            stage: None,
            latest_contextpack_ref: None,
            latest_review_id: None,
            latest_knowledge_delta_id: None,
            last_handoff_summary: None,
            updated_at: now,
        })
        .collect();

    MacroWorkflowState {
        schema_version: MACRO_SCHEMA_VERSION,
        macro_id: config.macro_id.clone(),
        mission_id: config.mission_id.clone(),
        objective: config.objective.clone(),
        workflow_kind: config.workflow_kind.clone(),
        current_index: -1,
        current_stage: MacroStage::Planning,
        chapters,
        last_transition_at: now,
        last_error: None,
    }
}

// ── C6: Crash recovery ─────────────────────────────────────────

/// Rebuild macro state from features.json when state.json is missing or stale.
fn rebuild_macro_state_from_features(
    project_path: &std::path::Path,
    mission_id: &str,
    config: &MacroWorkflowConfig,
) -> Result<MacroWorkflowState, AppError> {
    let features_doc = artifacts::read_features(project_path, mission_id)?;
    let now = chrono::Utc::now().timestamp_millis();
    let num_chapters = config.chapter_targets.len();

    let mut chapters: Vec<ChapterRunState> = config
        .chapter_targets
        .iter()
        .map(|t| ChapterRunState {
            chapter_ref: t.chapter_ref.clone(),
            write_path: t.write_path.clone(),
            display_title: t.display_title.clone(),
            status: ChapterRunStatus::Pending,
            stage: None,
            latest_contextpack_ref: None,
            latest_review_id: None,
            latest_knowledge_delta_id: None,
            last_handoff_summary: None,
            updated_at: now,
        })
        .collect();

    let mut current_index: i32 = -1;
    let mut current_stage = MacroStage::Planning;

    for (i, _target) in config.chapter_targets.iter().enumerate() {
        let idx = i + 1;
        let ctx_id = format!("ch{}_context", idx);
        let draft_id = format!("ch{}_draft", idx);

        let ctx_status = features_doc
            .features
            .iter()
            .find(|f| f.id == ctx_id)
            .map(|f| &f.status);
        let draft_status = features_doc
            .features
            .iter()
            .find(|f| f.id == draft_id)
            .map(|f| &f.status);

        match (ctx_status, draft_status) {
            (Some(FeatureStatus::Completed), Some(FeatureStatus::Completed)) => {
                chapters[i].status = ChapterRunStatus::Completed;
                chapters[i].stage = Some(MacroStage::Completed);
                current_index = i as i32;
                current_stage = MacroStage::Completed;
            }
            (Some(FeatureStatus::Completed), Some(FeatureStatus::InProgress)) => {
                chapters[i].status = ChapterRunStatus::Running;
                chapters[i].stage = Some(MacroStage::Draft);
                current_index = i as i32;
                current_stage = MacroStage::Draft;
            }
            (Some(FeatureStatus::InProgress), _) => {
                chapters[i].status = ChapterRunStatus::Running;
                chapters[i].stage = Some(MacroStage::Context);
                current_index = i as i32;
                current_stage = MacroStage::Context;
            }
            (Some(FeatureStatus::Failed), _) | (_, Some(FeatureStatus::Failed)) => {
                chapters[i].status = ChapterRunStatus::Failed;
                chapters[i].stage = Some(MacroStage::Failed);
                current_index = i as i32;
                current_stage = MacroStage::Failed;
            }
            (Some(FeatureStatus::Completed), _) => {
                chapters[i].status = ChapterRunStatus::Running;
                chapters[i].stage = Some(MacroStage::Draft);
                current_index = i as i32;
                current_stage = MacroStage::Draft;
            }
            (Some(FeatureStatus::Cancelled), _) | (_, Some(FeatureStatus::Cancelled)) => {
                chapters[i].status = ChapterRunStatus::Pending;
                chapters[i].stage = None;
            }
            _ => {}
        }
    }

    // If all chapters completed, mark integrate
    let all_done = chapters.iter().all(|c| c.status == ChapterRunStatus::Completed);
    if all_done && num_chapters > 0 {
        current_stage = MacroStage::Integrate;
    }

    Ok(MacroWorkflowState {
        schema_version: MACRO_SCHEMA_VERSION,
        macro_id: config.macro_id.clone(),
        mission_id: config.mission_id.clone(),
        objective: config.objective.clone(),
        workflow_kind: config.workflow_kind.clone(),
        current_index,
        current_stage,
        chapters,
        last_transition_at: now,
        last_error: None,
    })
}

// ── C5: State progression helper ───────────────────────────────

/// Update macro state when a feature starts or completes.
/// Called from the supervision layer during mission execution.
#[allow(dead_code)]
pub fn update_macro_state_on_feature_event(
    project_path: &std::path::Path,
    mission_id: &str,
    feature_id: &str,
    new_status: &FeatureStatus,
    emitter: Option<&MissionEventEmitter>,
) -> Result<(), AppError> {
    let mut state = match artifacts::read_macro_state(project_path, mission_id)? {
        Some(s) => s,
        None => return Ok(()), // not a macro mission
    };

    // Parse feature_id pattern: ch{N}_context or ch{N}_draft
    let (chapter_idx, stage) = match parse_macro_feature_id(feature_id) {
        Some(v) => v,
        None => return Ok(()), // not a macro feature
    };

    if chapter_idx >= state.chapters.len() {
        return Ok(());
    }

    let now = chrono::Utc::now().timestamp_millis();

    match new_status {
        FeatureStatus::InProgress => {
            state.chapters[chapter_idx].status = ChapterRunStatus::Running;
            state.chapters[chapter_idx].stage = Some(stage.clone());
            state.current_index = chapter_idx as i32;
            state.current_stage = stage;
        }
        FeatureStatus::Completed => {
            state.chapters[chapter_idx].stage = Some(stage.clone());
            if stage == MacroStage::Draft {
                state.chapters[chapter_idx].status = ChapterRunStatus::Completed;
                state.chapters[chapter_idx].stage = Some(MacroStage::Completed);
                let all_done = state.chapters.iter().all(|c| {
                    c.status == ChapterRunStatus::Completed
                });
                if all_done {
                    state.current_stage = MacroStage::Integrate;
                }
            }
        }
        FeatureStatus::Failed => {
            state.chapters[chapter_idx].status = ChapterRunStatus::Failed;
            state.chapters[chapter_idx].stage = Some(MacroStage::Failed);
            state.current_stage = MacroStage::Failed;
            state.last_error = Some(MacroLastError {
                code: "E_FEATURE_FAILED".into(),
                message: format!("Feature {} failed", feature_id),
                feature_id: Some(feature_id.to_string()),
                worker_id: None,
            });
        }
        FeatureStatus::Cancelled => {
            state.current_stage = MacroStage::Cancelled;
        }
        _ => {}
    }

    state.chapters[chapter_idx].updated_at = now;
    state.last_transition_at = now;

    artifacts::write_macro_state(project_path, mission_id, &state)?;

    // C8: append checkpoint
    let _ = artifacts::append_macro_checkpoint(
        project_path,
        mission_id,
        &serde_json::json!({
            "ts": now,
            "event": "feature_event",
            "feature_id": feature_id,
            "new_status": new_status,
            "chapter_idx": chapter_idx,
            "current_stage": state.current_stage,
            "current_index": state.current_index,
        }),
    );

    if let Some(em) = emitter {
        let _ = em.macro_state_updated(&state);
    }

    Ok(())
}

/// Parse "ch3_context" -> (2, MacroStage::Context), "ch1_draft" -> (0, MacroStage::Draft)
fn parse_macro_feature_id(feature_id: &str) -> Option<(usize, MacroStage)> {
    if !feature_id.starts_with("ch") {
        return None;
    }
    if let Some(rest) = feature_id.strip_prefix("ch") {
        if let Some((num_str, suffix)) = rest.split_once('_') {
            let num: usize = num_str.parse().ok()?;
            if num == 0 {
                return None;
            }
            let stage = match suffix {
                "context" => MacroStage::Context,
                "draft" => MacroStage::Draft,
                _ => return None,
            };
            return Some((num - 1, stage));
        }
    }
    None
}

// ── C3: Tauri commands ─────────────────────────────────────────

#[command]
pub async fn mission_macro_create(
    input: MacroCreateInput,
) -> Result<MacroCreateOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);

    if input.chapter_targets.is_empty() {
        return Err(AppError::invalid_argument(
            "chapter_targets must not be empty",
        ));
    }

    let macro_id = format!("macro_{}", uuid::Uuid::new_v4());

    // C4: generate feature pipeline from chapter targets
    let mut features = generate_macro_features(&input.chapter_targets);
    super::append_integrator_feature_if_missing(&mut features);

    // Create the underlying mission via Orchestrator
    let title = format!("Macro: {}", input.objective);
    let mission_text = format!(
        "Macro workflow ({:?}): {}\nChapters: {}",
        input.workflow_kind,
        input.objective,
        input.chapter_targets.len()
    );
    let mission_id =
        Orchestrator::create_mission(project_path, &title, &mission_text, features)?;

    // Build and write macro config (immutable)
    let config = MacroWorkflowConfig {
        schema_version: MACRO_SCHEMA_VERSION,
        macro_id: macro_id.clone(),
        mission_id: mission_id.clone(),
        workflow_kind: input.workflow_kind,
        objective: input.objective,
        chapter_targets: input.chapter_targets,
        strict_review: input.strict_review,
        auto_fix_on_block: input.auto_fix_on_block,
        token_budget: input.token_budget,
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    artifacts::write_macro_config(project_path, &mission_id, &config)?;

    // Build and write initial macro state
    let state = build_initial_macro_state(&config);
    artifacts::write_macro_state(project_path, &mission_id, &state)?;

    tracing::info!(
        target: "mission",
        mission_id = %mission_id,
        macro_id = %macro_id,
        chapters = %config.chapter_targets.len(),
        "macro workflow created"
    );

    Ok(MacroCreateOutput {
        mission_id,
        macro_id,
    })
}

#[command]
pub async fn mission_macro_get_state(
    input: MacroGetStateInput,
) -> Result<MacroGetStateOutput, AppError> {
    let project_path = std::path::Path::new(&input.project_path);

    let config = artifacts::read_macro_config(project_path, &input.mission_id)?;
    let mut state = artifacts::read_macro_state(project_path, &input.mission_id)?;

    // C6: If config exists but state is missing, attempt recovery
    if config.is_some() && state.is_none() {
        if let Some(ref cfg) = config {
            match rebuild_macro_state_from_features(project_path, &input.mission_id, cfg) {
                Ok(rebuilt) => {
                    let _ = artifacts::write_macro_state(
                        project_path,
                        &input.mission_id,
                        &rebuilt,
                    );
                    state = Some(rebuilt);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "mission",
                        mission_id = %input.mission_id,
                        error = %e,
                        "failed to rebuild macro state from features"
                    );
                }
            }
        }
    }

    Ok(MacroGetStateOutput { config, state })
}

// ── C5: Lifecycle hooks ────────────────────────────────────────

/// Update macro current_stage on pause/cancel/resume.
/// Fire-and-forget: errors are logged but not propagated.
pub fn update_macro_stage_on_lifecycle(
    project_path: &std::path::Path,
    mission_id: &str,
    new_stage: MacroStage,
    emitter: Option<&MissionEventEmitter>,
) {
    let mut state = match artifacts::read_macro_state(project_path, mission_id) {
        Ok(Some(s)) => s,
        _ => return, // not a macro mission
    };

    let now = chrono::Utc::now().timestamp_millis();
    state.current_stage = new_stage;
    state.last_transition_at = now;

    if let Err(e) = artifacts::write_macro_state(project_path, mission_id, &state) {
        tracing::warn!(
            target: "mission",
            mission_id = %mission_id,
            error = %e,
            "failed to update macro state on lifecycle event"
        );
        return;
    }

    // C8: append checkpoint
    let _ = artifacts::append_macro_checkpoint(
        project_path,
        mission_id,
        &serde_json::json!({
            "ts": now,
            "event": "lifecycle",
            "new_stage": state.current_stage,
        }),
    );

    if let Some(em) = emitter {
        let _ = em.macro_state_updated(&state);
    }
}

// ── C6: Resume recovery ────────────────────────────────────────

/// On resume, ensure macro state is consistent with features.json.
/// If state.json is missing or stale, rebuild from features.
pub fn try_recover_macro_state_on_resume(
    project_path: &std::path::Path,
    mission_id: &str,
    emitter: Option<&MissionEventEmitter>,
) {
    let config = match artifacts::read_macro_config(project_path, mission_id) {
        Ok(Some(c)) => c,
        _ => return, // not a macro mission
    };

    let needs_rebuild = match artifacts::read_macro_state(project_path, mission_id) {
        Ok(None) => true,
        Ok(Some(s)) => {
            // If state says blocked/cancelled but we're resuming, rebuild to get accurate picture
            matches!(s.current_stage, MacroStage::Blocked | MacroStage::Cancelled)
        }
        Err(_) => true,
    };

    if !needs_rebuild {
        return;
    }

    match rebuild_macro_state_from_features(project_path, mission_id, &config) {
        Ok(rebuilt) => {
            if let Err(e) = artifacts::write_macro_state(project_path, mission_id, &rebuilt) {
                tracing::warn!(
                    target: "mission",
                    mission_id = %mission_id,
                    error = %e,
                    "failed to write recovered macro state on resume"
                );
                return;
            }
            tracing::info!(
                target: "mission",
                mission_id = %mission_id,
                current_index = rebuilt.current_index,
                "macro state recovered on resume"
            );
            if let Some(em) = emitter {
                let _ = em.macro_state_updated(&rebuilt);
            }
        }
        Err(e) => {
            tracing::warn!(
                target: "mission",
                mission_id = %mission_id,
                error = %e,
                "failed to rebuild macro state on resume"
            );
        }
    }
}

// ── C9: Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_features_produces_correct_pipeline() {
        let targets = vec![
            ChapterTarget {
                chapter_ref: "vol1/ch1".into(),
                write_path: "chapters/ch1.md".into(),
                display_title: Some("Chapter 1".into()),
            },
            ChapterTarget {
                chapter_ref: "vol1/ch2".into(),
                write_path: "chapters/ch2.md".into(),
                display_title: None,
            },
            ChapterTarget {
                chapter_ref: "vol1/ch3".into(),
                write_path: "chapters/ch3.md".into(),
                display_title: None,
            },
        ];

        let features = generate_macro_features(&targets);
        assert_eq!(features.len(), 6); // 3 chapters * 2 features

        assert_eq!(features[0].id, "ch1_context");
        assert!(features[0].depends_on.is_empty());

        assert_eq!(features[1].id, "ch1_draft");
        assert_eq!(features[1].depends_on, vec!["ch1_context"]);
        assert_eq!(features[1].write_paths, vec!["chapters/ch1.md"]);

        // ch2_context depends on ch1_draft (sequential chain)
        assert_eq!(features[2].id, "ch2_context");
        assert_eq!(features[2].depends_on, vec!["ch1_draft"]);

        assert_eq!(features[5].id, "ch3_draft");
        assert_eq!(features[5].depends_on, vec!["ch3_context"]);
    }

    #[test]
    fn parse_macro_feature_id_valid() {
        assert_eq!(
            parse_macro_feature_id("ch1_context"),
            Some((0, MacroStage::Context))
        );
        assert_eq!(
            parse_macro_feature_id("ch3_draft"),
            Some((2, MacroStage::Draft))
        );
        assert_eq!(
            parse_macro_feature_id("ch10_context"),
            Some((9, MacroStage::Context))
        );
    }

    #[test]
    fn parse_macro_feature_id_invalid() {
        assert_eq!(parse_macro_feature_id("integrator"), None);
        assert_eq!(parse_macro_feature_id("ch0_context"), None);
        assert_eq!(parse_macro_feature_id("ch1_review"), None);
        assert_eq!(parse_macro_feature_id("chapter1_draft"), None);
    }

    fn make_test_config(mission_id: &str, num_chapters: usize) -> MacroWorkflowConfig {
        let targets: Vec<ChapterTarget> = (1..=num_chapters)
            .map(|i| ChapterTarget {
                chapter_ref: format!("vol1/ch{}", i),
                write_path: format!("chapters/ch{}.md", i),
                display_title: None,
            })
            .collect();
        MacroWorkflowConfig {
            schema_version: MACRO_SCHEMA_VERSION,
            macro_id: "macro_test".into(),
            mission_id: mission_id.into(),
            workflow_kind: WorkflowKind::Book,
            objective: "test".into(),
            chapter_targets: targets,
            strict_review: false,
            auto_fix_on_block: false,
            token_budget: TokenBudget::Medium,
            created_at: 0,
        }
    }

    fn write_test_features(
        project_path: &std::path::Path,
        mission_id: &str,
        features: Vec<Feature>,
    ) {
        let dir = project_path
            .join("magic_novel")
            .join("missions")
            .join(mission_id);
        std::fs::create_dir_all(&dir).unwrap();
        let doc = FeaturesDoc {
            schema_version: 1,
            mission_id: mission_id.into(),
            title: "test".into(),
            features,
        };
        std::fs::write(
            dir.join("features.json"),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();
    }

    fn make_feature(id: &str, status: FeatureStatus, write_paths: Vec<String>) -> Feature {
        Feature {
            id: id.into(),
            status,
            description: id.into(),
            skill: if id.contains("context") { "context" } else { "draft" }.into(),
            preconditions: vec![],
            depends_on: vec![],
            expected_behavior: vec![],
            verification_steps: vec![],
            write_paths,
        }
    }

    #[test]
    fn rebuild_all_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_test_config("mis1", 2);

        write_test_features(tmp.path(), "mis1", vec![
            make_feature("ch1_context", FeatureStatus::Pending, vec![]),
            make_feature("ch1_draft", FeatureStatus::Pending, vec!["chapters/ch1.md".into()]),
            make_feature("ch2_context", FeatureStatus::Pending, vec![]),
            make_feature("ch2_draft", FeatureStatus::Pending, vec!["chapters/ch2.md".into()]),
        ]);

        let state = rebuild_macro_state_from_features(tmp.path(), "mis1", &config).unwrap();
        assert_eq!(state.current_index, -1);
        assert_eq!(state.current_stage, MacroStage::Planning);
        assert_eq!(state.chapters[0].status, ChapterRunStatus::Pending);
        assert_eq!(state.chapters[1].status, ChapterRunStatus::Pending);
    }

    #[test]
    fn rebuild_partial_completion() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_test_config("mis2", 3);

        write_test_features(tmp.path(), "mis2", vec![
            make_feature("ch1_context", FeatureStatus::Completed, vec![]),
            make_feature("ch1_draft", FeatureStatus::Completed, vec!["chapters/ch1.md".into()]),
            make_feature("ch2_context", FeatureStatus::Completed, vec![]),
            make_feature("ch2_draft", FeatureStatus::InProgress, vec!["chapters/ch2.md".into()]),
            make_feature("ch3_context", FeatureStatus::Pending, vec![]),
            make_feature("ch3_draft", FeatureStatus::Pending, vec!["chapters/ch3.md".into()]),
        ]);

        let state = rebuild_macro_state_from_features(tmp.path(), "mis2", &config).unwrap();
        // ch1 fully done
        assert_eq!(state.chapters[0].status, ChapterRunStatus::Completed);
        assert_eq!(state.chapters[0].stage, Some(MacroStage::Completed));
        // ch2 draft in progress
        assert_eq!(state.chapters[1].status, ChapterRunStatus::Running);
        assert_eq!(state.chapters[1].stage, Some(MacroStage::Draft));
        // ch3 still pending
        assert_eq!(state.chapters[2].status, ChapterRunStatus::Pending);
        // current_index should point to ch2
        assert_eq!(state.current_index, 1);
        assert_eq!(state.current_stage, MacroStage::Draft);
    }

    #[test]
    fn rebuild_all_completed_triggers_integrate() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_test_config("mis3", 2);

        write_test_features(tmp.path(), "mis3", vec![
            make_feature("ch1_context", FeatureStatus::Completed, vec![]),
            make_feature("ch1_draft", FeatureStatus::Completed, vec!["chapters/ch1.md".into()]),
            make_feature("ch2_context", FeatureStatus::Completed, vec![]),
            make_feature("ch2_draft", FeatureStatus::Completed, vec!["chapters/ch2.md".into()]),
        ]);

        let state = rebuild_macro_state_from_features(tmp.path(), "mis3", &config).unwrap();
        assert_eq!(state.chapters[0].status, ChapterRunStatus::Completed);
        assert_eq!(state.chapters[1].status, ChapterRunStatus::Completed);
        assert_eq!(state.current_stage, MacroStage::Integrate);
    }

    #[test]
    fn rebuild_with_failed_feature() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_test_config("mis4", 2);

        write_test_features(tmp.path(), "mis4", vec![
            make_feature("ch1_context", FeatureStatus::Completed, vec![]),
            make_feature("ch1_draft", FeatureStatus::Failed, vec!["chapters/ch1.md".into()]),
            make_feature("ch2_context", FeatureStatus::Pending, vec![]),
            make_feature("ch2_draft", FeatureStatus::Pending, vec!["chapters/ch2.md".into()]),
        ]);

        let state = rebuild_macro_state_from_features(tmp.path(), "mis4", &config).unwrap();
        assert_eq!(state.chapters[0].status, ChapterRunStatus::Failed);
        assert_eq!(state.chapters[0].stage, Some(MacroStage::Failed));
        assert_eq!(state.current_stage, MacroStage::Failed);
    }
}