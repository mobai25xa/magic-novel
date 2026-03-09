use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorManifest {
    pub schema_version: i32,
    pub created_at: i64,
    pub embedding: VectorEmbeddingMeta,
    pub corpus: VectorCorpusMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbeddingMeta {
    pub provider: String,
    pub model: String,
    pub dims: usize,
    pub metric: String,
    pub normalized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCorpusMeta {
    pub fingerprint: String,
    pub chunk_count: usize,
}
