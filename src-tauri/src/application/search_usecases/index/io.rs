use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::json;

use crate::models::AppError;
use crate::services::read_json;
use crate::utils::atomic_write::{atomic_write, atomic_write_json};

use super::types::{ChunkRecord, CorpusFile, DictEntryDisk, SearchManifest};

pub fn write_chunks_jsonl_atomic(path: &Path, chunks: &[ChunkRecord]) -> Result<(), AppError> {
    let mut lines = String::new();
    for chunk in chunks {
        lines.push_str(&serde_json::to_string(chunk).unwrap_or_else(|_| json!({}).to_string()));
        lines.push('\n');
    }

    atomic_write(path, &lines)
}

pub fn read_chunks_jsonl(path: &Path) -> Result<Vec<ChunkRecord>, AppError> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(path).map_err(AppError::from)?;
    let mut out = vec![];

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(parsed) = serde_json::from_str::<ChunkRecord>(line) {
            out.push(parsed);
        }
    }

    Ok(out)
}

pub fn read_manifest(path: &Path) -> Result<SearchManifest, AppError> {
    read_json(path)
}

pub fn write_manifest(path: &Path, manifest: &SearchManifest) -> Result<(), AppError> {
    atomic_write_json(path, manifest)
}

pub fn write_corpus(path: &Path, corpus: &CorpusFile) -> Result<(), AppError> {
    atomic_write_json(path, corpus)
}

pub fn write_dict(path: &Path, dict: &HashMap<String, DictEntryDisk>) -> Result<(), AppError> {
    atomic_write_json(path, dict)
}

pub fn write_bytes_atomic(path: &Path, data: &[u8]) -> Result<(), AppError> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data).map_err(AppError::from)?;

    if path.exists() {
        let bak = path.with_extension("bak");
        if bak.exists() {
            let _ = fs::remove_file(&bak);
        }
        fs::rename(path, &bak).map_err(AppError::from)?;
    }

    fs::rename(&tmp, path).map_err(AppError::from)?;

    let bak = path.with_extension("bak");
    if bak.exists() {
        let _ = fs::remove_file(&bak);
    }

    Ok(())
}

pub fn write_u32_le_vec_atomic(path: &Path, values: &[u32]) -> Result<(), AppError> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    write_bytes_atomic(path, &bytes)
}

pub fn read_u32_le_vec(path: &Path) -> Result<Vec<u32>, AppError> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let bytes = fs::read(path).map_err(AppError::from)?;
    let mut out = Vec::with_capacity(bytes.len() / 4);

    for chunk in bytes.chunks_exact(4) {
        out.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    Ok(out)
}
