use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent_tools::tools::r#ref::{parse_tool_ref, RefKind};
use crate::services;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMapScope {
    Book,
    Volume,
    Knowledge,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceMapArgs {
    pub scope: Option<WorkspaceMapScope>,
    pub target_ref: Option<String>,
    pub depth: Option<u32>,
    pub include_stats: Option<bool>,
    pub include_children: Option<bool>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMapNode {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMapOutput {
    pub tree: Vec<WorkspaceMapNode>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMapRun {
    pub output: WorkspaceMapOutput,
    pub read_set: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMapError {
    pub code: &'static str,
    pub message: String,
}

pub fn run_workspace_map(
    project_path: &str,
    args: WorkspaceMapArgs,
) -> Result<WorkspaceMapRun, WorkspaceMapError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(WorkspaceMapError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "missing project_path".to_string(),
        });
    }

    let scope = args.scope.unwrap_or(WorkspaceMapScope::Book);
    let depth = args.depth.unwrap_or(2).min(2);
    let include_stats = args.include_stats.unwrap_or(true);
    let include_children = args.include_children.unwrap_or(true);
    let limit = args.limit.unwrap_or(200).clamp(1, 1000) as usize;
    let offset = parse_offset_cursor(args.cursor.as_deref())?;

    match scope {
        WorkspaceMapScope::Book => {
            let dataset = services::load_outline_dataset(project_path, None);
            let volume_count = dataset.volumes.len();
            let chapter_count: u32 = dataset.volumes.iter().map(|v| v.chapter_count).sum();
            let total_words: u64 = dataset.volumes.iter().map(|v| v.word_count).sum();

            let mut tree = Vec::new();
            if include_children && depth >= 1 {
                for vol in &dataset.volumes {
                    tree.push(WorkspaceMapNode {
                        ref_: format!("volume:manuscripts/{}", vol.volume_path),
                        kind: "volume".to_string(),
                        title: Some(vol.title.clone()),
                        path: Some(format!("manuscripts/{}", vol.volume_path)),
                        child_count: Some(vol.chapter_count),
                        word_count: include_stats.then_some(vol.word_count),
                        status: None,
                    });

                    if depth >= 2 {
                        for ch in &vol.chapters {
                            tree.push(WorkspaceMapNode {
                                ref_: format!("chapter:manuscripts/{}", ch.chapter_path),
                                kind: "chapter".to_string(),
                                title: Some(ch.title.clone()),
                                path: Some(format!("manuscripts/{}", ch.chapter_path)),
                                child_count: None,
                                word_count: include_stats.then_some(ch.word_count),
                                status: ch.status.as_ref().map(|s| s.to_lowercase()),
                            });
                        }
                    }
                }
            }

            let (page, next_cursor) = paginate(tree, offset, limit);
            let truncated = next_cursor.is_some();

            let mut summary = format!("book: volumes={volume_count}, chapters={chapter_count}");
            if include_stats {
                summary.push_str(&format!(", words={total_words}"));
            }
            if truncated {
                summary.push_str(", truncated=true");
            }

            Ok(WorkspaceMapRun {
                output: WorkspaceMapOutput {
                    tree: page,
                    summary,
                    truncated: truncated.then_some(true),
                    next_cursor,
                },
                read_set: None,
            })
        }
        WorkspaceMapScope::Volume => {
            let Some(target_ref) = args
                .target_ref
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return Err(WorkspaceMapError {
                    code: "E_TOOL_SCHEMA_INVALID",
                    message: "target_ref is required when scope=volume".to_string(),
                });
            };

            let tref = parse_tool_ref(target_ref).map_err(|err| WorkspaceMapError {
                code: err.code,
                message: err.message,
            })?;

            if tref.kind != RefKind::Volume {
                return Err(WorkspaceMapError {
                    code: "E_REF_KIND_UNSUPPORTED",
                    message: "target_ref must be a volume ref".to_string(),
                });
            }

            let volume_id = volume_id_from_volume_ref_path(&tref.path)?;

            let dataset = services::load_outline_dataset(project_path, Some(&volume_id));
            let Some(volume) = dataset.volumes.first() else {
                return Err(WorkspaceMapError {
                    code: "E_REF_NOT_FOUND",
                    message: "volume not found".to_string(),
                });
            };

            let mut tree = Vec::new();
            if include_children && depth >= 1 {
                for ch in &volume.chapters {
                    tree.push(WorkspaceMapNode {
                        ref_: format!("chapter:manuscripts/{}", ch.chapter_path),
                        kind: "chapter".to_string(),
                        title: Some(ch.title.clone()),
                        path: Some(format!("manuscripts/{}", ch.chapter_path)),
                        child_count: None,
                        word_count: include_stats.then_some(ch.word_count),
                        status: ch.status.as_ref().map(|s| s.to_lowercase()),
                    });
                }
            }

            let (page, next_cursor) = paginate(tree, offset, limit);
            let truncated = next_cursor.is_some();

            let mut summary = format!(
                "volume: title={}, chapters={}",
                volume.title, volume.chapter_count
            );
            if include_stats {
                summary.push_str(&format!(", words={}", volume.word_count));
            }
            if truncated {
                summary.push_str(", truncated=true");
            }

            Ok(WorkspaceMapRun {
                output: WorkspaceMapOutput {
                    tree: page,
                    summary,
                    truncated: truncated.then_some(true),
                    next_cursor,
                },
                read_set: Some(vec![format!("volume:manuscripts/{volume_id}")]),
            })
        }
        WorkspaceMapScope::Knowledge => {
            let project_root = Path::new(project_path);
            let roots = services::knowledge_paths::knowledge_read_roots(project_root);
            let dirs = services::knowledge_paths::knowledge_top_level_dirs(project_root);

            let mut tree = Vec::new();
            if include_children && depth >= 1 {
                for dir_name in &dirs {
                    let normalized = services::knowledge_paths::normalize_knowledge_virtual_path(
                        &format!(".magic_novel/{dir_name}"),
                    )
                    .map_err(|err| WorkspaceMapError {
                        code: err.code,
                        message: err.message,
                    })?;
                    let child_count = Some(count_knowledge_dir_files(&roots, dir_name));

                    tree.push(WorkspaceMapNode {
                        ref_: format!("knowledge:{normalized}"),
                        kind: "knowledge_dir".to_string(),
                        title: Some(dir_name.clone()),
                        path: Some(normalized),
                        child_count,
                        word_count: None,
                        status: None,
                    });
                }
            }

            let (page, next_cursor) = paginate(tree, offset, limit);
            let truncated = next_cursor.is_some();

            let mut summary = format!("knowledge: dirs={}", dirs.len());
            if truncated {
                summary.push_str(", truncated=true");
            }

            Ok(WorkspaceMapRun {
                output: WorkspaceMapOutput {
                    tree: page,
                    summary,
                    truncated: truncated.then_some(true),
                    next_cursor,
                },
                read_set: Some(vec!["knowledge:.magic_novel".to_string()]),
            })
        }
    }
}

fn volume_id_from_volume_ref_path(path: &str) -> Result<String, WorkspaceMapError> {
    let normalized = path
        .trim()
        .trim_start_matches("manuscripts/")
        .trim_end_matches("/volume.json")
        .trim_end_matches('/');

    let mut parts = normalized.split('/').filter(|p| !p.trim().is_empty());
    let Some(volume_id) = parts.next() else {
        return Err(WorkspaceMapError {
            code: "E_REF_INVALID",
            message: "invalid volume ref path".to_string(),
        });
    };
    if parts.next().is_some() {
        return Err(WorkspaceMapError {
            code: "E_REF_INVALID",
            message: "volume ref path must point to a volume directory".to_string(),
        });
    }
    Ok(volume_id.to_string())
}

fn parse_offset_cursor(cursor: Option<&str>) -> Result<usize, WorkspaceMapError> {
    let Some(cursor) = cursor.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(0);
    };

    cursor.parse::<usize>().map_err(|_| WorkspaceMapError {
        code: "E_TOOL_SCHEMA_INVALID",
        message: "cursor must be a numeric offset".to_string(),
    })
}

