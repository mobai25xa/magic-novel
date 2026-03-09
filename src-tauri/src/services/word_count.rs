use crate::models::ChapterCounts;

pub fn count_text(content: &serde_json::Value) -> ChapterCounts {
    let text = extract_text(content);
    let text_no_whitespace: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let text_length = text_no_whitespace.chars().count() as i32;

    ChapterCounts {
        text_length_no_whitespace: text_length,
        word_count: Some(count_words(&text)),
        algorithm_version: 1,
        last_calculated_at: chrono::Utc::now().timestamp_millis(),
    }
}

fn extract_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => {
            if let Some(text) = obj.get("text") {
                if let Some(s) = text.as_str() {
                    return s.to_string();
                }
            }
            if let Some(content) = obj.get("content") {
                return extract_text(content);
            }
            String::new()
        }
        serde_json::Value::Array(arr) => arr.iter().map(extract_text).collect::<Vec<_>>().join(""),
        _ => String::new(),
    }
}

fn count_words(text: &str) -> i32 {
    let mut count = 0;
    let mut in_word = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if in_word {
                count += 1;
                in_word = false;
            }
        } else if is_cjk(c) {
            if in_word {
                count += 1;
                in_word = false;
            }
            count += 1;
        } else {
            in_word = true;
        }
    }

    if in_word {
        count += 1;
    }

    count
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{2B820}'..='\u{2CEAF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}' |
        '\u{3000}'..='\u{303F}' |
        '\u{3040}'..='\u{309F}' |
        '\u{30A0}'..='\u{30FF}' |
        '\u{FF00}'..='\u{FFEF}'
    )
}
