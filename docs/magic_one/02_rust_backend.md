# Rust 后端开发文档

> 本文档详细描述 Rust 后端的模块设计和实现细节。

---

## 1. 模块概览

```
src-tauri/src/
├── main.rs           # 入口
├── lib.rs            # 库导出 + tauri-specta 配置
├── commands/         # Tauri 命令（对外接口）
├── models/           # 数据模型（与前端共享类型）
├── services/         # 业务逻辑
└── utils/            # 工具函数
```

---

## 2. 数据模型 (models/)

### 2.1 错误类型 - models/error.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    PermissionDenied,
    IoError,
    JsonParseError,
    SchemaValidationError,
    SchemaVersionUnsupported,
    MigrationRequired,
    MigrationFailed,
    ImportParseFailed,
    ExportFailed,
    Conflict,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoverable: Option<bool>,
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }

    pub fn io_error(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::IoError,
            message: msg.into(),
            details: None,
            recoverable: Some(true),
        }
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidArgument,
            message: msg.into(),
            details: None,
            recoverable: Some(true),
        }
    }

    pub fn json_parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::JsonParseError,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::json_parse_error(err.to_string())
    }
}
```

### 2.2 项目元数据 - models/project.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProjectMetadata {
    pub schema_version: i32,
    pub project_id: String,
    pub name: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_min_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<i64>,
}

impl ProjectMetadata {
    pub fn new(name: String, author: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            project_id: uuid::Uuid::new_v4().to_string(),
            name,
            author,
            description: None,
            created_at: now,
            updated_at: now,
            app_min_version: None,
            last_opened_at: Some(now),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum FileNode {
    #[serde(rename = "dir")]
    Dir {
        name: String,
        path: String,
        children: Vec<FileNode>,
    },
    #[serde(rename = "chapter")]
    Chapter {
        name: String,
        path: String,
        chapter_id: String,
        title: String,
        text_length_no_whitespace: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        updated_at: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProjectSnapshot {
    pub project: ProjectMetadata,
    pub tree: Vec<FileNode>,
}
```

### 2.3 卷元数据 - models/volume.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VolumeMetadata {
    pub schema_version: i32,
    pub volume_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl VolumeMetadata {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            volume_id: uuid::Uuid::new_v4().to_string(),
            title,
            summary: None,
            created_at: now,
            updated_at: now,
        }
    }
}
```

### 2.4 章节 - models/chapter.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ChapterStatus {
    Draft,
    Revised,
    Final,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChapterCounts {
    pub text_length_no_whitespace: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<i32>,
    pub algorithm_version: i32,
    pub last_calculated_at: i64,
}

impl Default for ChapterCounts {
    fn default() -> Self {
        Self {
            text_length_no_whitespace: 0,
            word_count: None,
            algorithm_version: 1,
            last_calculated_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Chapter {
    pub schema_version: i32,
    pub id: String,
    pub title: String,
    pub content: serde_json::Value,
    pub counts: ChapterCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_words: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ChapterStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cursor_position: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Chapter {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content: serde_json::json!({
                "type": "doc",
                "content": []
            }),
            counts: ChapterCounts::default(),
            target_words: None,
            status: Some(ChapterStatus::Draft),
            summary: None,
            tags: None,
            last_cursor_position: None,
            created_at: now,
            updated_at: now,
        }
    }
}
```