fn paginate<T: Clone>(items: Vec<T>, offset: usize, limit: usize) -> (Vec<T>, Option<String>) {
    if items.is_empty() {
        return (items, None);
    }

    let start = offset.min(items.len());
    let end = start.saturating_add(limit).min(items.len());
    let page = items[start..end].to_vec();
    let next_cursor = (end < items.len()).then_some(end.to_string());
    (page, next_cursor)
}

fn count_knowledge_dir_files(roots: &[PathBuf], dir_name: &str) -> u32 {
    let mut seen = HashSet::new();

    for root in roots {
        let dir = root.join(dir_name);
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    seen.insert(name.to_string());
                }
            }
        }
    }

    seen.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Chapter, VolumeMetadata};
    use tempfile::tempdir;

    fn write_volume(project: &Path, volume_id: &str, title: &str) {
        let vol_dir = project.join("manuscripts").join(volume_id);
        services::ensure_dir(&vol_dir).expect("dir");
        let mut vol = VolumeMetadata::new(title.to_string());
        vol.volume_id = volume_id.to_string();
        services::write_json(&vol_dir.join("volume.json"), &vol).expect("write volume");
    }

    fn write_chapter(project: &Path, volume_id: &str, file_name: &str, title: &str) {
        let ch_path = project.join("manuscripts").join(volume_id).join(file_name);
        let mut ch = Chapter::new(title.to_string());
        ch.id = file_name.trim_end_matches(".json").to_string();
        ch.counts.text_length_no_whitespace = 10;
        services::write_json(&ch_path, &ch).expect("write chapter");
    }

    #[test]
    fn volume_scope_rejects_non_volume_ref() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().to_string_lossy().to_string();

        let args = WorkspaceMapArgs {
            scope: Some(WorkspaceMapScope::Volume),
            target_ref: Some("chapter:manuscripts/vol_1/ch_1.json".to_string()),
            depth: None,
            include_stats: None,
            include_children: None,
            cursor: None,
            limit: None,
            timeout_ms: None,
        };

        let err = run_workspace_map(&project_path, args).unwrap_err();
        assert_eq!(err.code, "E_REF_KIND_UNSUPPORTED");
    }

    #[test]
    fn book_scope_paginates_with_cursor() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        services::ensure_dir(&project.join("manuscripts")).expect("manuscripts");

        write_volume(project, "vol_1", "Vol 1");
        write_chapter(project, "vol_1", "ch_1.json", "Ch 1");
        write_chapter(project, "vol_1", "ch_2.json", "Ch 2");

        let project_path = project.to_string_lossy().to_string();

        let first = run_workspace_map(
            &project_path,
            WorkspaceMapArgs {
                scope: Some(WorkspaceMapScope::Book),
                target_ref: None,
                depth: Some(2),
                include_stats: Some(false),
                include_children: Some(true),
                cursor: None,
                limit: Some(2),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(first.output.tree.len(), 2);
        assert_eq!(first.output.truncated, Some(true));
        let cursor = first.output.next_cursor.clone().expect("cursor");

        let second = run_workspace_map(
            &project_path,
            WorkspaceMapArgs {
                scope: Some(WorkspaceMapScope::Book),
                target_ref: None,
                depth: Some(2),
                include_stats: Some(false),
                include_children: Some(true),
                cursor: Some(cursor),
                limit: Some(2),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(second.output.tree.len(), 1);
        assert_eq!(second.output.next_cursor, None);
    }

    #[test]
    fn knowledge_scope_maps_canonical_root_to_dir_refs() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        services::ensure_dir(&project.join(".magic_novel").join("characters")).expect("dir");
        std::fs::write(
            project
                .join(".magic_novel")
                .join("characters")
                .join("alice.md"),
            "# Alice\n",
        )
        .expect("write");

        let run = run_workspace_map(
            &project.to_string_lossy(),
            WorkspaceMapArgs {
                scope: Some(WorkspaceMapScope::Knowledge),
                target_ref: None,
                depth: Some(1),
                include_stats: Some(false),
                include_children: Some(true),
                cursor: None,
                limit: Some(20),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(run.output.summary, "knowledge: dirs=1");
        assert_eq!(run.output.tree.len(), 1);
        assert_eq!(run.output.tree[0].ref_, "knowledge:.magic_novel/characters");
        assert_eq!(run.output.tree[0].child_count, Some(1));
    }
}
