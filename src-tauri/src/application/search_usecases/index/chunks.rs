use std::path::PathBuf;
use std::sync::Arc;

use crate::kernel::search::chunking::chunk_text;
use crate::kernel::search::corpus_extract::{extract_asset_text, extract_tiptap_text};
use crate::models::{AppError, AssetTree, Chapter};
use crate::services::read_json;

use super::types::{ChunkParams, ChunkRecord, CorpusItem};

const MANUSCRIPTS_PREFIX: &str = "manuscripts/";
const ASSETS_PREFIX: &str = "assets/";

pub fn chapter_source_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches(MANUSCRIPTS_PREFIX)
        .to_string()
}

pub fn asset_source_path(path: &str) -> String {
    let rel = path
        .replace('\\', "/")
        .trim_start_matches(ASSETS_PREFIX)
        .to_string();
    format!("assets/{rel}")
}

pub fn build_chunks(
    project_path: &str,
    items: &[CorpusItem],
    chunk_params: &ChunkParams,
) -> Result<Vec<ChunkRecord>, AppError> {
    let mut chunks = vec![];

    for item in items {
        match item.kind.as_str() {
            "chapter" => {
                let chapter_full = PathBuf::from(project_path).join(&item.path);
                let chapter: Chapter = read_json(&chapter_full)?;
                let raw_text = extract_tiptap_text(&chapter.content);
                push_chunk_records(
                    &mut chunks,
                    "chapter",
                    &chapter_source_path(&item.path),
                    &chapter.title,
                    &raw_text,
                    chunk_params,
                );
            }
            "asset" => {
                let asset_full = PathBuf::from(project_path).join(&item.path);
                let asset: AssetTree = read_json(&asset_full)?;
                let raw_text = extract_asset_text(&asset);
                push_chunk_records(
                    &mut chunks,
                    "asset",
                    &asset_source_path(&item.path),
                    &asset.title,
                    &raw_text,
                    chunk_params,
                );
            }
            _ => {}
        }
    }

    Ok(chunks)
}

pub fn chunks_texts(chunks: &[ChunkRecord]) -> Vec<Arc<String>> {
    chunks.iter().map(|c| Arc::new(c.text.clone())).collect()
}

fn push_chunk_records(
    chunks: &mut Vec<ChunkRecord>,
    source_kind: &str,
    source_path: &str,
    title: &str,
    raw_text: &str,
    params: &ChunkParams,
) {
    let chunked = chunk_text(
        raw_text,
        params.target_chars,
        params.overlap_chars,
        params.max_chunk_chars,
    );

    for (idx, chunk) in chunked.into_iter().enumerate() {
        chunks.push(ChunkRecord {
            schema_version: 1,
            chunk_id: format!("{source_kind}:{source_path}:{idx}"),
            source_kind: source_kind.to_string(),
            source_path: source_path.to_string(),
            title: title.to_string(),
            text_len: chunk.text.chars().count() as u32,
            text: chunk.text,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{asset_source_path, chapter_source_path};

    #[test]
    fn chapter_source_path_trims_manuscripts_prefix() {
        assert_eq!(
            chapter_source_path("manuscripts/vol1/chap.json"),
            "vol1/chap.json"
        );
    }

    #[test]
    fn asset_source_path_preserves_assets_prefix() {
        assert_eq!(
            asset_source_path("assets/characters/a.json"),
            "assets/characters/a.json"
        );
    }
}
