use std::path::{Path, PathBuf};

use tauri::command;

use crate::application::command_usecases::planning_bundle::load_planning_manifest;
use crate::models::{
    AppError, ApprovalState, MaterializationState, PlanningDocEntry, PlanningDocId,
    PlanningManifest, PLANNING_MANIFEST_REL_PATH,
};
use crate::services::{read_file, write_json};

const SOURCE_FILESYSTEM_SCAN: &str = "filesystem_scan";
const SOURCE_LEGACY_BOOTSTRAP: &str = "legacy_bootstrap_mapping";
const SOURCE_USER_EDIT: &str = "user_edit";
const SOURCE_USER_CONFIRMED: &str = "user_confirmed";

const LEGACY_STORY_BRIEF_PATHS: &[&str] = &[
    ".magic_novel/system/creation_brief.md",
    ".magic_novel/system/project_profile.md",
];
const LEGACY_CHARACTER_CARD_PATHS: &[&str] = &[
    ".magic_novel/characters/protagonist.md",
    ".magic_novel/characters/counterpart.md",
];
const LEGACY_CHAPTER_PLANNING_PATHS: &[&str] = &[".magic_novel/planning/chapter_backlog.md"];

#[derive(Debug, Clone)]
struct ScannedDocState {
    materialization_state: MaterializationState,
    last_source: String,
    updated_at: i64,
}

#[command]
pub async fn get_planning_manifest(project_path: String) -> Result<PlanningManifest, AppError> {
    refresh_planning_manifest_impl(Path::new(&project_path))
}

#[command]
pub async fn refresh_planning_manifest(project_path: String) -> Result<PlanningManifest, AppError> {
    refresh_planning_manifest_impl(Path::new(&project_path))
}

#[command]
pub async fn update_planning_document_approval_state(
    project_path: String,
    doc_id: String,
    approval_state: ApprovalState,
) -> Result<PlanningManifest, AppError> {
    let project_path = PathBuf::from(project_path);
    let mut manifest = refresh_planning_manifest_impl(&project_path)?;
    let doc = manifest
        .docs
        .iter_mut()
        .find(|entry| entry.id == doc_id)
        .ok_or_else(|| AppError::not_found(format!("planning doc not found: {doc_id}")))?;

    doc.approval_state = approval_state;
    doc.last_source = if approval_state == ApprovalState::Accepted {
        SOURCE_USER_CONFIRMED.to_string()
    } else {
        SOURCE_USER_EDIT.to_string()
    };
    doc.updated_at = now_millis();

    manifest.refresh_derived_fields();
    write_manifest(&project_path, &manifest)?;
    Ok(manifest)
}

pub(crate) fn mark_planning_doc_saved(
    project_path: &Path,
    virtual_path: &str,
) -> Result<(), AppError> {
    let mut manifest = match load_planning_manifest(project_path)? {
        Some(manifest) => manifest,
        None => return Ok(()),
    };

    if let Some(doc) = manifest
        .docs
        .iter_mut()
        .find(|entry| entry.path == virtual_path)
    {
        doc.approval_state = doc.approval_state.max(ApprovalState::UserRefined);
        doc.last_source = SOURCE_USER_EDIT.to_string();
        doc.updated_at = now_millis();
        manifest.refresh_derived_fields();
        write_manifest(project_path, &manifest)?;
    }

    Ok(())
}

pub(crate) fn refresh_planning_manifest_impl(
    project_path: &Path,
) -> Result<PlanningManifest, AppError> {
    let existing = load_planning_manifest(project_path)?;
    let optional_paths = existing
        .as_ref()
        .map(|manifest| manifest.optional_outputs.clone())
        .unwrap_or_default();
    let generation_source = existing
        .as_ref()
        .and_then(|manifest| manifest.generation_source.clone());
    let generation_provider = existing
        .as_ref()
        .and_then(|manifest| manifest.generation_provider.clone());
    let generation_model = existing
        .as_ref()
        .and_then(|manifest| manifest.generation_model.clone());

    let mut docs = Vec::new();
    for doc_id in PlanningDocId::core_docs() {
        docs.push(scan_doc_entry(project_path, *doc_id, existing.as_ref()));
    }

    let volume_plan_path = project_path.join(PlanningDocId::VolumePlan.relative_path());
    let volume_plan_present = volume_plan_path.exists();
    if volume_plan_present {
        docs.push(scan_doc_entry(
            project_path,
            PlanningDocId::VolumePlan,
            existing.as_ref(),
        ));
    }

    let optional_outputs = if volume_plan_present {
        let mut outputs = optional_paths;
        if !outputs
            .iter()
            .any(|item| item == PlanningDocId::VolumePlan.as_str())
        {
            outputs.push(PlanningDocId::VolumePlan.as_str().to_string());
        }
        outputs
    } else {
        optional_paths
            .into_iter()
            .filter(|item| item != PlanningDocId::VolumePlan.as_str())
            .collect()
    };

    let mut manifest = PlanningManifest::new(
        docs,
        optional_outputs,
        generation_source,
        generation_provider,
        generation_model,
        now_millis(),
    );
    manifest.refresh_derived_fields();
    write_manifest(project_path, &manifest)?;
    Ok(manifest)
}

