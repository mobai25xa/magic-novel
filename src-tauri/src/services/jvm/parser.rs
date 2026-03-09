use crate::models::jvm::Diagnostic;
use crate::models::{AppError, DiagnosticLevel};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};

#[derive(Debug, Clone)]
pub struct MdBlock {
    pub block_id: Option<String>,
    pub kind: MdBlockKind,
    pub text: String,
    pub _raw_lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdBlockKind {
    Heading { level: i32 },
    Paragraph,
    Blockquote,
    HorizontalRule,
}

pub fn parse_markdown_to_blocks(
    markdown: &str,
) -> Result<(Vec<MdBlock>, Vec<Diagnostic>), AppError> {
    ensure_markdown_within_limit(markdown)?;

    let blocks = collect_markdown_blocks(markdown);
    validate_markdown_events(markdown);

    let (blocks, diagnostics) = dedupe_duplicate_block_ids(blocks);

    Ok((blocks, diagnostics))
}

fn ensure_markdown_within_limit(markdown: &str) -> Result<(), AppError> {
    if markdown.len() > 400_000 {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "markdown too large".to_string(),
            false,
            None,
        ));
    }
    Ok(())
}

fn collect_markdown_blocks(markdown: &str) -> Vec<MdBlock> {
    let mut blocks: Vec<MdBlock> = vec![];
    let mut current_lines: Vec<String> = vec![];

    for line in markdown.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !current_lines.is_empty() {
                blocks.push(lines_to_block(&current_lines));
                current_lines.clear();
            }
            continue;
        }
        current_lines.push(trimmed.to_string());
    }

    if !current_lines.is_empty() {
        blocks.push(lines_to_block(&current_lines));
    }

    blocks
}

fn validate_markdown_events(markdown: &str) {
    let parser = Parser::new(markdown);
    for event in parser {
        match event {
            Event::Start(Tag::Heading(_, _, _))
            | Event::End(Tag::Heading(_, _, _))
            | Event::Start(Tag::BlockQuote)
            | Event::End(Tag::BlockQuote)
            | Event::Rule
            | Event::Text(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Code(_)
            | Event::Html(_)
            | Event::Start(Tag::Paragraph)
            | Event::End(Tag::Paragraph)
            | Event::Start(_)
            | Event::End(_)
            | _ => {}
        }
    }
}

fn dedupe_duplicate_block_ids(mut blocks: Vec<MdBlock>) -> (Vec<MdBlock>, Vec<Diagnostic>) {
    use std::collections::HashSet;

    let mut diagnostics: Vec<Diagnostic> = vec![];
    let mut seen: HashSet<String> = HashSet::new();

    for block in blocks.iter_mut() {
        let Some(id) = block.block_id.clone() else {
            continue;
        };

        if seen.insert(id.clone()) {
            continue;
        }

        let new_id = format!("{}-{}", id, uuid::Uuid::new_v4().simple());
        block.block_id = Some(new_id.clone());

        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warn,
            code: "E_JVM_BLOCK_ID_DUP_DEDUPED".to_string(),
            message: format!(
                "duplicate block id hint '{}' was auto-renamed to '{}'",
                id, new_id
            ),
            block_id: Some(new_id),
            suggestion: Some(
                "avoid reusing the same @block:id when duplicating content".to_string(),
            ),
        });
    }

    (blocks, diagnostics)
}

fn lines_to_block(lines: &[String]) -> MdBlock {
    let raw_lines = lines.to_vec();

    // Extract block hint if present on the first line.
    let mut idx = 0;
    let mut block_id: Option<String> = None;

    if let Some(first) = lines.first() {
        if let Some(id) = parse_block_hint_id(first) {
            block_id = Some(id);
            idx = 1;
        }
    }

    let content_lines = &lines[idx..];
    let joined = content_lines.join("\n");

    // classify
    if joined.trim() == "---" {
        return MdBlock {
            block_id,
            kind: MdBlockKind::HorizontalRule,
            text: "---".to_string(),
            _raw_lines: raw_lines,
        };
    }

    if let Some((level, title)) = parse_heading(&joined) {
        return MdBlock {
            block_id,
            kind: MdBlockKind::Heading { level },
            text: title,
            _raw_lines: raw_lines,
        };
    }

    if joined.trim_start().starts_with('>') {
        let text = joined
            .lines()
            .map(|l| l.trim_start())
            .map(|l| l.strip_prefix('>').unwrap_or(l).trim_start())
            .collect::<Vec<_>>()
            .join("\n");
        return MdBlock {
            block_id,
            kind: MdBlockKind::Blockquote,
            text,
            _raw_lines: raw_lines,
        };
    }

    MdBlock {
        block_id,
        kind: MdBlockKind::Paragraph,
        text: joined.trim().to_string(),
        _raw_lines: raw_lines,
    }
}

