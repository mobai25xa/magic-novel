use std::path::Path;

use crate::llm::bootstrap::BootstrapVolumePlan;
use crate::models::{
    AppError, Chapter, ChapterCounts, ChapterStatus, VolumeMetadata, VolumeStatus,
};
use crate::services::{ensure_dir, read_json, write_json};

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";

pub fn sync_bootstrap_structure(
    project_path: &Path,
    volumes: &[BootstrapVolumePlan],
) -> Result<(), AppError> {
    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
    ensure_dir(&manuscripts_path)?;

    let target_volume_ids = volumes
        .iter()
        .enumerate()
        .map(|(index, _)| stable_volume_id(index))
        .collect::<Vec<_>>();
    prune_stale_bootstrap_volumes(&manuscripts_path, &target_volume_ids)?;

    for (volume_index, plan) in volumes.iter().enumerate() {
        let volume_id = stable_volume_id(volume_index);
        let volume_dir = manuscripts_path.join(&volume_id);
        ensure_dir(&volume_dir)?;

        let volume_file = volume_dir.join(VOLUME_FILE);
        let mut volume = if volume_file.exists() {
            read_json::<VolumeMetadata>(&volume_file)?
        } else {
            VolumeMetadata::new(plan.title.clone())
        };
        let created_at = volume.created_at;
        volume.schema_version = crate::models::VOLUME_SCHEMA_VERSION;
        volume.volume_id = volume_id.clone();
        volume.title = plan.title.clone();
        volume.summary = Some(plan.summary.clone());
        volume.target_words = plan.target_words;
        volume.dramatic_goal = plan.dramatic_goal.clone();
        volume.status = VolumeStatus::Planned;
        volume.created_at = created_at;
        volume.updated_at = chrono::Utc::now().timestamp_millis();

        let target_chapter_ids = plan
            .chapters
            .iter()
            .enumerate()
            .map(|(chapter_index, _)| stable_chapter_id(volume_index, chapter_index))
            .collect::<Vec<_>>();
        prune_stale_bootstrap_chapters(&volume_dir, &target_chapter_ids)?;

        volume.chapter_order = Vec::new();
        for (chapter_index, chapter_plan) in plan.chapters.iter().enumerate() {
            let chapter_id = stable_chapter_id(volume_index, chapter_index);
            let chapter_file = volume_dir.join(format!("{chapter_id}.json"));
            let existing = if chapter_file.exists() {
                Some(read_json::<Chapter>(&chapter_file)?)
            } else {
                None
            };
            let chapter = build_bootstrap_chapter(existing, &chapter_id, chapter_plan);
            write_json(&chapter_file, &chapter)?;
            volume.chapter_order.push(chapter_id);
        }

        write_json(&volume_file, &volume)?;
    }

    Ok(())
}

fn build_bootstrap_chapter(
    existing: Option<Chapter>,
    chapter_id: &str,
    plan: &crate::llm::bootstrap::BootstrapChapterPlan,
) -> Chapter {
    let mut chapter = existing.unwrap_or_else(|| Chapter::new(plan.title.clone()));
    let preserve_content = chapter_has_content(&chapter);
    let created_at = chapter.created_at;
    let preserved_content = chapter.content.clone();
    let preserved_counts = chapter.counts.clone();
    let preserved_status = chapter.status.clone();

    chapter.schema_version = 2;
    chapter.id = chapter_id.to_string();
    chapter.title = plan.title.clone();
    chapter.summary = Some(plan.summary.clone());
    chapter.plot_goal = Some(plan.plot_goal.clone());
    chapter.emotional_goal = Some(plan.emotional_goal.clone());
    chapter.target_words = Some(plan.target_words);
    chapter.created_at = created_at;
    chapter.updated_at = chrono::Utc::now().timestamp_millis();

    if preserve_content {
        chapter.content = preserved_content;
        chapter.counts = preserved_counts;
        chapter.status = preserved_status.or(Some(ChapterStatus::Draft));
    } else {
        chapter.content = serde_json::json!({
            "type": "doc",
            "content": []
        });
        chapter.counts = ChapterCounts::default();
        chapter.status = Some(ChapterStatus::Planned);
    }

    chapter
}

fn chapter_has_content(chapter: &Chapter) -> bool {
    if chapter.counts.text_length_no_whitespace > 0 {
        return true;
    }

    chapter
        .content
        .get("content")
        .and_then(|value| value.as_array())
        .map(|content| !content.is_empty())
        .unwrap_or(false)
}

fn prune_stale_bootstrap_volumes(
    manuscripts_path: &Path,
    target_volume_ids: &[String],
) -> Result<(), AppError> {
    if !manuscripts_path.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(manuscripts_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with("bootstrap-v") {
            continue;
        }
        if target_volume_ids.iter().any(|target| target == &file_name) {
            continue;
        }
        std::fs::remove_dir_all(entry.path())?;
    }

    Ok(())
}

fn prune_stale_bootstrap_chapters(
    volume_dir: &Path,
    target_chapter_ids: &[String],
) -> Result<(), AppError> {
    if !volume_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(volume_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == VOLUME_FILE
            || !file_name.starts_with("bootstrap-v")
            || !file_name.ends_with(".json")
        {
            continue;
        }
        let chapter_id = file_name.trim_end_matches(".json").to_string();
        if target_chapter_ids
            .iter()
            .any(|target| target == &chapter_id)
        {
            continue;
        }
        std::fs::remove_file(entry.path())?;
    }

    Ok(())
}

fn stable_volume_id(index: usize) -> String {
    format!("bootstrap-v{:02}", index + 1)
}

fn stable_chapter_id(volume_index: usize, chapter_index: usize) -> String {
    format!(
        "bootstrap-v{:02}-c{:02}",
        volume_index + 1,
        chapter_index + 1
    )
}
