use std::collections::HashMap;
use std::sync::Arc;

use crate::kernel::search::bm25::{Bm25Index, Bm25Params as KernelBm25Params};
use crate::models::AppError;
use crate::services::ensure_dir;

use super::types::{
    Bm25IndexMeta, Bm25Params, ChunkParams, DictEntryDisk, ManifestBuildParams, SearchManifest,
};
use crate::application::search_usecases::index::chunks::{build_chunks, chunks_texts};
use crate::application::search_usecases::index::corpus::{
    build_corpus_summary, fingerprint_corpus, scan_corpus,
};
use crate::application::search_usecases::index::io::{
    read_manifest, write_bytes_atomic, write_chunks_jsonl_atomic, write_corpus, write_dict,
    write_manifest, write_u32_le_vec_atomic,
};
use crate::application::search_usecases::index::paths::{
    bm25_dir, bm25_doc_lens_path, bm25_postings_path, bm25_stats_path, chunks_path, corpus_path,
    index_root, manifest_path,
};

pub const DEFAULT_CHUNK_PARAMS: ChunkParams = ChunkParams {
    target_chars: 700,
    overlap_chars: 120,
    max_chunk_chars: 1200,
};

pub fn ensure_bm25_index(project_path: &str) -> Result<(), AppError> {
    let root = index_root(project_path);
    let manifest = manifest_path(&root);

    let corpus = scan_corpus(project_path)?;
    let fingerprint = fingerprint_corpus(&corpus);

    if let Ok(old) = read_manifest(&manifest) {
        if old.corpus.fingerprint == fingerprint {
            return Ok(());
        }
    }

    rebuild_index(project_path, corpus, fingerprint)
}

fn rebuild_index(
    project_path: &str,
    corpus: crate::application::search_usecases::index::types::CorpusFile,
    fingerprint: String,
) -> Result<(), AppError> {
    let root = index_root(project_path);
    let bm25_folder = bm25_dir(&root);
    ensure_dir(&bm25_folder)?;

    let chunks = build_chunks(project_path, &corpus.items, &DEFAULT_CHUNK_PARAMS)?;
    let texts = chunks_texts(&chunks);

    let bm25_params = KernelBm25Params { k1: 1.2, b: 0.75 };
    let index = Bm25Index::build(&texts, bm25_params);

    let now = chrono::Utc::now().timestamp_millis();

    write_index_files(&root, &chunks, &index)?;
    write_index_metadata(&root, &corpus, &fingerprint, &chunks, &index, now)?;

    Ok(())
}

fn write_index_files(
    root: &std::path::Path,
    chunks: &[crate::application::search_usecases::index::types::ChunkRecord],
    index: &Bm25Index,
) -> Result<(), AppError> {
    write_chunks_jsonl_atomic(&chunks_path(root), chunks)?;
    write_bytes_atomic(&bm25_postings_path(root), index.postings.as_ref())?;
    write_u32_le_vec_atomic(&bm25_doc_lens_path(root), &index.doc_lens)?;

    let dict_disk: HashMap<String, DictEntryDisk> = index
        .dict
        .iter()
        .map(|(term, entry)| {
            (
                term.clone(),
                DictEntryDisk {
                    df: entry.df,
                    offset_bytes: entry.offset_bytes,
                    len: entry.len,
                },
            )
        })
        .collect();

    write_dict(
        &crate::application::search_usecases::index::paths::bm25_dict_path(root),
        &dict_disk,
    )?;

    let meta = Bm25IndexMeta {
        schema_version: 1,
        created_at: chrono::Utc::now().timestamp_millis(),
        params: Bm25Params {
            tokenizer: "cjk_ngram".to_string(),
            ngram: 2,
            k1: index.params.k1,
            b: index.params.b,
        },
        avgdl: index.avgdl,
        doc_count: chunks.len() as u32,
    };

    crate::utils::atomic_write::atomic_write_json(&bm25_stats_path(root), &meta)
}

fn write_index_metadata(
    root: &std::path::Path,
    corpus: &crate::application::search_usecases::index::types::CorpusFile,
    fingerprint: &str,
    chunks: &[crate::application::search_usecases::index::types::ChunkRecord],
    index: &Bm25Index,
    now: i64,
) -> Result<(), AppError> {
    write_corpus(&corpus_path(root), corpus)?;

    let summary = build_corpus_summary(corpus, fingerprint, chunks);
    let manifest = SearchManifest {
        schema_version: 1,
        created_at: now,
        updated_at: now,
        build: ManifestBuildParams {
            chunk: DEFAULT_CHUNK_PARAMS,
            bm25: Bm25Params {
                tokenizer: "cjk_ngram".to_string(),
                ngram: 2,
                k1: index.params.k1,
                b: index.params.b,
            },
        },
        corpus: summary,
    };

    write_manifest(&manifest_path(root), &manifest)
}

pub fn load_bm25_index(
    project_path: &str,
) -> Result<
    (
        Vec<crate::application::search_usecases::index::types::ChunkRecord>,
        Bm25Index,
    ),
    AppError,
> {
    let root = index_root(project_path);
    let chunks =
        crate::application::search_usecases::index::io::read_chunks_jsonl(&chunks_path(&root))?;

    let meta: Bm25IndexMeta = crate::services::read_json(&bm25_stats_path(&root))?;
    let dict_disk: HashMap<String, DictEntryDisk> = crate::services::read_json(
        &crate::application::search_usecases::index::paths::bm25_dict_path(&root),
    )?;

    let postings =
        std::fs::read(crate::application::search_usecases::index::paths::bm25_postings_path(&root))
            .map_err(AppError::from)?;
    let doc_lens = crate::application::search_usecases::index::io::read_u32_le_vec(
        &bm25_doc_lens_path(&root),
    )?;

    let dict = dict_disk
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                crate::kernel::search::bm25::TermDictEntry {
                    df: v.df,
                    offset_bytes: v.offset_bytes,
                    len: v.len,
                },
            )
        })
        .collect();

    let index = Bm25Index {
        params: KernelBm25Params {
            k1: meta.params.k1,
            b: meta.params.b,
        },
        doc_lens,
        avgdl: meta.avgdl,
        dict,
        postings: Arc::new(postings),
    };

    Ok((chunks, index))
}
