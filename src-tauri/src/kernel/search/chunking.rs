#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TextChunk {
    pub start_char: usize,
    pub end_char: usize,
    pub text: String,
}

pub fn chunk_text(
    text: &str,
    target_chars: usize,
    overlap_chars: usize,
    max_chunk_chars: usize,
) -> Vec<TextChunk> {
    let normalized = text.replace("\r\n", "\n");
    let positions = char_positions(&normalized);

    let total_chars = positions.len().saturating_sub(1);
    if total_chars == 0 || target_chars == 0 {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < total_chars {
        let mut end = (start + target_chars).min(total_chars);
        let max_end = (start + max_chunk_chars).min(total_chars);
        if end > max_end {
            end = max_end;
        }

        let start_byte = positions[start];
        let end_byte = positions[end];

        let slice = normalized
            .get(start_byte..end_byte)
            .unwrap_or_default()
            .trim()
            .to_string();

        if !slice.is_empty() {
            chunks.push(TextChunk {
                start_char: start,
                end_char: end,
                text: slice,
            });
        }

        if end >= total_chars {
            break;
        }

        start = end.saturating_sub(overlap_chars);
    }

    chunks
}

fn char_positions(text: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    for (i, _) in text.char_indices() {
        positions.push(i);
    }
    positions.push(text.len());
    positions
}