fn scan_doc_entry(
    project_path: &Path,
    doc_id: PlanningDocId,
    existing: Option<&PlanningManifest>,
) -> PlanningDocEntry {
    let existing_entry = existing.and_then(|manifest| manifest.doc(doc_id));
    let scanned = scan_doc_state(project_path, doc_id);
    PlanningDocEntry {
        id: doc_id.as_str().to_string(),
        path: doc_id.relative_path().to_string(),
        required_for_create: doc_id.required_for_create(),
        required_for_write: doc_id.required_for_write(),
        materialization_state: scanned.materialization_state,
        approval_state: existing_entry
            .map(|entry| entry.approval_state)
            .unwrap_or(ApprovalState::AiDraft),
        last_source: existing_entry
            .map(|entry| entry.last_source.clone())
            .unwrap_or(scanned.last_source),
        updated_at: existing_entry
            .map(|entry| entry.updated_at)
            .unwrap_or(scanned.updated_at),
    }
}

fn scan_doc_state(project_path: &Path, doc_id: PlanningDocId) -> ScannedDocState {
    if let Some(scanned) = scan_first_meaningful(
        project_path,
        &[doc_id.relative_path()],
        SOURCE_FILESYSTEM_SCAN,
    ) {
        return scanned;
    }

    let legacy_paths = match doc_id {
        PlanningDocId::StoryBrief => LEGACY_STORY_BRIEF_PATHS,
        PlanningDocId::CharacterCards => LEGACY_CHARACTER_CARD_PATHS,
        PlanningDocId::ChapterPlanning => LEGACY_CHAPTER_PLANNING_PATHS,
        _ => &[],
    };
    if let Some(scanned) =
        scan_first_meaningful(project_path, legacy_paths, SOURCE_LEGACY_BOOTSTRAP)
    {
        return scanned;
    }

    ScannedDocState {
        materialization_state: MaterializationState::Failed,
        last_source: SOURCE_FILESYSTEM_SCAN.to_string(),
        updated_at: now_millis(),
    }
}

fn scan_first_meaningful(
    project_path: &Path,
    candidate_paths: &[&str],
    source: &str,
) -> Option<ScannedDocState> {
    let mut latest_updated_at: Option<i64> = None;
    let mut has_meaningful_content = false;

    for candidate in candidate_paths {
        let full_path = project_path.join(candidate);
        let content = read_file(&full_path).unwrap_or_default();
        latest_updated_at = latest_updated_at.max(file_modified_at(&full_path));

        if has_meaningful_planning_content(&content) {
            has_meaningful_content = true;
        }
    }

    if !has_meaningful_content {
        return None;
    }

    Some(ScannedDocState {
        materialization_state: MaterializationState::Ready,
        last_source: source.to_string(),
        updated_at: latest_updated_at.unwrap_or_else(now_millis),
    })
}

fn has_meaningful_planning_content(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lowered = trimmed.to_ascii_lowercase();
    let placeholder_tokens = [
        "todo",
        "tbd",
        "fill in",
        "placeholder",
        "pending",
        "待补充",
        "待生成",
        "暂无",
    ];

    if placeholder_tokens
        .iter()
        .any(|token| lowered.contains(token))
    {
        return false;
    }

    trimmed.chars().filter(|ch| ch.is_alphanumeric()).count() >= 12
}

fn write_manifest(project_path: &Path, manifest: &PlanningManifest) -> Result<(), AppError> {
    write_json(&project_path.join(PLANNING_MANIFEST_REL_PATH), manifest)
}

