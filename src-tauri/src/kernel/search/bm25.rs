use std::collections::HashMap;
use std::sync::Arc;

use super::tokenizer::tokenize_cjk_bigram;

#[derive(Debug, Clone, Copy)]
pub struct Bm25Params {
    pub k1: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct TermDictEntry {
    pub df: u32,
    pub offset_bytes: u64,
    pub len: u32,
}

#[derive(Debug, Clone)]
pub struct Bm25Index {
    pub params: Bm25Params,
    pub doc_lens: Vec<u32>,
    pub avgdl: f64,
    pub dict: HashMap<String, TermDictEntry>,
    pub postings: Arc<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Posting {
    pub doc_id: u32,
    pub tf: u32,
}

impl Bm25Index {
    pub fn build(texts: &[Arc<String>], params: Bm25Params) -> Self {
        let mut doc_lens = Vec::with_capacity(texts.len());
        let mut postings_by_term: HashMap<String, Vec<Posting>> = HashMap::new();

        for (doc_id, text) in texts.iter().enumerate() {
            let tokens = tokenize_cjk_bigram(text);
            let doc_len = tokens.len() as u32;
            doc_lens.push(doc_len);

            let mut tf_map: HashMap<String, u32> = HashMap::new();
            for token in tokens {
                *tf_map.entry(token).or_insert(0) += 1;
            }

            for (term, tf) in tf_map {
                postings_by_term.entry(term).or_default().push(Posting {
                    doc_id: doc_id as u32,
                    tf,
                });
            }
        }

        let avgdl = if doc_lens.is_empty() {
            0.0
        } else {
            doc_lens.iter().map(|v| *v as f64).sum::<f64>() / (doc_lens.len() as f64)
        };

        let (dict, postings) = build_dict_and_postings(postings_by_term);

        Self {
            params,
            doc_lens,
            avgdl,
            dict,
            postings: Arc::new(postings),
        }
    }

    pub fn score_query(&self, query: &str, scope_mask: Option<&[bool]>) -> HashMap<u32, f64> {
        let mut qtf: HashMap<String, u32> = HashMap::new();
        for term in tokenize_cjk_bigram(query) {
            *qtf.entry(term).or_insert(0) += 1;
        }

        let doc_count = self.doc_lens.len() as f64;
        let mut scores: HashMap<u32, f64> = HashMap::new();

        for (term, qf) in qtf {
            let Some(entry) = self.dict.get(&term) else {
                continue;
            };

            let df = entry.df as f64;
            if df <= 0.0 {
                continue;
            }

            let idf = ((doc_count - df + 0.5) / (df + 0.5) + 1.0).ln();
            let postings = self.postings_for(*entry);

            for posting in postings {
                let doc_id = posting.doc_id;
                if let Some(mask) = scope_mask {
                    if mask.get(doc_id as usize).copied() == Some(false) {
                        continue;
                    }
                }

                let doc_len = self.doc_lens.get(doc_id as usize).copied().unwrap_or(0) as f64;
                let tf = posting.tf as f64;
                let denom = tf
                    + self.params.k1
                        * (1.0 - self.params.b + self.params.b * doc_len / self.avgdl.max(1.0));
                let s = idf * (tf * (self.params.k1 + 1.0) / denom) * (qf as f64);

                *scores.entry(doc_id).or_insert(0.0) += s;
            }
        }

        scores
    }

    fn postings_for(&self, entry: TermDictEntry) -> impl Iterator<Item = Posting> + '_ {
        let start = entry.offset_bytes as usize;
        let end = start.saturating_add(entry.len as usize * 8);
        let slice = self.postings.get(start..end).unwrap_or(&[]);

        slice.chunks_exact(8).map(|chunk| {
            let doc_id = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let tf = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            Posting { doc_id, tf }
        })
    }
}

fn build_dict_and_postings(
    mut postings_by_term: HashMap<String, Vec<Posting>>,
) -> (HashMap<String, TermDictEntry>, Vec<u8>) {
    let mut terms: Vec<String> = postings_by_term.keys().cloned().collect();
    terms.sort();

    let mut postings_bytes: Vec<u8> = Vec::new();
    let mut dict: HashMap<String, TermDictEntry> = HashMap::new();

    for term in terms {
        let list = postings_by_term.remove(&term).unwrap_or_default();
        let offset = postings_bytes.len() as u64;
        let len = list.len() as u32;

        for posting in list {
            postings_bytes.extend_from_slice(&posting.doc_id.to_le_bytes());
            postings_bytes.extend_from_slice(&posting.tf.to_le_bytes());
        }

        dict.insert(
            term,
            TermDictEntry {
                df: len,
                offset_bytes: offset,
                len,
            },
        );
    }

    (dict, postings_bytes)
}
