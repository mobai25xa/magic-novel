use crate::models::{
    AppError, AssetKind, AssetNode, AssetSource, AssetTree, Chapter, VolumeMetadata,
};
use crate::services::ensure_dir;
use crate::services::{read_json, write_json};
use crate::utils::atomic_write::atomic_write_json;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use std::fs;
use std::path::PathBuf;

pub fn read_supported_text(input_path: &PathBuf) -> Result<(String, String), AppError> {
    if !input_path.exists() {
        return Err(AppError::not_found("输入文件不存在"));
    }

    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = match extension.as_str() {
        "txt" | "md" => fs::read_to_string(input_path)?,
        _ => {
            return Err(AppError::invalid_argument(
                "不支持的文件格式，仅支持 txt/md",
            ))
        }
    };

    Ok((content, extension))
}

pub fn parse_to_asset_tree(
    content: &str,
    filename: &str,
    kind: &str,
    importer: &str,
) -> Result<AssetTree, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let asset_id = uuid::Uuid::new_v4().to_string();

    Ok(AssetTree {
        schema_version: 1,
        id: asset_id,
        kind: asset_kind_from_str(kind)?,
        title: filename.to_string(),
        source: Some(AssetSource {
            original_filename: Some(filename.to_string()),
            imported_at: now,
            importer: importer.to_string(),
        }),
        root: parse_markdown_to_nodes(content),
    })
}

fn asset_kind_from_str(kind: &str) -> Result<AssetKind, AppError> {
    match kind {
        "lore" => Ok(AssetKind::Lore),
        "prompt" => Ok(AssetKind::Prompt),
        "worldview" => Ok(AssetKind::Worldview),
        "outline" => Ok(AssetKind::Outline),
        "character" => Ok(AssetKind::Character),
        _ => Err(AppError::invalid_argument("无效的资产类型")),
    }
}

pub fn parse_markdown_to_nodes(content: &str) -> AssetNode {
    let parser = Parser::new(content);
    let mut root = AssetNode {
        node_id: uuid::Uuid::new_v4().to_string(),
        title: "root".to_string(),
        level: 0,
        content: String::new(),
        children: Vec::new(),
        tags: None,
    };

    let mut stack: Vec<AssetNode> = vec![];
    let mut current_text = String::new();
    let mut current_heading: Option<(i32, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading(level, _, _)) => {
                flush_text_to_current(&mut stack, &mut root, &mut current_text);
                current_heading = Some((heading_level_to_i32(level), String::new()));
            }
            Event::End(Tag::Heading(_, _, _)) => {
                if let Some((level, title)) = current_heading.take() {
                    attach_node_for_heading(&mut stack, &mut root, level, title);
                }
            }
            Event::Text(text) => {
                if let Some((_, title)) = current_heading.as_mut() {
                    title.push_str(&text);
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak if current_heading.is_none() => {
                current_text.push('\n');
            }
            Event::End(Tag::Paragraph) => current_text.push_str("\n\n"),
            _ => {}
        }
    }

    flush_text_to_current(&mut stack, &mut root, &mut current_text);
    attach_remaining_nodes(&mut stack, &mut root);
    root
}

fn heading_level_to_i32(level: HeadingLevel) -> i32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn flush_text_to_current(stack: &mut [AssetNode], root: &mut AssetNode, current_text: &mut String) {
    if current_text.is_empty() {
        return;
    }

    if let Some(node) = stack.last_mut() {
        node.content.push_str(current_text);
    } else {
        root.content.push_str(current_text);
    }
    current_text.clear();
}

fn attach_node_for_heading(
    stack: &mut Vec<AssetNode>,
    root: &mut AssetNode,
    level: i32,
    title: String,
) {
    while let Some(node) = stack.pop() {
        if node.level < level {
            stack.push(node);
            break;
        }
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            root.children.push(node);
        }
    }

    stack.push(AssetNode {
        node_id: uuid::Uuid::new_v4().to_string(),
        title,
        level,
        content: String::new(),
        children: Vec::new(),
        tags: None,
    });
}

fn attach_remaining_nodes(stack: &mut Vec<AssetNode>, root: &mut AssetNode) {
    while let Some(node) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            root.children.push(node);
        }
    }
}

pub fn ensure_volume_dir_with_meta(
    manuscripts_dir: &PathBuf,
    volume_path: &str,
) -> Result<PathBuf, AppError> {
    let volume_dir = manuscripts_dir.join(volume_path);
    ensure_dir(&volume_dir)?;

    let volume_file = volume_dir.join("volume.json");
    if !volume_file.exists() {
        let volume_meta = VolumeMetadata::new("默认卷".to_string());
        write_json(&volume_file, &volume_meta)?;
    }

    Ok(volume_dir)
}

pub fn build_chapter_content_json(content: &str) -> serde_json::Value {
    let paragraphs: Vec<serde_json::Value> = content
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .map(|para| {
            serde_json::json!({
                "type": "paragraph",
                "attrs": { "id": uuid::Uuid::new_v4().to_string() },
                "content": [{ "type": "text", "text": para.trim() }]
            })
        })
        .collect();

    serde_json::json!({
        "type": "doc",
        "content": paragraphs
    })
}

