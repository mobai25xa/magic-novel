pub fn tokenize_cjk_bigram(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut latin = String::new();
    let mut cjk_run: Vec<char> = Vec::new();

    for c in text.chars() {
        if is_cjk(c) {
            flush_latin(&mut tokens, &mut latin);
            cjk_run.push(c);
            continue;
        }

        if c.is_ascii_alphanumeric() {
            flush_cjk(&mut tokens, &mut cjk_run);
            latin.push(c.to_ascii_lowercase());
            continue;
        }

        flush_latin(&mut tokens, &mut latin);
        flush_cjk(&mut tokens, &mut cjk_run);
    }

    flush_latin(&mut tokens, &mut latin);
    flush_cjk(&mut tokens, &mut cjk_run);

    tokens
}

fn flush_latin(tokens: &mut Vec<String>, latin: &mut String) {
    let t = latin.trim();
    if !t.is_empty() {
        tokens.push(t.to_string());
    }
    latin.clear();
}

fn flush_cjk(tokens: &mut Vec<String>, run: &mut Vec<char>) {
    if run.is_empty() {
        return;
    }

    if run.len() == 1 {
        tokens.push(run[0].to_string());
        run.clear();
        return;
    }

    for i in 0..run.len().saturating_sub(1) {
        tokens.push([run[i], run[i + 1]].iter().collect());
    }

    run.clear();
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
