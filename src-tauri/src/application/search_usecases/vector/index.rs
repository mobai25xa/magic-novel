use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::json;

use crate::kernel::search::embedding::{embed_texts_openai_compatible, EmbeddingProviderConfig};
use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, load_openai_search_settings};

use crate::application::search_usecases::bm25::build::{ensure_bm25_index, load_bm25_index};
use crate::application::search_usecases::index::corpus::{fingerprint_corpus, scan_corpus};
use crate::application::search_usecases::index::paths::{
    index_root, vec_manifest_path, vecs_f32_path, vectors_dir,
};
use crate::application::search_usecases::index::types::ChunkRecord;

use super::io::{
    read_f32_le_vec, read_vector_manifest, write_f32_le_vec_atomic, write_vector_manifest,
};
use super::types::{VectorCorpusMeta, VectorEmbeddingMeta, VectorManifest};

const EMBEDDING_BATCH_SIZE: usize = 32;

#[derive(Clone)]
pub struct LoadedVectorIndex {
    pub fingerprint: String,
    pub dims: usize,
    pub chunks: Arc<Vec<ChunkRecord>>,
    pub vectors: Arc<Vec<f32>>,
}

static VECTOR_CACHE: OnceLock<Mutex<HashMap<String, LoadedVectorIndex>>> = OnceLock::new();

pub fn ensure_vector_index(project_path: &str) -> Result<LoadedVectorIndex, AppError> {
    let settings = load_openai_search_settings()?;
    ensure_embedding_search_enabled(&settings)?;

    let corpus = scan_corpus(project_path)?;
    let fingerprint = fingerprint_corpus(&corpus);

    if let Some(cached) = get_cached(project_path, &fingerprint) {
        return Ok(cached);
    }

    let root = index_root(project_path);
    let manifest_path = vec_manifest_path(&root);
    let vecs_path = vecs_f32_path(&root);

    if let (Ok(manifest), Ok(vectors), Ok((chunks, _))) = (
        read_vector_manifest(&manifest_path),
        read_f32_le_vec(&vecs_path),
        load_bm25_index(project_path),
    ) {
        if manifest.corpus.fingerprint == fingerprint
            && manifest.corpus.chunk_count == chunks.len()
            && manifest.embedding.dims > 0
            && vectors.len() == chunks.len().saturating_mul(manifest.embedding.dims)
        {
            let loaded = LoadedVectorIndex {
                fingerprint,
                dims: manifest.embedding.dims,
                chunks: Arc::new(chunks),
                vectors: Arc::new(vectors),
            };

            put_cached(project_path, loaded.clone());
            return Ok(loaded);
        }
    }

    let rebuilt = rebuild_vector_index(project_path, &fingerprint)?;
    put_cached(project_path, rebuilt.clone());
    Ok(rebuilt)
}

pub fn query_vector_topn(
    query_vector: &[f32],
    loaded: &LoadedVectorIndex,
    scope_mask: Option<&[bool]>,
    top_n: usize,
) -> Result<Vec<(usize, f64)>, AppError> {
    if query_vector.len() != loaded.dims {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: format!(
                "向量维度不匹配: query={}, index={}",
                query_vector.len(),
                loaded.dims
            ),
            details: Some(json!({ "code": "E_SEARCH_VECTORS_DIMS_MISMATCH" })),
            recoverable: Some(false),
        });
    }

    let chunk_count = loaded.chunks.len();
    let mut scored = Vec::with_capacity(chunk_count);

    for chunk_idx in 0..chunk_count {
        if let Some(mask) = scope_mask {
            if !mask.get(chunk_idx).copied().unwrap_or(false) {
                continue;
            }
        }

        let start = chunk_idx * loaded.dims;
        let end = start + loaded.dims;
        let Some(vector) = loaded.vectors.get(start..end) else {
            continue;
        };

        let mut dot = 0.0f64;
        for i in 0..loaded.dims {
            dot += (query_vector[i] as f64) * (vector[i] as f64);
        }

        scored.push((chunk_idx, dot));
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_n);
    Ok(scored)
}

pub fn embed_query(project_path: &str, query: &str) -> Result<Vec<f32>, AppError> {
    let settings = load_openai_search_settings()?;
    ensure_embedding_search_enabled(&settings)?;
    validate_embedding_settings(&settings)?;

    let _ = project_path;

    let config = build_embedding_provider_config(&settings);

    let vectors = embed_texts_openai_compatible(&config, &[query.to_string()])?;
    vectors.into_iter().next().ok_or_else(|| AppError {
        code: ErrorCode::Internal,
        message: "查询向量为空".to_string(),
        details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
        recoverable: Some(true),
    })
}

fn rebuild_vector_index(
    project_path: &str,
    fingerprint: &str,
) -> Result<LoadedVectorIndex, AppError> {
    ensure_bm25_index(project_path)?;
    let (chunks, _) = load_bm25_index(project_path)?;

    let settings = load_openai_search_settings()?;
    validate_embedding_settings(&settings)?;

    let config = build_embedding_provider_config(&settings);

    let dims = build_vector_files(project_path, &chunks, fingerprint, &config)?;
    let root = index_root(project_path);
    let vectors = read_f32_le_vec(&vecs_f32_path(&root))?;

    if vectors.len() != chunks.len().saturating_mul(dims) {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "向量索引维度与块数量不一致".to_string(),
            details: Some(json!({ "code": "E_SEARCH_VECTORS_INDEX_MISSING" })),
            recoverable: Some(true),
        });
    }

    Ok(LoadedVectorIndex {
        fingerprint: fingerprint.to_string(),
        dims,
        chunks: Arc::new(chunks),
        vectors: Arc::new(vectors),
    })
}

