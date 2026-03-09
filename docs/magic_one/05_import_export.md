# 导入导出功能文档

> 本文档详细描述导入导出功能的实现。

---

## 1. 功能概览

### 1.1 导入功能

| 功能 | 输入 | 输出 |
|------|------|------|
| 资产导入 | txt/md/docx | AssetTree JSON (lore/prompts) |
| 正文导入 | txt/md/docx | 卷目录 + Chapter JSON |

### 1.2 导出功能

| 功能 | 输入 | 输出 |
|------|------|------|
| 单文件导出 | 整本书 | txt/md/docx |
| 多文件导出 | 整本书 | 按目录结构的 txt/md/docx |

---

## 2. 依赖库

### 2.1 Rust 依赖

在 `Cargo.toml` 添加：

```toml
[dependencies]
pulldown-cmark = "0.9"           # Markdown 解析
docx-rs = "0.4"                  # DOCX 读写
regex = "1"                      # 正则表达式
```

---

## 3. 导入实现

### 3.1 导入命令 - commands/import.rs

```rust
use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use tauri::command;
use specta::specta;

use crate::models::*;
use crate::services::*;
use crate::utils::*;

#[command]
#[specta]
pub async fn import_asset(
    project_root: String,
    input_path: String,
    kind: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&project_root);
    let input_path = PathBuf::from(&input_path);

    if !input_path.exists() {
        return Err(AppError::not_found("输入文件不存在"));
    }

    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = match extension.as_str() {
        "txt" => fs::read_to_string(&input_path)?,
        "md" => fs::read_to_string(&input_path)?,
        "docx" => extract_docx_text(&input_path)?,
        _ => return Err(AppError::invalid_argument("不支持的文件格式")),
    };

    let filename = input_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("未命名")
        .to_string();

    let asset_tree = parse_to_asset_tree(&content, &filename, &kind, &extension)?;

    let asset_dir = match kind.as_str() {
        "lore" => project_root.join("magic_novel/lore"),
        "prompt" => project_root.join("magic_novel/prompts"),
        _ => return Err(AppError::invalid_argument("无效的资产类型")),
    };

    fs::create_dir_all(&asset_dir)?;
    let asset_path = asset_dir.join(format!("{}.json", asset_tree.id));
    atomic_write_json(&asset_path, &asset_tree)?;

    Ok(asset_tree.id)
}

#[command]
#[specta]
pub async fn import_manuscript(
    project_root: String,
    input_path: String,
) -> Result<(), AppError> {
    let project_root = PathBuf::from(&project_root);
    let input_path = PathBuf::from(&input_path);

    if !input_path.exists() {
        return Err(AppError::not_found("输入文件不存在"));
    }

    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = match extension.as_str() {
        "txt" => fs::read_to_string(&input_path)?,
        "md" => fs::read_to_string(&input_path)?,
        "docx" => extract_docx_text(&input_path)?,
        _ => return Err(AppError::invalid_argument("不支持的文件格式")),
    };

    let content_dir = project_root.join("content");
    fs::create_dir_all(&content_dir)?;

    parse_manuscript_to_chapters(&content, &content_dir)?;

    Ok(())
}

fn extract_docx_text(path: &Path) -> Result<String, AppError> {
    let file = fs::File::open(path)?;
    let doc = docx_rs::read_docx(&file).map_err(|e| AppError::internal(e.to_string()))?;
    
    let mut text = String::new();
    for child in doc.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            let mut para_text = String::new();
            for child in para.children {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    for child in run.children {
                        if let docx_rs::RunChild::Text(t) = child {
                            para_text.push_str(&t.text);
                        }
                    }
                }
            }
            
            let style = para.property.style.as_ref().map(|s| s.val.as_str());
            match style {
                Some("Heading1") | Some("heading 1") => {
                    text.push_str(&format!("# {}\n\n", para_text));
                }
                Some("Heading2") | Some("heading 2") => {
                    text.push_str(&format!("## {}\n\n", para_text));
                }
                Some("Heading3") | Some("heading 3") => {
                    text.push_str(&format!("### {}\n\n", para_text));
                }
                _ => {
                    if !para_text.is_empty() {
                        text.push_str(&para_text);
                        text.push_str("\n\n");
                    }
                }
            }
        }
    }

    Ok(text)
}

fn parse_to_asset_tree(
    content: &str,
    filename: &str,
    kind: &str,
    importer: &str,
) -> Result<AssetTree, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let asset_id = uuid::Uuid::new_v4().to_string();

    let root = parse_markdown_to_nodes(content);

    let asset_kind = match kind {
        "lore" => AssetKind::Lore,
        "prompt" => AssetKind::Prompt,
        _ => return Err(AppError::invalid_argument("无效的资产类型")),
    };

    Ok(AssetTree {
        schema_version: 1,
        id: asset_id,
        kind: asset_kind,
        title: filename.to_string(),
        source: Some(AssetSource {
            original_filename: Some(filename.to_string()),
            imported_at: now,
            importer: importer.to_string(),
        }),
        root,
    })
}

fn parse_markdown_to_nodes(content: &str) -> AssetNode {
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
                if !current_text.is_empty() {
                    if let Some(node) = stack.last_mut() {
                        node.content.push_str(&current_text);
                    } else {
                        root.content.push_str(&current_text);
                    }
                    current_text.clear();
                }

                let level_num = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };

                current_heading = Some((level_num, String::new()));
            }
            Event::End(Tag::Heading(_, _, _)) => {
                if let Some((level, title)) = current_heading.take() {
                    while let Some(mut node) = stack.pop() {
                        if node.level < level {
                            stack.push(node);
                            break;
                        } else {
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(node);
                            } else {
                                root.children.push(node);
                            }
                        }
                    }

                    let new_node = AssetNode {
                        node_id: uuid::Uuid::new_v4().to_string(),
                        title,
                        level,
                        content: String::new(),
                        children: Vec::new(),
                        tags: None,
                    };
                    stack.push(new_node);
                }
            }
            Event::Text(text) => {
                if let Some((_, ref mut title)) = current_heading {
                    title.push_str(&text);
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if current_heading.is_none() {
                    current_text.push('\n');
                }
            }
            Event::End(Tag::Paragraph) => {
                current_text.push_str("\n\n");
            }
            _ => {}
        }
    }

    if !current_text.is_empty() {
        if let Some(node) = stack.last_mut() {
            node.content.push_str(&current_text);
        } else {
            root.content.push_str(&current_text);
        }
    }

    while let Some(mut node) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            root.children.push(node);
        }
    }

    root
}

fn parse_manuscript_to_chapters(content: &str, content_dir: &Path) -> Result<(), AppError> {
    let parser = Parser::new(content);
    
    let mut current_volume: Option<(String, PathBuf)> = None;
    let mut current_chapter: Option<(String, String)> = None;
    let mut current_content = String::new();
    let mut chapter_count = 0;
    let mut volume_count = 0;

    let mut in_heading = false;
    let mut heading_level = 0;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading(level, _, _)) => {
                in_heading = true;
                heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    _ => 3,
                };
                heading_text.clear();
            }
            Event::End(Tag::Heading(_, _, _)) => {
                in_heading = false;

                if heading_level == 1 {
                    if let Some((chapter_title, chapter_content)) = current_chapter.take() {
                        save_chapter(
                            content_dir,
                            &current_volume,
                            &chapter_title,
                            &chapter_content,
                            chapter_count,
                        )?;
                    }
                    current_content.clear();

                    volume_count += 1;
                    chapter_count = 0;
                    let folder_name = format!("{:02}_{}", volume_count, sanitize_filename(&heading_text));
                    let volume_dir = content_dir.join(&folder_name);
                    fs::create_dir_all(&volume_dir)?;

                    let volume_meta = VolumeMetadata::new(heading_text.clone());
                    atomic_write_json(&volume_dir.join("_volume.json"), &volume_meta)?;

                    current_volume = Some((heading_text.clone(), volume_dir));
                } else if heading_level == 2 {
                    if let Some((chapter_title, chapter_content)) = current_chapter.take() {
                        save_chapter(
                            content_dir,
                            &current_volume,
                            &chapter_title,
                            &chapter_content,
                            chapter_count,
                        )?;
                    }

                    chapter_count += 1;
                    current_chapter = Some((heading_text.clone(), String::new()));
                    current_content.clear();
                }

                heading_text.clear();
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else {
                    current_content.push_str(&text);
                    if let Some((_, ref mut content)) = current_chapter {
                        content.push_str(&text);
                    }
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
                if let Some((_, ref mut content)) = current_chapter {
                    content.push('\n');
                }
            }
            Event::End(Tag::Paragraph) => {
                current_content.push_str("\n\n");
                if let Some((_, ref mut content)) = current_chapter {
                    content.push_str("\n\n");
                }
            }
            _ => {}
        }
    }

    if let Some((chapter_title, chapter_content)) = current_chapter.take() {
        save_chapter(
            content_dir,
            &current_volume,
            &chapter_title,
            &chapter_content,
            chapter_count,
        )?;
    }

    if current_volume.is_none() && !current_content.trim().is_empty() {
        chapter_count += 1;
        save_chapter(
            content_dir,
            &None,
            "导入内容",
            &current_content,
            chapter_count,
        )?;
    }

    Ok(())
}

fn save_chapter(
    content_dir: &Path,
    current_volume: &Option<(String, PathBuf)>,
    title: &str,
    content: &str,
    chapter_num: i32,
) -> Result<(), AppError> {
    let paragraphs: Vec<serde_json::Value> = content
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            serde_json::json!({
                "type": "paragraph",
                "attrs": { "id": uuid::Uuid::new_v4().to_string() },
                "content": [{ "type": "text", "text": p.trim() }]
            })
        })
        .collect();

    let mut chapter = Chapter::new(title.to_string());
    chapter.content = serde_json::json!({
        "type": "doc",
        "content": paragraphs
    });

    let (char_count, word_count) = calculate_counts(&chapter.content);
    chapter.counts.text_length_no_whitespace = char_count;
    chapter.counts.word_count = Some(word_count);

    let target_dir = current_volume
        .as_ref()
        .map(|(_, dir)| dir.clone())
        .unwrap_or_else(|| content_dir.to_path_buf());

    let filename = format!("{:03}_{}.json", chapter_num, sanitize_filename(title));
    atomic_write_json(&target_dir.join(filename), &chapter)?;

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c > '\u{7F}' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
```

