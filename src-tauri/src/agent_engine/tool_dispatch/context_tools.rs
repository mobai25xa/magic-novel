use serde_json::json;

use crate::agent_tools::contracts::{ToolMeta, ToolResult};
use crate::services::{
    load_outline_dataset, lookup_character_sheet, search_knowledge_files, CharacterSheetLookup,
    KnowledgeSearchLookup,
};

use super::super::types::ToolCallInfo;

const OUTLINE_FIELDS: &[&str] = &["volume_path", "include_summary"];
const CHARACTER_SHEET_FIELDS: &[&str] = &["name"];
const SEARCH_KNOWLEDGE_FIELDS: &[&str] = &["query", "top_k"];

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_fields(tool_name: &str) -> Option<&'static [&'static str]> {
    match tool_name {
        "outline" => Some(OUTLINE_FIELDS),
        "character_sheet" => Some(CHARACTER_SHEET_FIELDS),
        "search_knowledge" => Some(SEARCH_KNOWLEDGE_FIELDS),
        _ => None,
    }
}

pub(super) fn execute_outline_tool(
    tc: &ToolCallInfo,
    project_path: &str,
    call_id: &str,
) -> ToolResult<serde_json::Value> {
    let started = std::time::Instant::now();

    if let Err(error) = reject_unknown_fields(&tc.args, OUTLINE_FIELDS, "outline") {
        return super::tool_parse_error("outline", call_id, &error);
    }

    let volume_path = tc.args.get("volume_path").and_then(|v| v.as_str());
    let include_summary = tc
        .args
        .get("include_summary")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let outline = load_outline_dataset(project_path, volume_path, include_summary);

    let mut lines = Vec::new();
    lines.push("# Book Outline".to_string());

    for volume in outline.volumes {
        let mut ch_lines = Vec::new();
        for (index, chapter) in volume.chapters.iter().enumerate() {
            let status = chapter
                .status
                .as_ref()
                .map(|status| format!(" [{}]", status))
                .unwrap_or_default();

            let mut line = format!(
                "{}. {} ({}) — {} words{}",
                index + 1,
                chapter.title,
                chapter.chapter_path,
                chapter.word_count,
                status
            );
            if let Some(summary) = chapter.summary.as_deref() {
                line.push_str(&format!("\n   Summary: {}", summary));
            }
            ch_lines.push(line);
        }

        lines.push(format!(
            "\n## {} ({})\n{} chapters, {} words",
            volume.title, volume.volume_path, volume.chapter_count, volume.word_count
        ));
        lines.extend(ch_lines);
    }

    let mut text = lines.join("\n");
    if text.chars().count() > 3000 {
        text = format!(
            "{}[...truncated, narrow the scope to refine results]",
            super::super::text_utils::truncate_chars(&text, 2955)
        );
    }

    ToolResult {
        ok: true,
        data: Some(json!({ "outline": text })),
        error: None,
        meta: ToolMeta {
            tool: "outline".to_string(),
            call_id: call_id.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

pub(super) fn execute_character_sheet_tool(
    tc: &ToolCallInfo,
    project_path: &str,
    call_id: &str,
) -> ToolResult<serde_json::Value> {
    let started = std::time::Instant::now();

    if let Err(error) = reject_unknown_fields(&tc.args, CHARACTER_SHEET_FIELDS, "character_sheet") {
        return super::tool_parse_error("character_sheet", call_id, &error);
    }

    let name = tc.args.get("name").and_then(|v| v.as_str());
    let text = match lookup_character_sheet(project_path, name) {
        CharacterSheetLookup::MissingDirectory => {
            "Character directory does not exist (.magic_novel/characters/). Create character profiles first.".to_string()
        }
        CharacterSheetLookup::EmptyDirectory => "Character directory is empty.".to_string(),
        CharacterSheetLookup::DirectoryList { files } => {
            let mut lines = vec!["Character list:".to_string()];
            for file in &files {
                let display = file
                    .rsplit_once('.')
                    .map(|(base, _)| base)
                    .unwrap_or(file);
                lines.push(format!("- {} ({})", display, file));
            }
            lines.join("\n")
        }
        CharacterSheetLookup::Match { content, .. } => content,
        CharacterSheetLookup::NotFound { query, available } => format!(
            "No matching character found: {}. Available: {}",
            query,
            available.join(", ")
        ),
    };

    let text = if text.chars().count() > 2000 {
        format!(
            "{}[...truncated, narrow the scope to refine results]",
            super::super::text_utils::truncate_chars(&text, 1955)
        )
    } else {
        text
    };

    ToolResult {
        ok: true,
        data: Some(json!({ "result": text })),
        error: None,
        meta: ToolMeta {
            tool: "character_sheet".to_string(),
            call_id: call_id.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

pub(super) fn execute_search_knowledge_tool(
    tc: &ToolCallInfo,
    project_path: &str,
    call_id: &str,
) -> ToolResult<serde_json::Value> {
    let started = std::time::Instant::now();

    if let Err(error) = reject_unknown_fields(&tc.args, SEARCH_KNOWLEDGE_FIELDS, "search_knowledge")
    {
        return super::tool_parse_error("search_knowledge", call_id, &error);
    }

    let query = tc.args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let top_k = tc.args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

    if query.trim().is_empty() {
        return super::tool_parse_error("search_knowledge", call_id, "query must not be empty");
    }

    let text = match search_knowledge_files(project_path, query, top_k) {
        KnowledgeSearchLookup::MissingDirectory => {
            "Knowledge base directory does not exist (.magic_novel/).".to_string()
        }
        KnowledgeSearchLookup::Matches { query, hits } => {
            if hits.is_empty() {
                format!("No knowledge matches found for \"{}\".", query)
            } else {
                let mut lines = vec![format!("Search results for \"{}\":", query)];
                for hit in &hits {
                    lines.push(format!("\n--- {} ---\n{}", hit.path, hit.snippet));
                }
                lines.join("\n")
            }
        }
    };

    let text = if text.chars().count() > 2000 {
        format!(
            "{}[...truncated, narrow the scope to refine results]",
            super::super::text_utils::truncate_chars(&text, 1955)
        )
    } else {
        text
    };

    ToolResult {
        ok: true,
        data: Some(json!({ "result": text })),
        error: None,
        meta: ToolMeta {
            tool: "search_knowledge".to_string(),
            call_id: call_id.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before: None,
            revision_after: None,
            tx_id: None,
            read_set: None,
            write_set: None,
        },
    }
}

fn reject_unknown_fields(
    args: &serde_json::Value,
    fields: &[&str],
    tool: &str,
) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(format!("{tool} args: unknown field '{key}'"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn context_parser_allowlists_match_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();

        for tool_name in ["outline", "character_sheet", "search_knowledge"] {
            let schema = crate::agent_tools::registry::get_schema(tool_name, &context)
                .unwrap_or_else(|| panic!("missing schema for {tool_name}"));
            let schema_fields: BTreeSet<String> = schema
                .get("properties")
                .and_then(|value| value.as_object())
                .expect("schema properties")
                .keys()
                .cloned()
                .collect();
            let parser_fields: BTreeSet<String> = super::parser_contract_fields(tool_name)
                .expect("parser fields")
                .iter()
                .map(|field| field.to_string())
                .collect();

            assert_eq!(
                schema_fields, parser_fields,
                "schema/parser mismatch for {tool_name}"
            );
        }
    }
}