### 2.5 资产树 - models/asset.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Lore,
    Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AssetSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    pub imported_at: i64,
    pub importer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AssetNode {
    pub node_id: String,
    pub title: String,
    pub level: i32,
    pub content: String,
    pub children: Vec<AssetNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AssetTree {
    pub schema_version: i32,
    pub id: String,
    pub kind: AssetKind,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AssetSource>,
    pub root: AssetNode,
}
```

### 2.6 AI Proposal - models/proposal.rs

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Generated,
    Accepted,
    PartiallyAccepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposalTarget {
    #[serde(rename = "type")]
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposalContextRefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lore_asset_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_asset_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposalModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposalOutput {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiptap_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AiProposal {
    pub schema_version: i32,
    pub proposal_id: String,
    pub created_at: i64,
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_path: String,
    pub target: ProposalTarget,
    pub prompt: String,
    pub context_refs: ProposalContextRefs,
    pub model: ProposalModel,
    pub output: ProposalOutput,
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op")]
pub enum PatchOp {
    #[serde(rename = "insert_blocks")]
    InsertBlocks {
        after_block_id: Option<String>,
        blocks: Vec<serde_json::Value>,
    },
    #[serde(rename = "update_block")]
    UpdateBlock {
        block_id: String,
        before: serde_json::Value,
        after: serde_json::Value,
    },
    #[serde(rename = "delete_blocks")]
    DeleteBlocks {
        block_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChapterHistoryEvent {
    pub schema_version: i32,
    pub event_id: String,
    pub created_at: i64,
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_proposal_id: Option<String>,
    pub before_hash: String,
    pub after_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub patch: Vec<PatchOp>,
}
```

### 2.7 模块导出 - models/mod.rs

```rust
pub mod asset;
pub mod chapter;
pub mod error;
pub mod project;
pub mod proposal;
pub mod volume;

pub use asset::*;
pub use chapter::*;
pub use error::*;
pub use project::*;
pub use proposal::*;
pub use volume::*;
```

---

## 3. 工具函数 (utils/)

### 3.1 原子写入 - utils/atomic_write.rs

```rust
use std::fs;
use std::path::Path;

use crate::models::AppError;

pub fn atomic_write<P: AsRef<Path>>(path: P, content: &[u8]) -> Result<(), AppError> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");

    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)?;

    Ok(())
}

pub fn atomic_write_json<P: AsRef<Path>, T: serde::Serialize>(
    path: P,
    data: &T,
) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(data)?;
    atomic_write(path, json.as_bytes())
}
```

### 3.2 模块导出 - utils/mod.rs

```rust
pub mod atomic_write;
pub use atomic_write::*;
```

---

## 4. 业务服务 (services/)

### 4.1 文件系统操作 - services/file_system.rs

```rust
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::*;

pub fn ensure_project_structure(project_root: &Path) -> Result<(), AppError> {
    let magic_novel = project_root.join("magic_novel");
    let content = project_root.join("content");

    fs::create_dir_all(&magic_novel)?;
    fs::create_dir_all(magic_novel.join("lore"))?;
    fs::create_dir_all(magic_novel.join("prompts"))?;
    fs::create_dir_all(magic_novel.join("ai/proposals"))?;
    fs::create_dir_all(magic_novel.join("history/chapters"))?;
    fs::create_dir_all(magic_novel.join("backups"))?;
    fs::create_dir_all(&content)?;

    Ok(())
}

pub fn scan_content_tree(content_dir: &Path, base_path: &Path) -> Result<Vec<FileNode>, AppError> {
    let mut nodes = Vec::new();
    let mut dirs = Vec::new();
    let mut chapters = Vec::new();

    if !content_dir.exists() {
        return Ok(nodes);
    }

    for entry in fs::read_dir(content_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && !name.starts_with('.') {
            let volume_meta_path = path.join("_volume.json");
            let children = scan_content_tree(&path, base_path)?;

            dirs.push(FileNode::Dir {
                name: name.clone(),
                path: path.strip_prefix(base_path).unwrap().to_string_lossy().to_string(),
                children,
            });
        } else if path.extension().map_or(false, |ext| ext == "json") && name != "_volume.json" {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(chapter) = serde_json::from_str::<Chapter>(&content) {
                    chapters.push(FileNode::Chapter {
                        name: name.clone(),
                        path: path.strip_prefix(base_path).unwrap().to_string_lossy().to_string(),
                        chapter_id: chapter.id,
                        title: chapter.title,
                        text_length_no_whitespace: chapter.counts.text_length_no_whitespace,
                        status: chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
                        updated_at: chapter.updated_at,
                    });
                }
            }
        }
    }

    dirs.sort_by(|a, b| {
        if let (FileNode::Dir { name: a_name, .. }, FileNode::Dir { name: b_name, .. }) = (a, b) {
            a_name.cmp(b_name)
        } else {
            std::cmp::Ordering::Equal
        }
    });

    chapters.sort_by(|a, b| {
        if let (FileNode::Chapter { name: a_name, .. }, FileNode::Chapter { name: b_name, .. }) = (a, b) {
            a_name.cmp(b_name)
        } else {
            std::cmp::Ordering::Equal
        }
    });

    nodes.extend(dirs);
    nodes.extend(chapters);

    Ok(nodes)
}
```

### 4.2 字数统计 - services/word_count.rs

```rust
use serde_json::Value;

pub fn extract_plain_text(doc: &Value) -> String {
    let mut texts = Vec::new();
    walk_node(doc, &mut texts);
    texts.join("")
}

fn walk_node(node: &Value, texts: &mut Vec<String>) {
    if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
        if node_type == "text" {
            if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
                texts.push(text.to_string());
            }
        }
    }

    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            walk_node(child, texts);
        }
    }
}

pub fn count_chars_no_whitespace(plain_text: &str) -> i32 {
    plain_text
        .chars()
        .filter(|c| !c.is_whitespace())
        .count() as i32
}

pub fn calculate_counts(doc: &Value) -> (i32, i32) {
    let plain_text = extract_plain_text(doc);
    let char_count = count_chars_no_whitespace(&plain_text);
    let word_count = plain_text
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .count() as i32;
    (char_count, word_count)
}
```

### 4.3 模块导出 - services/mod.rs

```rust
pub mod file_system;
pub mod word_count;

pub use file_system::*;
pub use word_count::*;
```

---

## 5. Tauri 命令 (commands/)

### 5.1 项目命令 - commands/project.rs

```rust
use std::fs;
use std::path::PathBuf;

use tauri::command;
use specta::specta;

use crate::models::*;
use crate::services::*;
use crate::utils::*;

#[command]
#[specta]
pub async fn create_project(
    library_root: String,
    project_folder_name: String,
    name: String,
    author: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&library_root).join(&project_folder_name);

    if project_root.exists() {
        return Err(AppError::invalid_argument("项目目录已存在"));
    }

    fs::create_dir_all(&project_root)?;
    ensure_project_structure(&project_root)?;

    let metadata = ProjectMetadata::new(name, author);
    let project_json_path = project_root.join("magic_novel/project.json");
    atomic_write_json(&project_json_path, &metadata)?;

    Ok(project_root.to_string_lossy().to_string())
}

#[command]
#[specta]
pub async fn open_project(project_root: String) -> Result<ProjectSnapshot, AppError> {
    let project_root = PathBuf::from(&project_root);

    if !project_root.exists() {
        return Err(AppError::not_found("项目目录不存在"));
    }

    let project_json_path = project_root.join("magic_novel/project.json");
    if !project_json_path.exists() {
        return Err(AppError::not_found("找不到 project.json"));
    }

    let content = fs::read_to_string(&project_json_path)?;
    let mut project: ProjectMetadata = serde_json::from_str(&content)?;

    project.last_opened_at = Some(chrono::Utc::now().timestamp_millis());
    atomic_write_json(&project_json_path, &project)?;

    let content_dir = project_root.join("content");
    let tree = scan_content_tree(&content_dir, &project_root)?;

    Ok(ProjectSnapshot { project, tree })
}

#[command]
#[specta]
pub async fn set_library_root(path: String) -> Result<(), AppError> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(())
}
```

### 5.2 章节命令 - commands/chapter.rs

```rust
use std::fs;
use std::path::PathBuf;

use tauri::command;
use specta::specta;

use crate::models::*;
use crate::services::*;
use crate::utils::*;

#[command]
#[specta]
pub async fn create_chapter(
    project_root: String,
    volume_path: String,
    file_name: String,
    title: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&project_root);
    let volume_dir = project_root.join(&volume_path);

    if !volume_dir.exists() {
        return Err(AppError::not_found("卷目录不存在"));
    }

    let chapter = Chapter::new(title);
    let chapter_path = volume_dir.join(format!("{}.json", file_name));

    if chapter_path.exists() {
        return Err(AppError::invalid_argument("章节文件已存在"));
    }

    atomic_write_json(&chapter_path, &chapter)?;

    Ok(chapter.id)
}

#[command]
#[specta]
pub async fn read_chapter(project_root: String, path: String) -> Result<Chapter, AppError> {
    let full_path = PathBuf::from(&project_root).join(&path);

    if !full_path.exists() {
        return Err(AppError::not_found("章节文件不存在"));
    }

    let content = fs::read_to_string(&full_path)?;
    let chapter: Chapter = serde_json::from_str(&content)?;

    Ok(chapter)
}

#[command]
#[specta]
pub async fn save_chapter(
    project_root: String,
    path: String,
    mut data: Chapter,
) -> Result<(), AppError> {
    let full_path = PathBuf::from(&project_root).join(&path);

    let (char_count, word_count) = calculate_counts(&data.content);
    data.counts.text_length_no_whitespace = char_count;
    data.counts.word_count = Some(word_count);
    data.counts.last_calculated_at = chrono::Utc::now().timestamp_millis();
    data.updated_at = chrono::Utc::now().timestamp_millis();

    atomic_write_json(&full_path, &data)?;

    Ok(())
}

#[command]
#[specta]
pub async fn delete_chapter(project_root: String, path: String) -> Result<(), AppError> {
    let full_path = PathBuf::from(&project_root).join(&path);

    if !full_path.exists() {
        return Err(AppError::not_found("章节文件不存在"));
    }

    fs::remove_file(&full_path)?;

    Ok(())
}

#[command]
#[specta]
pub async fn rename_chapter(
    project_root: String,
    old_path: String,
    new_name: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&project_root);
    let old_full_path = project_root.join(&old_path);

    if !old_full_path.exists() {
        return Err(AppError::not_found("章节文件不存在"));
    }

    let parent = old_full_path.parent().ok_or_else(|| AppError::internal("无法获取父目录"))?;
    let new_full_path = parent.join(format!("{}.json", new_name));

    if new_full_path.exists() {
        return Err(AppError::invalid_argument("目标文件名已存在"));
    }

    fs::rename(&old_full_path, &new_full_path)?;

    let new_path = new_full_path.strip_prefix(&project_root)
        .map_err(|_| AppError::internal("路径处理失败"))?
        .to_string_lossy()
        .to_string();

    Ok(new_path)
}
```

### 5.3 卷命令 - commands/volume.rs

```rust
use std::fs;
use std::path::PathBuf;

use tauri::command;
use specta::specta;

use crate::models::*;
use crate::utils::*;

#[command]
#[specta]
pub async fn create_volume(
    project_root: String,
    parent_path: String,
    folder_name: String,
    title: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&project_root);
    let parent_dir = project_root.join(&parent_path);
    let volume_dir = parent_dir.join(&folder_name);

    if volume_dir.exists() {
        return Err(AppError::invalid_argument("卷目录已存在"));
    }

    fs::create_dir_all(&volume_dir)?;

    let volume = VolumeMetadata::new(title);
    let volume_json_path = volume_dir.join("_volume.json");
    atomic_write_json(&volume_json_path, &volume)?;

    Ok(volume.volume_id)
}

#[command]
#[specta]
pub async fn delete_volume(project_root: String, path: String) -> Result<(), AppError> {
    let full_path = PathBuf::from(&project_root).join(&path);

    if !full_path.exists() {
        return Err(AppError::not_found("卷目录不存在"));
    }

    fs::remove_dir_all(&full_path)?;

    Ok(())
}

#[command]
#[specta]
pub async fn rename_volume(
    project_root: String,
    old_path: String,
    new_name: String,
) -> Result<String, AppError> {
    let project_root = PathBuf::from(&project_root);
    let old_full_path = project_root.join(&old_path);

    if !old_full_path.exists() {
        return Err(AppError::not_found("卷目录不存在"));
    }

    let parent = old_full_path.parent().ok_or_else(|| AppError::internal("无法获取父目录"))?;
    let new_full_path = parent.join(&new_name);

    if new_full_path.exists() {
        return Err(AppError::invalid_argument("目标目录名已存在"));
    }

    fs::rename(&old_full_path, &new_full_path)?;

    let new_path = new_full_path.strip_prefix(&project_root)
        .map_err(|_| AppError::internal("路径处理失败"))?
        .to_string_lossy()
        .to_string();

    Ok(new_path)
}
```

### 5.4 AI 命令 - commands/ai.rs

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use tauri::command;
use specta::specta;

use crate::models::*;
use crate::utils::*;

#[command]
#[specta]
pub async fn save_ai_proposal(
    project_root: String,
    proposal: AiProposal,
) -> Result<(), AppError> {
    let project_root = PathBuf::from(&project_root);
    let proposals_dir = project_root.join("magic_novel/ai/proposals");
    fs::create_dir_all(&proposals_dir)?;

    let proposal_path = proposals_dir.join(format!("{}.json", proposal.proposal_id));
    atomic_write_json(&proposal_path, &proposal)?;

    Ok(())
}

#[command]
#[specta]
pub async fn append_chapter_history_event(
    project_root: String,
    chapter_id: String,
    event: ChapterHistoryEvent,
) -> Result<(), AppError> {
    let project_root = PathBuf::from(&project_root);
    let history_dir = project_root.join("magic_novel/history/chapters");
    fs::create_dir_all(&history_dir)?;

    let history_path = history_dir.join(format!("{}.jsonl", chapter_id));
    let event_json = serde_json::to_string(&event)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_path)?;

    writeln!(file, "{}", event_json)?;

    Ok(())
}
```

### 5.5 模块导出 - commands/mod.rs

```rust
pub mod ai;
pub mod chapter;
pub mod project;
pub mod volume;

pub use ai::*;
pub use chapter::*;
pub use project::*;
pub use volume::*;
```

---

## 6. 入口配置

### 6.1 lib.rs

```rust
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

mod commands;
mod models;
mod services;
mod utils;

pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::set_library_root,
            commands::create_project,
            commands::open_project,
            commands::create_volume,
            commands::delete_volume,
            commands::rename_volume,
            commands::create_chapter,
            commands::read_chapter,
            commands::save_chapter,
            commands::delete_chapter,
            commands::rename_chapter,
            commands::save_ai_proposal,
            commands::append_chapter_history_event,
        ]);

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/lib/tauri-bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(builder.into_plugin())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 6.2 main.rs

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    magic_novel::run()
}
```

---

## 7. 添加依赖 chrono

在 Cargo.toml 中添加：

```toml
chrono = { version = "0.4", features = ["serde"] }
```

---

## 下一步

Rust 后端完成后，继续阅读 [03_frontend_editor.md](./03_frontend_editor.md) 开始前端编辑器开发。
