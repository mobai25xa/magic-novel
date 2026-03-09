use regex::Regex;

#[derive(Debug, Clone)]
pub struct KeywordMatch {
    pub count: u32,
    pub start: usize,
    pub end: usize,
}

pub fn find_keyword(text: &str, query_re: &Regex, cap: u32) -> Option<KeywordMatch> {
    let mut iter = query_re.find_iter(text);
    let first = iter.next()?;

    let mut count: u32 = 1;
    for _ in iter.take(cap.saturating_sub(1) as usize) {
        count += 1;
    }

    Some(KeywordMatch {
        count,
        start: first.start(),
        end: first.end(),
    })
}

pub fn score_keyword(m: &KeywordMatch) -> f64 {
    let early_bonus = 1.0 / (1.0 + m.start as f64);
    (m.count as f64) + early_bonus
}

pub fn build_snippet(text: &str, start: usize, end: usize, context_chars: usize) -> String {
    if start >= end || end > text.len() {
        return String::new();
    }

    let before = &text[..start];
    let after = &text[end..];

    let before_start = last_n_chars_start(before, context_chars);
    let after_end = first_n_chars_end(after, context_chars);

    let mut out = String::new();
    if before_start > 0 {
        out.push_str("...");
    }
    out.push_str(&before[before_start..]);
    out.push_str(&text[start..end]);
    out.push_str(&after[..after_end]);
    if after_end < after.len() {
        out.push_str("...");
    }

    out
}

fn last_n_chars_start(s: &str, n: usize) -> usize {
    if n == 0 {
        return s.len();
    }

    s.char_indices()
        .rev()
        .nth(n.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn first_n_chars_end(s: &str, n: usize) -> usize {
    if n == 0 {
        return 0;
    }

    s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len())
}
