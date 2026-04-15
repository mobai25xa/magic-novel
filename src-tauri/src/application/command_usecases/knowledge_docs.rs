use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::command;

use crate::models::{AppError, PlanningDocId};
use crate::services::jvm::{
    build_doc_from_markdown_blocks, ensure_doc_block_ids, parse_markdown_to_blocks,
};
use crate::services::{ensure_dir, read_file, write_file};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum KnowledgeTreeNode {
    #[serde(rename = "dir")]
    Dir {
        name: String,
        path: String,
        title: Option<String>,
        children: Vec<KnowledgeTreeNode>,
    },
    #[serde(rename = "file")]
    File {
        name: String,
        path: String,
        title: Option<String>,
        modified_at: Option<i64>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeDocument {
    pub path: String,
    pub title: String,
    pub markdown: String,
    pub content: serde_json::Value,
}

#[command]
pub async fn get_knowledge_tree(project_path: String) -> Result<Vec<KnowledgeTreeNode>, AppError> {
    let project_path = PathBuf::from(project_path);
    let root = project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
    build_knowledge_tree(
        &root,
        crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY,
    )
}

#[command]
pub async fn read_knowledge_document(
    project_path: String,
    virtual_path: String,
) -> Result<KnowledgeDocument, AppError> {
    let project_path = PathBuf::from(project_path);
    let normalized = normalize_knowledge_virtual_path(&virtual_path)?;
    ensure_markdown_virtual_path(&normalized)?;

    let full_path = crate::services::knowledge_paths::resolve_knowledge_physical_path(
        &project_path,
        &normalized,
    );
    let markdown = read_file(&full_path)?;
    let content = markdown_to_editor_doc(&markdown)?;

    Ok(KnowledgeDocument {
        path: normalized.clone(),
        title: resolve_document_title(&normalized, Some(&markdown)),
        markdown,
        content,
    })
}

#[command]
pub async fn save_knowledge_document(
    project_path: String,
    virtual_path: String,
    markdown: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(project_path);
    let normalized = normalize_knowledge_virtual_path(&virtual_path)?;
    ensure_markdown_virtual_path(&normalized)?;

    let rel = knowledge_relative_path(&normalized);
    let full_path =
        crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project_path)?
            .join(rel);
    write_file(&full_path, &markdown)?;
    crate::application::command_usecases::planning_status::mark_planning_doc_saved(
        &project_path,
        &normalized,
    )?;
    Ok(())
}

#[command]
pub async fn create_knowledge_folder(
    project_path: String,
    parent_virtual_dir: String,
    name: String,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(project_path);
    let normalized_parent = normalize_optional_knowledge_dir(&parent_virtual_dir)?;
    let folder_name = sanitize_path_segment(&name)?;
    let rel = knowledge_relative_path(&normalized_parent);
    let full_path =
        crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project_path)?
            .join(rel)
            .join(&folder_name);

    ensure_dir(&full_path)?;
    Ok(join_virtual_path(&normalized_parent, &folder_name))
}

#[command]
pub async fn create_knowledge_document(
    project_path: String,
    parent_virtual_dir: String,
    name: String,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(project_path);
    let normalized_parent = normalize_optional_knowledge_dir(&parent_virtual_dir)?;
    let file_stem = sanitize_path_segment(&name)?;
    let file_name = if file_stem.ends_with(".md") {
        file_stem
    } else {
        format!("{file_stem}.md")
    };
    let rel = knowledge_relative_path(&normalized_parent);
    let full_path =
        crate::services::knowledge_paths::resolve_knowledge_root_for_write(&project_path)?
            .join(rel)
            .join(&file_name);

    if full_path.exists() {
        return Err(AppError::invalid_argument("知识库文件已存在"));
    }

    let heading = name.trim();
    let initial = if heading.is_empty() {
        String::new()
    } else {
        format!("# {heading}\n")
    };
    write_file(&full_path, &initial)?;

    Ok(join_virtual_path(&normalized_parent, &file_name))
}

#[command]
pub async fn delete_knowledge_entry(
    project_path: String,
    virtual_path: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(project_path);
    let normalized = normalize_knowledge_virtual_path(&virtual_path)?;

    if normalized == crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY {
        return Err(AppError::invalid_argument("不允许删除知识库根目录"));
    }

    let full_path = crate::services::knowledge_paths::resolve_knowledge_physical_path(
        &project_path,
        &normalized,
    );
    if !full_path.exists() {
        return Ok(());
    }

    let meta = std::fs::metadata(&full_path)?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&full_path)?;
    } else {
        std::fs::remove_file(&full_path)?;
    }

    Ok(())
}