pub fn parse_manuscript_to_chapters(
    content: &str,
    manuscripts_dir: &PathBuf,
) -> Result<(), AppError> {
    let parser = Parser::new(content);
    let mut current_volume: Option<(String, PathBuf)> = None;
    let mut current_chapter: Option<(String, String)> = None;
    let mut chapter_count = 0;
    let mut in_heading = false;
    let mut heading_level = 0;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading(level, _, _)) => {
                in_heading = true;
                heading_level = heading_level_to_i32(level);
                heading_text.clear();
            }
            Event::End(Tag::Heading(_, _, _)) => {
                in_heading = false;
                handle_heading_end(
                    manuscripts_dir,
                    &mut current_volume,
                    &mut current_chapter,
                    &mut chapter_count,
                    heading_level,
                    &heading_text,
                )?;
                heading_text.clear();
            }
            Event::Text(text) if in_heading => heading_text.push_str(&text),
            Event::Text(text) => append_text_to_current_chapter(&mut current_chapter, &text),
            Event::SoftBreak | Event::HardBreak => {
                append_text_to_current_chapter(&mut current_chapter, "\n")
            }
            Event::End(Tag::Paragraph) => {
                append_text_to_current_chapter(&mut current_chapter, "\n\n")
            }
            _ => {}
        }
    }

    finalize_current_chapter(
        manuscripts_dir,
        &current_volume,
        &mut current_chapter,
        chapter_count,
    )
}

fn handle_heading_end(
    manuscripts_dir: &PathBuf,
    current_volume: &mut Option<(String, PathBuf)>,
    current_chapter: &mut Option<(String, String)>,
    chapter_count: &mut i32,
    heading_level: i32,
    heading_text: &str,
) -> Result<(), AppError> {
    if heading_level == 1 {
        finalize_current_chapter(
            manuscripts_dir,
            current_volume,
            current_chapter,
            *chapter_count,
        )?;
        *chapter_count = 0;
        *current_volume = Some(create_volume_for_heading(manuscripts_dir, heading_text)?);
        return Ok(());
    }

    if heading_level == 2 {
        finalize_current_chapter(
            manuscripts_dir,
            current_volume,
            current_chapter,
            *chapter_count,
        )?;
        *chapter_count += 1;
        *current_chapter = Some((heading_text.to_string(), String::new()));
    }

    Ok(())
}

fn append_text_to_current_chapter(current_chapter: &mut Option<(String, String)>, text: &str) {
    if let Some((_, content)) = current_chapter.as_mut() {
        content.push_str(text);
    }
}

fn create_volume_for_heading(
    manuscripts_dir: &PathBuf,
    heading_text: &str,
) -> Result<(String, PathBuf), AppError> {
    let volume_meta = VolumeMetadata::new(heading_text.to_string());
    let volume_dir = manuscripts_dir.join(&volume_meta.volume_id);
    ensure_dir(&volume_dir)?;
    atomic_write_json(&volume_dir.join("volume.json"), &volume_meta)?;
    Ok((heading_text.to_string(), volume_dir))
}

fn finalize_current_chapter(
    manuscripts_dir: &PathBuf,
    volume: &Option<(String, PathBuf)>,
    current_chapter: &mut Option<(String, String)>,
    chapter_count: i32,
) -> Result<(), AppError> {
    if let Some((chapter_title, chapter_content)) = current_chapter.take() {
        save_imported_chapter(
            manuscripts_dir,
            volume,
            &chapter_title,
            &chapter_content,
            chapter_count,
        )?;
    }
    Ok(())
}

fn save_imported_chapter(
    manuscripts_dir: &PathBuf,
    volume: &Option<(String, PathBuf)>,
    title: &str,
    content: &str,
    _index: i32,
) -> Result<(), AppError> {
    let volume_dir = match volume {
        Some((_, dir)) => dir.clone(),
        None => {
            let volume_meta = VolumeMetadata::new("默认卷".to_string());
            let default_dir = manuscripts_dir.join(&volume_meta.volume_id);
            ensure_dir(&default_dir)?;
            let volume_file = default_dir.join("volume.json");
            if !volume_file.exists() {
                atomic_write_json(&volume_file, &volume_meta)?;
            }
            default_dir
        }
    };

    let mut chapter = Chapter::new(title.to_string());
    chapter.content = build_chapter_content_json(content);
    chapter.counts.text_length_no_whitespace =
        content.chars().filter(|c| !c.is_whitespace()).count() as i32;

    let filename = format!("{}.json", chapter.id);
    atomic_write_json(&volume_dir.join(&filename), &chapter)?;

    let volume_file = volume_dir.join("volume.json");
    if volume_file.exists() {
        let mut volume_meta: VolumeMetadata = read_json(&volume_file)?;
        if !volume_meta.chapter_order.contains(&chapter.id) {
            volume_meta.chapter_order.push(chapter.id.clone());
            volume_meta.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&volume_file, &volume_meta)?;
        }
    }

    Ok(())
}