---

## 4. 导出实现

### 4.1 导出命令 - commands/export.rs

```rust
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tauri::command;
use specta::specta;

use crate::models::*;
use crate::services::*;

#[command]
#[specta]
pub async fn export_book_single(
    project_root: String,
    format: String,
    output_path: String,
) -> Result<(), AppError> {
    let project_root = PathBuf::from(&project_root);
    let output_path = PathBuf::from(&output_path);
    let content_dir = project_root.join("content");

    let mut full_content = String::new();

    collect_content_recursive(&content_dir, &mut full_content, &format, 0)?;

    match format.as_str() {
        "txt" | "md" => {
            fs::write(&output_path, &full_content)?;
        }
        "docx" => {
            export_to_docx(&full_content, &output_path)?;
        }
        _ => return Err(AppError::invalid_argument("不支持的导出格式")),
    }

    Ok(())
}

#[command]
#[specta]
pub async fn export_tree_multi(
    project_root: String,
    format: String,
    output_dir: String,
) -> Result<(), AppError> {
    let project_root = PathBuf::from(&project_root);
    let output_dir = PathBuf::from(&output_dir);
    let content_dir = project_root.join("content");

    fs::create_dir_all(&output_dir)?;

    export_tree_recursive(&content_dir, &output_dir, &format)?;

    Ok(())
}

fn collect_content_recursive(
    dir: &Path,
    content: &mut String,
    format: &str,
    depth: usize,
) -> Result<(), AppError> {
    if !dir.exists() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().map_or(false, |e| e == "json") {
            let name = entry.file_name().to_string_lossy().to_string();
            if name != "_volume.json" {
                files.push(path);
            }
        }
    }

    for dir_path in &dirs {
        let volume_meta_path = dir_path.join("_volume.json");
        if volume_meta_path.exists() {
            if let Ok(meta_content) = fs::read_to_string(&volume_meta_path) {
                if let Ok(volume_meta) = serde_json::from_str::<VolumeMetadata>(&meta_content) {
                    let heading = match format {
                        "md" => format!("# {}\n\n", volume_meta.title),
                        _ => format!("{}\n\n", volume_meta.title),
                    };
                    content.push_str(&heading);
                }
            }
        }

        collect_content_recursive(dir_path, content, format, depth + 1)?;
    }

    for file_path in files {
        if let Ok(file_content) = fs::read_to_string(&file_path) {
            if let Ok(chapter) = serde_json::from_str::<Chapter>(&file_content) {
                let chapter_heading = match format {
                    "md" => format!("## {}\n\n", chapter.title),
                    _ => format!("{}\n\n", chapter.title),
                };
                content.push_str(&chapter_heading);

                let text = extract_plain_text(&chapter.content);
                content.push_str(&text);
                content.push_str("\n\n");
            }
        }
    }

    Ok(())
}

fn export_tree_recursive(
    source_dir: &Path,
    output_dir: &Path,
    format: &str,
) -> Result<(), AppError> {
    if !source_dir.exists() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(source_dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let sub_output_dir = output_dir.join(&name);
            fs::create_dir_all(&sub_output_dir)?;
            export_tree_recursive(&path, &sub_output_dir, format)?;
        } else if path.extension().map_or(false, |e| e == "json") && name != "_volume.json" {
            if let Ok(file_content) = fs::read_to_string(&path) {
                if let Ok(chapter) = serde_json::from_str::<Chapter>(&file_content) {
                    let base_name = path.file_stem().unwrap().to_string_lossy();
                    let output_file = output_dir.join(format!("{}.{}", base_name, format));

                    let text = extract_plain_text(&chapter.content);
                    let content = match format {
                        "md" => format!("# {}\n\n{}", chapter.title, text),
                        _ => format!("{}\n\n{}", chapter.title, text),
                    };

                    fs::write(&output_file, &content)?;
                }
            }
        }
    }

    Ok(())
}

fn export_to_docx(content: &str, output_path: &Path) -> Result<(), AppError> {
    use docx_rs::*;

    let mut doc = Docx::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("# ") {
            let text = &trimmed[2..];
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(text).bold())
                    .style("Heading1"),
            );
        } else if trimmed.starts_with("## ") {
            let text = &trimmed[3..];
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(text).bold())
                    .style("Heading2"),
            );
        } else {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(trimmed)),
            );
        }
    }

    let file = File::create(output_path)?;
    doc.build().pack(file).map_err(|e| AppError::internal(e.to_string()))?;

    Ok(())
}
```