fn build_vector_files(
    project_path: &str,
    chunks: &[ChunkRecord],
    fingerprint: &str,
    config: &EmbeddingProviderConfig,
) -> Result<usize, AppError> {
    let mut all_values = Vec::new();
    let mut dims = 0usize;

    for batch in chunks.chunks(EMBEDDING_BATCH_SIZE) {
        let texts: Vec<String> = batch.iter().map(|item| item.text.clone()).collect();
        let vectors = embed_texts_openai_compatible(config, &texts)?;

        for vector in vectors {
            if dims == 0 {
                dims = vector.len();
            } else if vector.len() != dims {
                return Err(AppError {
                    code: ErrorCode::Internal,
                    message: format!("Embedding 维度不一致: expect={dims}, got={}", vector.len()),
                    details: Some(json!({ "code": "E_SEARCH_VECTORS_DIMS_MISMATCH" })),
                    recoverable: Some(false),
                });
            }
            all_values.extend(vector);
        }
    }

    if dims == 0 {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "无法构建空向量索引".to_string(),
            details: Some(json!({ "code": "E_SEARCH_VECTORS_INDEX_MISSING" })),
            recoverable: Some(true),
        });
    }

    let root = index_root(project_path);
    let vectors_dir_path = vectors_dir(&root);
    ensure_dir(&vectors_dir_path)?;

    write_f32_le_vec_atomic(&vecs_f32_path(&root), &all_values)?;

    let manifest = VectorManifest {
        schema_version: 1,
        created_at: chrono::Utc::now().timestamp_millis(),
        embedding: VectorEmbeddingMeta {
            provider: "openai-compatible".to_string(),
            model: config.model.clone(),
            dims,
            metric: "cosine".to_string(),
            normalized: true,
        },
        corpus: VectorCorpusMeta {
            fingerprint: fingerprint.to_string(),
            chunk_count: chunks.len(),
        },
    };

    write_vector_manifest(&vec_manifest_path(&root), &manifest)?;
    Ok(dims)
}

pub fn ensure_embedding_search_enabled(
    settings: &crate::services::OpenAiSearchSettings,
) -> Result<(), AppError> {
    if !settings.openai_embedding_detected {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "Embedding 模型未检测到，语义检索不可用".to_string(),
            details: Some(json!({
                "code": "E_AI_SETTINGS_EMBEDDING_UNAVAILABLE",
                "reason": if settings.openai_embedding_detection_reason.trim().is_empty() {
                    "embedding_model_unavailable"
                } else {
                    settings.openai_embedding_detection_reason.trim()
                }
            })),
            recoverable: Some(true),
        });
    }

    if !settings.openai_embedding_enabled {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "Embedding 服务开关已关闭，语义检索不可用".to_string(),
            details: Some(json!({
                "code": "E_AI_SETTINGS_EMBEDDING_DISABLED",
                "reason": "embedding_disabled"
            })),
            recoverable: Some(true),
        });
    }

    Ok(())
}

fn validate_embedding_settings(
    settings: &crate::services::OpenAiSearchSettings,
) -> Result<(), AppError> {
    let source = settings.openai_embedding_source.trim();
    let is_local = source == "local";

    let base_url = if is_local {
        settings.openai_local_embedding_base_url.trim()
    } else {
        settings.openai_embedding_base_url.trim()
    };

    let api_key = if is_local {
        settings.openai_local_embedding_api_key.trim()
    } else {
        settings.openai_embedding_api_key.trim()
    };

    if base_url.is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: if is_local {
                "Local embedding baseUrl 未配置".to_string()
            } else {
                "Embedding baseUrl 未配置".to_string()
            },
            details: Some(json!({ "code": "E_AI_SETTINGS_MISSING_BASEURL" })),
            recoverable: Some(true),
        });
    }

    if !is_local && api_key.is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "Embedding apiKey 未配置".to_string(),
            details: Some(json!({ "code": "E_AI_SETTINGS_MISSING_APIKEY" })),
            recoverable: Some(true),
        });
    }

    if settings.openai_embedding_model.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "OpenAI embedding model 未配置".to_string(),
            details: Some(json!({ "code": "E_AI_SETTINGS_MISSING_EMBEDDING_MODEL" })),
            recoverable: Some(true),
        });
    }

    Ok(())
}

fn build_embedding_provider_config(
    settings: &crate::services::OpenAiSearchSettings,
) -> EmbeddingProviderConfig {
    let is_local = settings.openai_embedding_source.trim() == "local";

    let base_url = if is_local {
        settings.openai_local_embedding_base_url.trim()
    } else {
        settings.openai_embedding_base_url.trim()
    };

    let api_key = if is_local {
        settings.openai_local_embedding_api_key.trim()
    } else {
        settings.openai_embedding_api_key.trim()
    };

    EmbeddingProviderConfig {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        model: settings.openai_embedding_model.clone(),
    }
}

fn get_cached(project_path: &str, fingerprint: &str) -> Option<LoadedVectorIndex> {
    let cache = VECTOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = cache.lock().ok()?;
    let entry = guard.get(project_path)?;
    if entry.fingerprint == fingerprint {
        return Some(entry.clone());
    }
    None
}

fn put_cached(project_path: &str, loaded: LoadedVectorIndex) {
    let cache = VECTOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(project_path.to_string(), loaded);
    }
}
