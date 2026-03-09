mod fusion;
mod index;
mod io;
mod neighbor;
mod types;

pub use fusion::merge_rrf;
pub use index::{
    embed_query, ensure_embedding_search_enabled, ensure_vector_index, query_vector_topn,
};
pub use neighbor::expand_with_neighbors;
