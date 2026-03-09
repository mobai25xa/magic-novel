use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchManifest {
    pub schema_version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub build: ManifestBuildParams,
    pub corpus: CorpusSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestBuildParams {
    pub chunk: ChunkParams,
    pub bm25: Bm25Params,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChunkParams {
    pub target_chars: usize,
    pub overlap_chars: usize,
    pub max_chunk_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Params {
    pub tokenizer: String,
    pub ngram: u32,
    pub k1: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummary {
    pub fingerprint: String,
    pub chapters: u32,
    pub assets: u32,
    pub total_chars: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusItem {
    pub kind: String,
    pub path: String,
    pub mtime_ms: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusFile {
    pub schema_version: i32,
    pub generated_at: i64,
    pub items: Vec<CorpusItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub schema_version: i32,
    pub chunk_id: String,
    pub source_kind: String,
    pub source_path: String,
    pub title: String,
    pub text: String,
    pub text_len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25IndexMeta {
    pub schema_version: i32,
    pub created_at: i64,
    pub params: Bm25Params,
    pub avgdl: f64,
    pub doc_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntryDisk {
    pub df: u32,
    pub offset_bytes: u64,
    pub len: u32,
}
