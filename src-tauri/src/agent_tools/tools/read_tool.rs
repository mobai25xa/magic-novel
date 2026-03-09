use serde_json::json;

use crate::agent_tools::contracts::{ReadInput, ReadKind, ViewFormat};
use crate::application::command_usecases::chapter::read_chapter_usecase;
use crate::application::command_usecases::volume::read_volume_usecase;
use crate::models::AppError;
use crate::services::jvm::build_chapter_snapshot;
use crate::services::{VcCommitPort, VersioningService};

#[derive(Debug, Clone, Copy)]
enum EffectiveReadKind {
    Volume,
    Chapter,
}

fn normalize_volume_path(path: &str) -> String {
    let path = path.trim().trim_matches('/');
    path.strip_suffix("/volume.json")
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_string())
}

fn resolve_kind(input: &ReadInput) -> EffectiveReadKind {
    if let Some(kind) = &input.kind {
        return match kind {
            ReadKind::Volume => EffectiveReadKind::Volume,
            ReadKind::Chapter => EffectiveReadKind::Chapter,
        };
    }

    let path = input.path.trim();
    if matches!(input.view, ViewFormat::Meta) {
        if path.ends_with("/volume.json") || (!path.ends_with(".json") && !path.contains('/')) {
            return EffectiveReadKind::Volume;
        }
    }

    EffectiveReadKind::Chapter
}

fn read_chapter_head(
    project_path: &str,
    chapter_path: &str,
) -> Result<crate::services::EntityHead, AppError> {
    let vc = VersioningService::new();
    let entity_id = format!("chapter:{}", chapter_path);
    vc.get_current_head(project_path, &entity_id)
}

pub fn run(input: ReadInput, call_id: &str) -> Result<serde_json::Value, AppError> {
    if input.project_path.trim().is_empty() || input.path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path/path are required",
        ));
    }

    let kind = resolve_kind(&input);
    let path = input.path.trim().to_string();

    match (kind, &input.view) {
        (EffectiveReadKind::Volume, ViewFormat::Meta) => {
            let normalized_path = normalize_volume_path(&path);
            let volume = read_volume_usecase(&input.project_path, &normalized_path)?;
            Ok(json!({
                "path": normalized_path,
                "kind": "domain_object",
                "metadata": {
                    "type": "volume",
                    "id": volume.volume_id,
                    "title": volume.title,
                    "summary": volume.summary,
                    "chapter_order": volume.chapter_order,
                    "created_at": volume.created_at,
                    "updated_at": volume.updated_at,
                }
            }))
        }
        (EffectiveReadKind::Volume, _) => Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: volume only supports view=meta",
        )),
        (EffectiveReadKind::Chapter, ViewFormat::Meta) => {
            let chapter = read_chapter_usecase(&input.project_path, &path)?;
            let head = read_chapter_head(&input.project_path, &path)?;

            Ok(json!({
                "path": input.path,
                "kind": "domain_object",
                "revision": head.revision,
                "hash": head.json_hash,
                "metadata": {
                    "type": "chapter",
                    "chapter_id": chapter.id,
                    "title": chapter.title,
                    "summary": chapter.summary,
                    "status": chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
                    "target_words": chapter.target_words,
                    "tags": chapter.tags,
                    "pinned_assets": chapter.pinned_assets,
                    "created_at": chapter.created_at,
                    "updated_at": chapter.updated_at,
                }
            }))
        }
        (EffectiveReadKind::Chapter, ViewFormat::Json) => {
            let chapter = read_chapter_usecase(&input.project_path, &path)?;
            let head = read_chapter_head(&input.project_path, &path)?;

            Ok(json!({
                "path": input.path,
                "kind": "domain_object",
                "revision": head.revision,
                "hash": head.json_hash,
                "content_json": chapter.content,
                "metadata": {
                    "type": "chapter",
                    "chapter_id": chapter.id,
                    "title": chapter.title,
                    "status": chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
                }
            }))
        }
        (EffectiveReadKind::Chapter, ViewFormat::Snapshot) => {
            let chapter = read_chapter_usecase(&input.project_path, &path)?;
            let head = read_chapter_head(&input.project_path, &path)?;

            let snapshot =
                build_chapter_snapshot(&path, head.revision, &head.json_hash, &chapter.content);

            Ok(json!({
                "path": input.path,
                "kind": "chapter",
                "revision": head.revision,
                "hash": head.json_hash,
                "snapshot": snapshot,
                "metadata": {
                    "type": "chapter",
                    "chapter_id": chapter.id,
                    "title": chapter.title,
                    "status": chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
                    "view": "snapshot",
                    "call_id": call_id,
                }
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_kind_defaults_to_chapter_snapshot() {
        let input = ReadInput {
            project_path: "D:/tmp/project".to_string(),
            path: "vol_1/ch_1.json".to_string(),
            kind: None,
            view: ViewFormat::Snapshot,
        };

        assert!(matches!(resolve_kind(&input), EffectiveReadKind::Chapter));
    }

    #[test]
    fn resolve_kind_volume_for_meta_volume_path() {
        let input = ReadInput {
            project_path: "D:/tmp/project".to_string(),
            path: "vol_1/volume.json".to_string(),
            kind: None,
            view: ViewFormat::Meta,
        };

        assert!(matches!(resolve_kind(&input), EffectiveReadKind::Volume));
    }

    #[test]
    fn normalize_volume_path_strips_suffix() {
        assert_eq!(normalize_volume_path("vol_1/volume.json"), "vol_1");
        assert_eq!(normalize_volume_path("vol_1"), "vol_1");
    }
}