---

## 5. 更新命令模块

### 5.1 commands/mod.rs

```rust
pub mod ai;
pub mod chapter;
pub mod export;
pub mod import;
pub mod project;
pub mod volume;

pub use ai::*;
pub use chapter::*;
pub use export::*;
pub use import::*;
pub use project::*;
pub use volume::*;
```

### 5.2 更新 lib.rs

```rust
.commands(collect_commands![
    // ... 现有命令
    commands::import_asset,
    commands::import_manuscript,
    commands::export_book_single,
    commands::export_tree_multi,
])
```

---

## 6. 前端导入导出 UI

### 6.1 添加到 TopBar 菜单

```tsx
<DropdownMenuSeparator />
<DropdownMenuItem onClick={importAssetDialog}>
  <FileInput className="mr-2 h-4 w-4" />
  导入资产
</DropdownMenuItem>
<DropdownMenuItem onClick={importManuscriptDialog}>
  <FileInput className="mr-2 h-4 w-4" />
  导入正文
</DropdownMenuItem>
<DropdownMenuSeparator />
<DropdownMenuItem onClick={exportSingleDialog}>
  <FileOutput className="mr-2 h-4 w-4" />
  导出整本
</DropdownMenuItem>
<DropdownMenuItem onClick={exportTreeDialog}>
  <FolderOutput className="mr-2 h-4 w-4" />
  按目录导出
</DropdownMenuItem>
```

