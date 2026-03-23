use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppError;
use crate::services::{list_dirs, list_files};

use super::types::{ChunkRecord, CorpusFile, CorpusItem, CorpusSummary};

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";
const ASSETS_DIR: &str = "assets";
const ASSET_FOLDER_META: &str = ".magic_folder.json";

pub fn scan_corpus(project_path: &str) -> Result<CorpusFile, AppError> {
    let mut items = vec![];
    scan_chapters(project_path, &mut items)?;
    scan_assets(project_path, &mut items)?;

    Ok(CorpusFile {
        schema_version: 1,
        generated_at: chrono::Utc::now().timestamp_millis(),
        items,
    })
}

pub fn fingerprint_corpus(corpus: &CorpusFile) -> String {
    let mut items: Vec<String> = corpus
        .items
        .iter()
        .map(|i| format!("{}:{}:{}:{}", i.kind, i.path, i.mtime_ms, i.size_bytes))
        .collect();

    items.sort();
    format!("v1:{}", items.join("|"))
}

pub fn build_corpus_summary(
    corpus: &CorpusFile,
    fingerprint: &str,
    chunks: &[ChunkRecord],
) -> CorpusSummary {
    let (chapters, assets) = corpus.items.iter().fold((0u32, 0u32), |acc, item| {
        if item.kind == "chapter" {
            (acc.0 + 1, acc.1)
        } else if item.kind == "asset" {
            (acc.0, acc.1 + 1)
        } else {
            acc
        }
    });

    let total_chars = chunks.iter().map(|c| c.text_len as u64).sum();

    CorpusSummary {
        fingerprint: fingerprint.to_string(),
        chapters,
        assets,
        total_chars,
    }
}

fn scan_chapters(project_path: &str, items: &mut Vec<CorpusItem>) -> Result<(), AppError> {
    let manuscripts_root = PathBuf::from(project_path).join(MANUSCRIPTS_DIR);
    if !manuscripts_root.exists() {
        return Ok(());
    }

    for volume_id in list_dirs(&manuscripts_root)? {
        let volume_dir = manuscripts_root.join(&volume_id);
        if !volume_dir.join(VOLUME_FILE).exists() {
            continue;
        }

        let files = list_files(&volume_dir, ".json")?;
        for file_name in files {
            if file_name == VOLUME_FILE {
                continue;
            }

            let full = volume_dir.join(&file_name);
            let rel = PathBuf::from(MANUSCRIPTS_DIR)
                .join(&volume_id)
                .join(&file_name)
                .to_string_lossy()
                .replace('\\', "/");

            items.push(CorpusItem {
                kind: "chapter".to_string(),
                path: rel,
                mtime_ms: file_modified_millis(&full),
                size_bytes: file_size_bytes(&full),
            });
        }
    }

    Ok(())
}

fn scan_assets(project_path: &str, items: &mut Vec<CorpusItem>) -> Result<(), AppError> {
    let assets_root = PathBuf::from(project_path).join(ASSETS_DIR);
    if !assets_root.exists() {
        return Ok(());
    }

    walk_assets(&assets_root, "", items)
}

fn walk_assets(root: &Path, relative: &str, items: &mut Vec<CorpusItem>) -> Result<(), AppError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        let rel = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };

        if file_type.is_dir() {
            walk_assets(&path, &rel, items)?;
            continue;
        }

        if !file_type.is_file() || name == ASSET_FOLDER_META || !name.ends_with(".json") {
            continue;
        }

        let rel_path = PathBuf::from(ASSETS_DIR)
            .join(&rel)
            .to_string_lossy()
            .replace('\\', "/");

        items.push(CorpusItem {
            kind: "asset".to_string(),
            path: rel_path,
            mtime_ms: file_modified_millis(&path),
            size_bytes: file_size_bytes(&path),
        });
    }

    Ok(())
}

fn file_modified_millis(path: &Path) -> i64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(system_time_to_millis)
        .unwrap_or(0)
}

fn file_size_bytes(path: &Path) -> u64 {
    fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0)
}

fn system_time_to_millis(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}