fn build_knowledge_tree(
    base_dir: &Path,
    virtual_dir: &str,
) -> Result<Vec<KnowledgeTreeNode>, AppError> {
    if !base_dir.exists() {
        return Ok(vec![]);
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(base_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let virtual_path = join_virtual_path(virtual_dir, &name);

        if file_type.is_dir() {
            let children = build_knowledge_tree(&path, &virtual_path)?;
            if children.is_empty() {
                continue;
            }
            out.push(KnowledgeTreeNode::Dir {
                title: Some(resolve_directory_title(&virtual_path, &name)),
                name,
                path: virtual_path,
                children,
            });
            continue;
        }

        if file_type.is_file() && name.ends_with(".md") {
            out.push(KnowledgeTreeNode::File {
                title: Some(resolve_tree_file_title(&virtual_path, &path)),
                name,
                path: virtual_path,
                modified_at: modified_at(&path),
            });
        }
    }

    sort_knowledge_nodes(&mut out);
    Ok(out)
}

fn markdown_to_editor_doc(markdown: &str) -> Result<serde_json::Value, AppError> {
    let (blocks, _diagnostics) = parse_markdown_to_blocks(markdown)?;
    let mut doc = if blocks.is_empty() {
        serde_json::json!({
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "attrs": { "id": uuid::Uuid::new_v4().to_string() },
                    "content": []
                }
            ]
        })
    } else {
        build_doc_from_markdown_blocks(&blocks)
    };
    ensure_doc_block_ids(&mut doc);
    Ok(doc)
}

fn normalize_optional_knowledge_dir(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY.to_string());
    }
    normalize_knowledge_virtual_path(trimmed)
}

fn normalize_knowledge_virtual_path(raw: &str) -> Result<String, AppError> {
    crate::services::knowledge_paths::normalize_knowledge_virtual_path(raw)
        .map_err(|error| AppError::invalid_argument(error.message))
}

fn ensure_markdown_virtual_path(path: &str) -> Result<(), AppError> {
    if !path.ends_with(".md") {
        return Err(AppError::invalid_argument("当前只支持 Markdown 知识库文件"));
    }
    Ok(())
}

fn knowledge_relative_path(virtual_path: &str) -> PathBuf {
    if virtual_path == crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY {
        return PathBuf::new();
    }

    PathBuf::from(
        virtual_path
            .trim_start_matches(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY)
            .trim_start_matches('/'),
    )
}

fn sanitize_path_segment(raw: &str) -> Result<String, AppError> {
    let sanitized = raw
        .trim()
        .replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_")
        .trim_matches('.')
        .trim()
        .to_string();

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        return Err(AppError::invalid_argument("请输入有效名称"));
    }

    Ok(sanitized)
}

fn resolve_document_title(virtual_path: &str, markdown: Option<&str>) -> String {
    crate::services::knowledge_paths::builtin_knowledge_display_name(virtual_path)
        .map(str::to_string)
        .or_else(|| markdown.and_then(extract_markdown_title))
        .unwrap_or_else(|| file_title(virtual_path))
}

fn resolve_directory_title(virtual_path: &str, fallback_name: &str) -> String {
    crate::services::knowledge_paths::builtin_knowledge_display_name(virtual_path)
        .unwrap_or(fallback_name)
        .to_string()
}

fn resolve_tree_file_title(virtual_path: &str, path: &Path) -> String {
    crate::services::knowledge_paths::builtin_knowledge_display_name(virtual_path)
        .map(str::to_string)
        .or_else(|| read_markdown_title_from_file(path))
        .unwrap_or_else(|| file_title(virtual_path))
}

fn file_title(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn read_markdown_title_from_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.ok()?;
        if let Some(title) = parse_markdown_heading(&line) {
            return Some(title);
        }
        if !line.trim().is_empty() {
            break;
        }
    }
    None
}

fn extract_markdown_title(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        if let Some(title) = parse_markdown_heading(line) {
            return Some(title);
        }
        if !line.trim().is_empty() {
            break;
        }
    }
    None
}

fn parse_markdown_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }

    let content = trimmed.trim_start_matches('#').trim();
    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
}

fn join_virtual_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        return name.to_string();
    }

    let normalized_base = base.trim_end_matches('/');
    format!("{normalized_base}/{name}")
}

