//! Search Engine
//!
//! Phase 1: keyword grep (linear scan + snippet)
//! Phase 1.5: chunk-level BM25 index (disk + incremental rebuild)

pub mod bm25;
pub mod chunking;
pub mod corpus_extract;
pub mod embedding;
pub mod keyword;
pub mod tokenizer;
