use std::path::{Path, PathBuf};

use crate::models::{Chapter, VolumeMetadata};
use crate::services::{list_dirs, list_files, read_json};

const MANUSCRIPTS_DIR: &str = "manuscripts";
const KNOWLEDGE_DIR: &str = ".magic_novel";
const CHARACTER_DIR: &str = "characters";
const CHARACTER_EXTENSIONS: &[&str] = &["md", "json", "txt"];
const KNOWLEDGE_EXTENSIONS: &[&str] = &["md", "json", "txt", "yaml", "yml"];

#[derive(Debug, Clone)]
pub struct OutlineDataset {
    pub volumes: Vec<OutlineVolumeEntry>,
}

#[derive(Debug, Clone)]
pub struct OutlineVolumeEntry {
    pub title: String,
    pub volume_path: String,
    pub chapter_count: u32,
    pub word_count: u64,
    pub chapters: Vec<OutlineChapterEntry>,
}

#[derive(Debug, Clone)]
pub struct OutlineChapterEntry {
    pub title: String,
    pub chapter_path: String,
    pub word_count: u64,
    pub status: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterSheetLookup {
    MissingDirectory,
    EmptyDirectory,
    DirectoryList {
        files: Vec<String>,
    },
    Match {
        file: String,
        content: String,
    },
    NotFound {
        query: String,
        available: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeSearchLookup {
    MissingDirectory,
    Matches {
        query: String,
        hits: Vec<KnowledgeMatch>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeMatch {
    pub path: String,
    pub snippet: String,
}

pub fn load_outline_dataset(
    project_path: &str,
    volume_filter: Option<&str>,
    include_summary: bool,
) -> OutlineDataset {
    let manuscripts_root = PathBuf::from(project_path).join(MANUSCRIPTS_DIR);
    let volume_dirs = list_dirs(&manuscripts_root).unwrap_or_default();

    let mut volumes = Vec::new();
    for vol_dir in volume_dirs {
        if let Some(filter) = volume_filter {
            if vol_dir != filter {
                continue;
            }
        }

        let vol_path = manuscripts_root.join(&vol_dir);
        let vol_meta: VolumeMetadata = match read_json(&vol_path.join("volume.json")) {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        let chapter_files = list_files(&vol_path, ".json").unwrap_or_default();
        let mut chapter_count = 0_u32;
        let mut word_count = 0_u64;
        let mut chapters = Vec::new();

        for ch_file in chapter_files {
            if ch_file == "volume.json" {
                continue;
            }

            let chapter: Chapter = match read_json(&vol_path.join(&ch_file)) {
                Ok(chapter) => chapter,
                Err(_) => continue,
            };

            chapter_count += 1;
            let chapter_words = chapter.counts.text_length_no_whitespace.max(0) as u64;
            word_count += chapter_words;

            chapters.push(OutlineChapterEntry {
                title: chapter.title,
                chapter_path: format!("{}/{}", vol_dir, ch_file),
                word_count: chapter_words,
                status: chapter
                    .status
                    .as_ref()
                    .map(|status| format!("{:?}", status)),
                summary: if include_summary {
                    chapter.summary.filter(|summary| !summary.is_empty())
                } else {
                    None
                },
            });
        }

        volumes.push(OutlineVolumeEntry {
            title: vol_meta.title,
            volume_path: vol_dir,
            chapter_count,
            word_count,
            chapters,
        });
    }

    OutlineDataset { volumes }
}

pub fn lookup_character_sheet(project_path: &str, name: Option<&str>) -> CharacterSheetLookup {
    let chars_dir = PathBuf::from(project_path)
        .join(KNOWLEDGE_DIR)
        .join(CHARACTER_DIR);

    if !chars_dir.exists() {
        return CharacterSheetLookup::MissingDirectory;
    }

    let files = list_files_with_extensions(&chars_dir, CHARACTER_EXTENSIONS);
    if files.is_empty() {
        return CharacterSheetLookup::EmptyDirectory;
    }

    match name.map(str::trim).filter(|name| !name.is_empty()) {
        None => CharacterSheetLookup::DirectoryList { files },
        Some(query) => {
            if let Some(file) = find_character_match(&files, query) {
                let content = std::fs::read_to_string(chars_dir.join(&file))
                    .unwrap_or_else(|_| format!("Cannot read file: {}", file));
                CharacterSheetLookup::Match { file, content }
            } else {
                CharacterSheetLookup::NotFound {
                    query: query.to_string(),
                    available: files,
                }
            }
        }
    }
}

pub fn search_knowledge_files(
    project_path: &str,
    query: &str,
    top_k: usize,
) -> KnowledgeSearchLookup {
    let root = PathBuf::from(project_path).join(KNOWLEDGE_DIR);
    if !root.exists() {
        return KnowledgeSearchLookup::MissingDirectory;
    }

    let mut matches = Vec::new();
    collect_recursive_matches(&root, &root, query, KNOWLEDGE_EXTENSIONS, &mut matches);
    matches.truncate(top_k);

    KnowledgeSearchLookup::Matches {
        query: query.to_string(),
        hits: matches,
    }
}

fn list_files_with_extensions(dir: &Path, allowed_extensions: &[&str]) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .file_type()
                        .map(|file_type| file_type.is_file())
                        .unwrap_or(false)
                })
                .filter_map(|entry| entry.file_name().to_str().map(|name| name.to_string()))
                .filter(|name| {
                    name.rsplit('.')
                        .next()
                        .map(|ext| {
                            allowed_extensions
                                .iter()
                                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
                        })
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    files.sort();
    files
}

fn find_character_match(files: &[String], query: &str) -> Option<String> {
    let query_lower = query.to_lowercase();
    let normalized_query = normalize_token(query);

    files.iter().find_map(|file| {
        let file_lower = file.to_lowercase();
        let normalized_file = normalize_token(file);
        (file_lower.contains(&query_lower) || normalized_file.contains(&normalized_query))
            .then(|| file.clone())
    })
}

fn normalize_token(input: &str) -> String {
    input
        .to_lowercase()
        .replace(['_', '-', ' '], "")
        .trim()
        .to_string()
}

fn collect_recursive_matches(
    base: &Path,
    dir: &Path,
    query: &str,
    allowed_extensions: &[&str],
    matches: &mut Vec<KnowledgeMatch>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let query_lower = query.to_lowercase();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive_matches(base, &path, query, allowed_extensions, matches);
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if !allowed_extensions
            .iter()
            .any(|allowed| extension.eq_ignore_ascii_case(allowed))
        {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let content_lower = content.to_lowercase();
        if !content_lower.contains(&query_lower) {
            continue;
        }

        let rel_path = path
            .strip_prefix(base)
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        if let Some(snippet) = extract_snippet(&content, &content_lower, &query_lower, query.len())
        {
            matches.push(KnowledgeMatch {
                path: rel_path,
                snippet,
            });
        }
    }
}

fn extract_snippet(
    content: &str,
    content_lower: &str,
    query_lower: &str,
    query_len: usize,
) -> Option<String> {
    let pos = content_lower.find(query_lower)?;
    let start = pos.saturating_sub(100);
    let end = (pos + query_len + 100).min(content.len());

    let start = content[..start]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0);
    let end = content[end..]
        .char_indices()
        .next()
        .map(|(index, _)| end + index)
        .unwrap_or(content.len());

    Some(content[start..end].to_string())
}