### 6.2 导入导出函数

```typescript
const importAssetDialog = async () => {
  const selected = await open({
    multiple: false,
    filters: [
      { name: 'Documents', extensions: ['txt', 'md', 'docx'] }
    ],
  });
  
  if (selected && projectRoot) {
    const kind = await askAssetKind();
    if (kind) {
      await commands.importAsset(projectRoot, selected, kind);
      alert('导入成功');
    }
  }
};

const importManuscriptDialog = async () => {
  const selected = await open({
    multiple: false,
    filters: [
      { name: 'Documents', extensions: ['txt', 'md', 'docx'] }
    ],
  });
  
  if (selected && projectRoot) {
    await commands.importManuscript(projectRoot, selected);
    await refreshTree();
    alert('导入成功');
  }
};

const exportSingleDialog = async () => {
  const format = await askExportFormat();
  if (!format || !projectRoot) return;
  
  const output = await save({
    filters: [{ name: format.toUpperCase(), extensions: [format] }],
  });
  
  if (output) {
    await commands.exportBookSingle(projectRoot, format, output);
    alert('导出成功');
  }
};

const exportTreeDialog = async () => {
  const format = await askExportFormat();
  if (!format || !projectRoot) return;
  
  const output = await open({ directory: true });
  
  if (output) {
    await commands.exportTreeMulti(projectRoot, format, output);
    alert('导出成功');
  }
};
```

---

## 7. 添加到 tauri-commands.ts

```typescript
export const commands = {
  // ... 现有命令

  importAsset: (projectRoot: string, inputPath: string, kind: string) =>
    invoke<string>('import_asset', { projectRoot, inputPath, kind }),

  importManuscript: (projectRoot: string, inputPath: string) =>
    invoke<void>('import_manuscript', { projectRoot, inputPath }),

  exportBookSingle: (projectRoot: string, format: string, outputPath: string) =>
    invoke<void>('export_book_single', { projectRoot, format, outputPath }),

  exportTreeMulti: (projectRoot: string, format: string, outputDir: string) =>
    invoke<void>('export_tree_multi', { projectRoot, format, outputDir }),
};
```

---

## 下一步

导入导出完成后，继续阅读 [06_ai_infrastructure.md](./06_ai_infrastructure.md) 完成 AI 底座开发。
