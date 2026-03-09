use std::path::{Path, PathBuf};

const MAGIC_NOVEL_DIR: &str = "magic_novel";
const AI_DIR: &str = "ai";
const SEARCH_DIR: &str = "search";

pub fn index_root(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(SEARCH_DIR)
}

pub fn manifest_path(index_root: &Path) -> PathBuf {
    index_root.join("manifest.json")
}

pub fn corpus_path(index_root: &Path) -> PathBuf {
    index_root.join("corpus.json")
}

pub fn chunks_path(index_root: &Path) -> PathBuf {
    index_root.join("chunks.jsonl")
}

pub fn bm25_dir(index_root: &Path) -> PathBuf {
    index_root.join("bm25")
}

pub fn bm25_dict_path(index_root: &Path) -> PathBuf {
    bm25_dir(index_root).join("dict.json")
}

pub fn bm25_postings_path(index_root: &Path) -> PathBuf {
    bm25_dir(index_root).join("postings.bin")
}

pub fn bm25_stats_path(index_root: &Path) -> PathBuf {
    bm25_dir(index_root).join("doc_stats.json")
}

pub fn bm25_doc_lens_path(index_root: &Path) -> PathBuf {
    bm25_dir(index_root).join("doc_lens.bin")
}

pub fn vectors_dir(index_root: &Path) -> PathBuf {
    index_root.join("vectors")
}

pub fn vec_manifest_path(index_root: &Path) -> PathBuf {
    vectors_dir(index_root).join("vec_manifest.json")
}

pub fn vecs_f32_path(index_root: &Path) -> PathBuf {
    vectors_dir(index_root).join("vecs.f32")
}

pub fn locks_dir(index_root: &Path) -> PathBuf {
    index_root.join("locks")
}

pub fn index_lock_path(index_root: &Path) -> PathBuf {
    locks_dir(index_root).join("index.lock")
}
