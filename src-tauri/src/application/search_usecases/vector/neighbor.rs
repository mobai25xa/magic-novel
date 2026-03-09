use std::collections::HashMap;

use crate::application::search_usecases::index::types::ChunkRecord;

const EXPANDED_SNIPPET_LIMIT: usize = 1200;

pub fn expand_with_neighbors(chunks: &[ChunkRecord], chunk_idx: usize) -> String {
    let Some(current) = chunks.get(chunk_idx) else {
        return String::new();
    };

    let position_map = build_chunk_position_map(chunks);
    let (source_path, seq) =
        parse_chunk_id(&current.chunk_id).unwrap_or_else(|| (current.source_path.clone(), 0usize));

    let mut parts = Vec::new();

    if let Some(prev_idx) = position_map.get(&(source_path.clone(), seq.saturating_sub(1))) {
        if let Some(prev) = chunks.get(*prev_idx) {
            parts.push(prev.text.as_str());
        }
    }

    parts.push(current.text.as_str());

    if let Some(next_idx) = position_map.get(&(source_path.clone(), seq + 1)) {
        if let Some(next) = chunks.get(*next_idx) {
            parts.push(next.text.as_str());
        }
    }

    let joined = parts.join("\n");
    if joined.chars().count() > EXPANDED_SNIPPET_LIMIT {
        return joined.chars().take(EXPANDED_SNIPPET_LIMIT).collect();
    }

    joined
}

pub fn parse_chunk_id(chunk_id: &str) -> Option<(String, usize)> {
    let mut parts = chunk_id.rsplitn(2, ':');
    let idx_part = parts.next()?;
    let source_part = parts.next()?;

    let seq = idx_part.parse::<usize>().ok()?;
    let mut source_sections = source_part.splitn(2, ':');
    let _source_kind = source_sections.next()?;
    let source_path = source_sections.next()?.to_string();

    Some((source_path, seq))
}

fn build_chunk_position_map(chunks: &[ChunkRecord]) -> HashMap<(String, usize), usize> {
    let mut map = HashMap::new();

    for (idx, chunk) in chunks.iter().enumerate() {
        if let Some((source_path, seq)) = parse_chunk_id(&chunk.chunk_id) {
            map.insert((source_path, seq), idx);
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::expand_with_neighbors;
    use super::parse_chunk_id;
    use crate::application::search_usecases::index::types::ChunkRecord;

    fn chunk(chunk_id: &str, source_path: &str, text: &str) -> ChunkRecord {
        ChunkRecord {
            schema_version: 1,
            chunk_id: chunk_id.to_string(),
            source_kind: "chapter".to_string(),
            source_path: source_path.to_string(),
            title: "title".to_string(),
            text: text.to_string(),
            text_len: text.chars().count() as u32,
        }
    }

    #[test]
    fn parse_chunk_id_extracts_source_and_seq() {
        let parsed = parse_chunk_id("chapter:vol_001/chap_a.json:3");
        assert_eq!(parsed, Some(("vol_001/chap_a.json".to_string(), 3)));
    }

    #[test]
    fn expand_with_neighbors_returns_prev_current_next() {
        let chunks = vec![
            chunk(
                "chapter:vol_001/chap_a.json:0",
                "vol_001/chap_a.json",
                "prev",
            ),
            chunk(
                "chapter:vol_001/chap_a.json:1",
                "vol_001/chap_a.json",
                "cur",
            ),
            chunk(
                "chapter:vol_001/chap_a.json:2",
                "vol_001/chap_a.json",
                "next",
            ),
        ];

        let expanded = expand_with_neighbors(&chunks, 1);
        assert!(expanded.contains("prev"));
        assert!(expanded.contains("cur"));
        assert!(expanded.contains("next"));
    }
}