fn parse_block_hint_id(line: &str) -> Option<String> {
    let s = line.trim();
    if !s.starts_with("<!--") || !s.ends_with("-->") {
        return None;
    }
    // Very small parser for "@block:id=...".
    let inner = s.trim_start_matches("<!--").trim_end_matches("-->").trim();

    if !inner.starts_with("@block") {
        return None;
    }

    // Split by whitespace
    for part in inner.split_whitespace() {
        if let Some(rest) = part.strip_prefix("id=") {
            let id = rest.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
        if let Some(rest) = part.strip_prefix("@block:id=") {
            let id = rest.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
        if let Some(rest) = part.strip_prefix("@block:id") {
            let _ = rest;
        }
    }

    // Fallback: search "id=" within the string.
    if let Some(pos) = inner.find("id=") {
        let after = &inner[pos + 3..];
        let id = after
            .split(|c: char| c.is_whitespace() || c == '>' || c == '-')
            .next()
            .unwrap_or("")
            .trim();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    None
}

fn parse_heading(s: &str) -> Option<(i32, String)> {
    let trimmed = s.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = trimmed[hashes..].trim_start();
    if rest.is_empty() {
        return Some((hashes as i32, "".to_string()));
    }
    Some((hashes as i32, rest.to_string()))
}

pub fn to_tiptap_block(block: &MdBlock) -> serde_json::Value {
    let id = block
        .block_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    match block.kind {
        MdBlockKind::Heading { level } => {
            let attrs = serde_json::json!({ "level": level, "id": id });
            serde_json::json!({
                "type": "heading",
                "attrs": attrs,
                "content": text_to_inline_nodes(&block.text),
            })
        }
        MdBlockKind::Paragraph => {
            let attrs = serde_json::json!({ "id": id });
            serde_json::json!({
                "type": "paragraph",
                "attrs": attrs,
                "content": text_to_inline_nodes(&block.text),
            })
        }
        MdBlockKind::Blockquote => {
            let attrs = serde_json::json!({ "id": id });
            serde_json::json!({
                "type": "blockquote",
                "attrs": attrs,
                "content": [
                    {
                        "type": "paragraph",
                        "content": text_to_inline_nodes(&block.text)
                    }
                ]
            })
        }
        MdBlockKind::HorizontalRule => serde_json::json!({ "type": "horizontalRule" }),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn build_doc_from_markdown_blocks(md_blocks: &[MdBlock]) -> serde_json::Value {
    let content = md_blocks
        .iter()
        .map(|block| to_tiptap_block(block))
        .collect::<Vec<_>>();

    serde_json::json!({
        "type": "doc",
        "content": content,
    })
}

/// Ensure every block in a TipTap document has a unique `attrs.id`.
/// Blocks missing an id get a freshly generated UUID.
/// Returns the number of blocks that were repaired.
pub fn ensure_doc_block_ids(doc: &mut serde_json::Value) -> usize {
    let Some(arr) = doc.get_mut("content").and_then(|v| v.as_array_mut()) else {
        return 0;
    };
    let mut repaired = 0;
    for node in arr.iter_mut() {
        let has_id = node
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty());
        if has_id {
            continue;
        }
        let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if matches!(node_type, "heading" | "paragraph" | "blockquote") {
            if node.get("attrs").is_none() {
                node["attrs"] = serde_json::json!({});
            }
            node["attrs"]["id"] = serde_json::Value::String(uuid::Uuid::new_v4().to_string());
            repaired += 1;
        }
    }
    repaired
}

fn text_to_inline_nodes(text: &str) -> Vec<serde_json::Value> {
    if text.is_empty() {
        return vec![];
    }
    vec![serde_json::json!({ "type": "text", "text": text })]
}

pub fn app_err_jvm(
    code: &str,
    message: String,
    recoverable: bool,
    details: Option<serde_json::Value>,
) -> AppError {
    AppError {
        code: crate::models::ErrorCode::Internal,
        message: format!("{code}: {message}"),
        details: Some(merge_details(code, details)),
        recoverable: Some(recoverable),
    }
}

fn merge_details(code: &str, details: Option<serde_json::Value>) -> serde_json::Value {
    let mut base = serde_json::json!({ "code": code });
    if let Some(d) = details {
        match d {
            serde_json::Value::Object(map) => {
                if let serde_json::Value::Object(base_map) = &mut base {
                    for (k, v) in map {
                        base_map.insert(k, v);
                    }
                }
            }
            other => {
                base["extra"] = other;
            }
        }
    }
    base
}

#[allow(dead_code)]
fn _heading_level(level: HeadingLevel) -> i32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