fn file_modified_at(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
}

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::application::command_usecases::inspiration::{
        ConsensusField, ConsensusFieldId, ConsensusValue, CreateProjectHandoffDraft,
        InspirationConsensusState,
    };
    use crate::application::command_usecases::knowledge_docs::save_knowledge_document;
    use crate::application::command_usecases::planning_bundle::{
        build_deterministic_planning_bundle, persist_planning_bundle, PlanningGenerationMetadata,
    };
    use crate::models::{ProjectBootstrapState, ProjectMetadata, PROJECT_SCHEMA_VERSION};

    fn field(field_id: ConsensusFieldId, value: ConsensusValue) -> ConsensusField {
        ConsensusField {
            field_id,
            draft_value: Some(value),
            confirmed_value: None,
            locked: false,
            updated_at: 1,
            last_source_turn_id: None,
        }
    }

    fn project() -> ProjectMetadata {
        ProjectMetadata {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: "project-1".to_string(),
            name: "暗潮协议".to_string(),
            author: "Tester".to_string(),
            description: Some("一部围绕代价交换展开的悬疑长篇".to_string()),
            cover_image: None,
            project_type: vec!["悬疑".to_string()],
            target_total_words: 300_000,
            planned_volumes: None,
            target_words_per_volume: None,
            target_words_per_chapter: None,
            narrative_pov: "third_limited".to_string(),
            tone: vec!["压迫".to_string()],
            audience: "general".to_string(),
            story_core: Some("秘密交易撬动旧秩序".to_string()),
            protagonist_anchor: Some("沈砚".to_string()),
            conflict_anchor: Some("规则以身份为代价".to_string()),
            origin_inspiration_session_id: Some("session-1".to_string()),
            planning_bundle_version: Some(1),
            bootstrap_state: ProjectBootstrapState::ScaffoldReady,
            bootstrap_updated_at: 1,
            created_at: 1,
            updated_at: 1,
            app_min_version: None,
            last_opened_at: Some(1),
        }
    }

    fn consensus() -> InspirationConsensusState {
        let mut state = InspirationConsensusState::default();
        state.story_core = field(
            ConsensusFieldId::StoryCore,
            ConsensusValue::Text("秘密交易撬动旧秩序".to_string()),
        );
        state.premise = field(
            ConsensusFieldId::Premise,
            ConsensusValue::Text("一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string()),
        );
        state.genre_tone = field(
            ConsensusFieldId::GenreTone,
            ConsensusValue::List(vec!["悬疑".to_string(), "压迫感".to_string()]),
        );
        state.protagonist = field(
            ConsensusFieldId::Protagonist,
            ConsensusValue::Text("沈砚".to_string()),
        );
        state.core_conflict = field(
            ConsensusFieldId::CoreConflict,
            ConsensusValue::Text("想查明真相就必须继续喂养那套危险规则".to_string()),
        );
        state
    }

    fn handoff() -> CreateProjectHandoffDraft {
        CreateProjectHandoffDraft {
            name: "暗潮协议".to_string(),
            description: "一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string(),
            project_type: vec!["悬疑".to_string()],
            tone: vec!["压迫".to_string()],
            audience: "偏好强情节女性向悬疑的读者".to_string(),
            protagonist_seed: Some("沈砚，擅长隐藏真实意图".to_string()),
            counterpart_seed: None,
            world_seed: None,
            ending_direction: Some("主角必须亲手切断最诱人的捷径".to_string()),
        }
    }

    fn seed_manifest(project_root: &Path) {
        let bundle = build_deterministic_planning_bundle(
            &project(),
            &consensus(),
            &handoff(),
            PlanningGenerationMetadata {
                generation_source: "deterministic_fallback".to_string(),
                generation_provider: None,
                generation_model: None,
            },
        )
        .expect("bundle");
        persist_planning_bundle(project_root, &bundle).expect("persist");
    }

    fn write_legacy_doc(project_root: &Path, relative_path: &str, content: &str) {
        crate::services::write_file(&project_root.join(relative_path), content)
            .expect("legacy doc");
    }

    #[tokio::test]
    async fn refresh_manifest_reads_existing_bundle() {
        let dir = tempdir().expect("temp");
        seed_manifest(dir.path());

        let manifest = refresh_planning_manifest(dir.path().to_string_lossy().to_string())
            .await
            .expect("manifest");

        assert_eq!(
            manifest.bundle_version,
            crate::models::PLANNING_BUNDLE_VERSION
        );
        assert_eq!(manifest.docs.len(), 6);
        assert!(manifest
            .writing_readiness
            .blockers
            .contains(&"narrative_contract_unconfirmed".to_string()));
    }

    #[tokio::test]
    async fn saving_tracked_doc_promotes_it_to_user_refined() {
        let dir = tempdir().expect("temp");
        seed_manifest(dir.path());

        save_knowledge_document(
            dir.path().to_string_lossy().to_string(),
            ".magic_novel/planning/narrative_contract.md".to_string(),
            "# Narrative Contract\n\nWe keep a close third person voice and escalate every chapter with a clear cost.\n".to_string(),
        )
        .await
        .expect("save");

        let manifest = get_planning_manifest(dir.path().to_string_lossy().to_string())
            .await
            .expect("manifest");
        let doc = manifest
            .docs
            .iter()
            .find(|entry| entry.id == "narrative_contract")
            .expect("doc");

        assert_eq!(doc.materialization_state, MaterializationState::Ready);
        assert_eq!(doc.approval_state, ApprovalState::UserRefined);
        assert_eq!(doc.last_source, SOURCE_USER_EDIT);
    }

    #[tokio::test]
    async fn refresh_manifest_maps_legacy_bootstrap_outputs_to_current_doc_entries() {
        let dir = tempdir().expect("temp");
        write_legacy_doc(
            dir.path(),
            ".magic_novel/system/creation_brief.md",
            "# Creation Brief\n\n一位总是自保的人被迫撕开秩序裂缝，并继续追查代价来源。\n",
        );
        write_legacy_doc(
            dir.path(),
            ".magic_novel/planning/story_blueprint.md",
            "# Story Blueprint\n\n主角被异常交易卷入更大的规则失衡，并在连续选择里逐步失去退路。\n",
        );
        write_legacy_doc(
            dir.path(),
            ".magic_novel/characters/protagonist.md",
            "# Protagonist\n\n沈砚，擅长隐藏真实意图，却已经没有办法继续旁观。\n",
        );
        write_legacy_doc(
            dir.path(),
            ".magic_novel/planning/chapter_backlog.md",
            "# Chapter Backlog\n\n- 第1章：异常信号进入视野，主角第一次被迫越界。\n",
        );
        write_legacy_doc(
            dir.path(),
            ".magic_novel/planning/volume_plan.md",
            "# Volume Plan\n\n## 卷1：裂缝显形\n- 目标字数：90000\n",
        );

        let manifest = refresh_planning_manifest(dir.path().to_string_lossy().to_string())
            .await
            .expect("manifest");

        let story_brief = manifest
            .doc(PlanningDocId::StoryBrief)
            .expect("story brief");
        assert_eq!(
            story_brief.materialization_state,
            MaterializationState::Ready
        );
        assert_eq!(story_brief.last_source, SOURCE_LEGACY_BOOTSTRAP);

        let character_cards = manifest
            .doc(PlanningDocId::CharacterCards)
            .expect("character cards");
        assert_eq!(
            character_cards.materialization_state,
            MaterializationState::Ready
        );
        assert_eq!(character_cards.last_source, SOURCE_LEGACY_BOOTSTRAP);

        let chapter_planning = manifest
            .doc(PlanningDocId::ChapterPlanning)
            .expect("chapter planning");
        assert_eq!(
            chapter_planning.materialization_state,
            MaterializationState::Ready
        );
        assert_eq!(chapter_planning.last_source, SOURCE_LEGACY_BOOTSTRAP);

        let narrative_contract = manifest
            .doc(PlanningDocId::NarrativeContract)
            .expect("narrative contract");
        assert_eq!(
            narrative_contract.materialization_state,
            MaterializationState::Failed
        );
        assert_eq!(
            manifest.recommended_next_doc,
            PlanningDocId::NarrativeContract.relative_path()
        );
        assert!(manifest
            .optional_outputs
            .iter()
            .any(|entry| entry == PlanningDocId::VolumePlan.as_str()));
        assert!(manifest.doc(PlanningDocId::VolumePlan).is_some());
    }

    #[tokio::test]
    async fn refresh_manifest_marks_template_only_project_as_needing_rework() {
        let dir = tempdir().expect("temp");
        write_legacy_doc(
            dir.path(),
            ".magic_novel/planning/story_blueprint.md",
            "# Story Blueprint\n\nTODO: complete the structure.\n",
        );
        write_legacy_doc(
            dir.path(),
            ".magic_novel/system/project_profile.md",
            "# Project Profile\n\n待补充\n",
        );

        let manifest = refresh_planning_manifest(dir.path().to_string_lossy().to_string())
            .await
            .expect("manifest");

        assert_eq!(manifest.bundle_status, "failed");
        assert!(manifest
            .docs
            .iter()
            .filter(|entry| entry.required_for_create)
            .all(|entry| entry.materialization_state == MaterializationState::Failed));
        assert_eq!(
            manifest.recommended_next_doc,
            PlanningDocId::NarrativeContract.relative_path()
        );
    }
}
