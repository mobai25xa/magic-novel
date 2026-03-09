use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use regex::Regex;
use serde_json::json;

use crate::agent_tools::contracts::GrepHit;
use crate::kernel::search::corpus_extract::extract_asset_text;
use crate::kernel::search::keyword::{build_snippet, find_keyword, score_keyword};
use crate::models::{AppError, AssetTree};
use crate::services::read_json;

use crate::application::search_usecases::index::scope::in_scope;

const MAGIC_ASSETS_DIR: &str = "magic_assets";
const MAGIC_FOLDER_META: &str = ".magic_folder.json";
const MATCH_COUNT_CAP: u32 = 64;
const SNIPPET_CONTEXT_CHARS: usize = 60;

#[derive(Debug, Clone)]
struct AssetFileInfo {
    doc_path: String,
    rel_path: String,
    full_path: PathBuf,
    modified_ms: i64,
}

#[derive(Debug, Clone)]
struct CachedAssetDoc {
    doc_path: String,
    modified_ms: i64,
    rel_path: String,
    asset_id: String,
    asset_kind: String,
    title: String,
    text: Arc<String>,
}

#[derive(Debug, Default)]
struct ProjectAssetCache {
    docs: HashMap<String, CachedAssetDoc>,
}

static ASSET_CACHE: OnceLock<Mutex<HashMap<String, ProjectAssetCache>>> = OnceLock::new();

pub fn grep_assets_keyword(
    project_path: &str,
    query_re: &Regex,
    scope_prefixes: &[String],
    top_k: u32,
) -> Result<Vec<GrepHit>, AppError> {
    let files = list_all_asset_files(project_path)?;
    let docs = sync_cache_and_snapshot(project_path, files)?;

    let mut hits = vec![];

    for doc in docs {
        if !in_scope(&doc.doc_path, scope_prefixes) {
            continue;
        }

        let Some(matched) = find_keyword(doc.text.as_str(), query_re, MATCH_COUNT_CAP) else {
            continue;
        };

        let snippet = build_snippet(
            doc.text.as_str(),
            matched.start,
            matched.end,
            SNIPPET_CONTEXT_CHARS,
        );

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("source".to_string(), json!("asset"));
        metadata.insert("asset_id".to_string(), json!(doc.asset_id));
        metadata.insert("asset_kind".to_string(), json!(doc.asset_kind));
        metadata.insert("title".to_string(), json!(doc.title));
        metadata.insert("relative_path".to_string(), json!(doc.rel_path));
        metadata.insert("match_count".to_string(), json!(matched.count));

        hits.push(GrepHit {
            path: doc.doc_path,
            score: score_keyword(&matched),
            snippet,
            metadata,
        });
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(top_k as usize);

    Ok(hits)
}

fn list_all_asset_files(project_path: &str) -> Result<Vec<AssetFileInfo>, AppError> {
    let root = PathBuf::from(project_path).join(MAGIC_ASSETS_DIR);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut out = vec![];
    walk_asset_dir(&root, "", &mut out)?;
    Ok(out)
}

fn walk_asset_dir(
    dir: &Path,
    relative: &str,
    out: &mut Vec<AssetFileInfo>,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        let rel_path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };

        if file_type.is_dir() {
            walk_asset_dir(&path, &rel_path, out)?;
            continue;
        }

        if !file_type.is_file() || name == MAGIC_FOLDER_META || !name.ends_with(".json") {
            continue;
        }

        out.push(AssetFileInfo {
            doc_path: format!(".magic_novel/{rel_path}"),
            rel_path,
            full_path: path.clone(),
            modified_ms: file_modified_millis(&path),
        });
    }

    Ok(())
}

fn file_modified_millis(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(system_time_to_millis)
        .unwrap_or(0)
}

fn system_time_to_millis(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

fn sync_cache_and_snapshot(
    project_path: &str,
    files: Vec<AssetFileInfo>,
) -> Result<Vec<CachedAssetDoc>, AppError> {
    let wanted: HashSet<String> = files.iter().map(|f| f.doc_path.clone()).collect();

    let (to_refresh, to_remove) = {
        let mut cache_map = lock_cache()?;
        let cache = cache_map
            .entry(project_path.to_string())
            .or_insert_with(ProjectAssetCache::default);

        let mut refresh = vec![];
        for info in &files {
            let needs = cache
                .docs
                .get(&info.doc_path)
                .map(|c| c.modified_ms != info.modified_ms)
                .unwrap_or(true);

            if needs {
                refresh.push(info.clone());
            }
        }

        let remove: Vec<String> = cache
            .docs
            .keys()
            .filter(|k| !wanted.contains(*k))
            .cloned()
            .collect();

        (refresh, remove)
    };

    let refreshed = load_refreshed_docs(&to_refresh);

    let mut cache_map = lock_cache()?;
    let cache = cache_map
        .entry(project_path.to_string())
        .or_insert_with(ProjectAssetCache::default);

    for key in to_remove {
        cache.docs.remove(&key);
    }

    for (doc_path, doc) in refreshed {
        cache.docs.insert(doc_path, doc);
    }

    Ok(cache.docs.values().cloned().collect())
}

fn lock_cache(
) -> Result<std::sync::MutexGuard<'static, HashMap<String, ProjectAssetCache>>, AppError> {
    ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::internal("E_TOOL_INTERNAL: grep asset cache lock poisoned"))
}

fn load_refreshed_docs(to_refresh: &[AssetFileInfo]) -> Vec<(String, CachedAssetDoc)> {
    let mut out = vec![];

    for info in to_refresh {
        if let Ok(doc) = load_asset_doc(info) {
            out.push((info.doc_path.clone(), doc));
        }
    }

    out
}

fn load_asset_doc(info: &AssetFileInfo) -> Result<CachedAssetDoc, AppError> {
    let asset: AssetTree = read_json(&info.full_path)?;
    let text = extract_asset_text(&asset);

    Ok(CachedAssetDoc {
        doc_path: info.doc_path.clone(),
        modified_ms: info.modified_ms,
        rel_path: info.rel_path.clone(),
        asset_id: asset.id,
        asset_kind: format!("{:?}", asset.kind).to_lowercase(),
        title: asset.title,
        text: Arc::new(text),
    })
}