fn modified_at(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
}

fn sort_knowledge_nodes(nodes: &mut [KnowledgeTreeNode]) {
    nodes.sort_by(|a, b| {
        let a_is_dir = matches!(a, KnowledgeTreeNode::Dir { .. });
        let b_is_dir = matches!(b, KnowledgeTreeNode::Dir { .. });
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => node_sort_rank(a)
                .cmp(&node_sort_rank(b))
                .then_with(|| node_title(a).cmp(node_title(b)))
                .then_with(|| node_name(a).cmp(node_name(b))),
        }
    });
}

fn node_sort_rank(node: &KnowledgeTreeNode) -> usize {
    match node_path(node)
        .and_then(PlanningDocId::from_relative_path)
        .map(PlanningDocId::sort_index)
    {
        Some(rank) => rank,
        None => usize::MAX,
    }
}

fn node_name(node: &KnowledgeTreeNode) -> &str {
    match node {
        KnowledgeTreeNode::Dir { name, .. } => name,
        KnowledgeTreeNode::File { name, .. } => name,
    }
}

fn node_title(node: &KnowledgeTreeNode) -> &str {
    match node {
        KnowledgeTreeNode::Dir { title, name, .. } => title.as_deref().unwrap_or(name),
        KnowledgeTreeNode::File { title, name, .. } => title.as_deref().unwrap_or(name),
    }
}

fn node_path(node: &KnowledgeTreeNode) -> Option<&str> {
    match node {
        KnowledgeTreeNode::Dir { path, .. } => Some(path),
        KnowledgeTreeNode::File { path, .. } => Some(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::services::{ensure_dir, write_file};

    #[test]
    fn built_in_planning_docs_keep_stable_chinese_titles() {
        let dir = tempdir().expect("temp dir");
        let planning_dir = dir.path().join(".magic_novel").join("planning");
        ensure_dir(&planning_dir).expect("planning dir");
        let doc_path = planning_dir.join("narrative_contract.md");
        write_file(&doc_path, "# Narrative Contract\n\nlegacy").expect("write file");

        let nodes = build_knowledge_tree(&planning_dir, ".magic_novel/planning").expect("tree");
        let title = match nodes.first().expect("planning doc") {
            KnowledgeTreeNode::File { title, .. } => title.clone(),
            _ => None,
        };

        assert_eq!(title.as_deref(), Some("叙事合同"));
        assert_eq!(
            resolve_document_title(
                ".magic_novel/planning/narrative_contract.md",
                Some("# Narrative Contract\n")
            ),
            "叙事合同"
        );
    }

    #[test]
    fn generic_docs_use_first_markdown_heading_as_title() {
        let dir = tempdir().expect("temp dir");
        let characters_dir = dir.path().join(".magic_novel").join("characters");
        ensure_dir(&characters_dir).expect("characters dir");
        let doc_path = characters_dir.join("alice.md");
        write_file(&doc_path, "# 阿离\n\n角色资料").expect("write file");

        assert_eq!(
            resolve_tree_file_title(".magic_novel/characters/alice.md", &doc_path),
            "阿离"
        );
        assert_eq!(
            resolve_document_title(".magic_novel/characters/alice.md", Some("# 阿离\n\n角色资料")),
            "阿离"
        );
    }

    #[test]
    fn planning_docs_sort_by_contract_order() {
        let dir = tempdir().expect("temp dir");
        let planning_dir = dir.path().join(".magic_novel").join("planning");
        ensure_dir(&planning_dir).expect("planning dir");

        for doc_id in [
            PlanningDocId::ChapterPlanning,
            PlanningDocId::StoryBrief,
            PlanningDocId::NarrativeContract,
        ] {
            let file_path = dir.path().join(doc_id.relative_path());
            write_file(&file_path, format!("{}\n\ncontent", doc_id.markdown_h1()).as_str())
                .expect("write file");
        }

        let nodes = build_knowledge_tree(&planning_dir, ".magic_novel/planning").expect("tree");
        let ordered_paths: Vec<String> = nodes
            .iter()
            .filter_map(|node| match node {
                KnowledgeTreeNode::File { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(
            ordered_paths,
            vec![
                ".magic_novel/planning/story_brief.md".to_string(),
                ".magic_novel/planning/narrative_contract.md".to_string(),
                ".magic_novel/planning/chapter_planning.md".to_string(),
            ]
        );
    }
}
