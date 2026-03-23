use std::path::Path;

use super::super::path::{ensure_safe_relative_path, normalize_path};
use super::super::roots::knowledge_root_read;
use super::super::storage::read_stored_object;

pub(super) fn normalize_summary_key(input: &str) -> String {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

pub(super) fn recent_fact_dir_ref(target_ref: &str) -> Option<String> {
    let tr = normalize_path(target_ref);
    if !tr.starts_with("recent_facts/") {
        return None;
    }
    tr.rsplit_once('/').map(|(dir, _)| dir.to_string())
}

pub(super) fn load_existing_recent_fact_index(project_path: &Path, dir_ref: &str) -> Vec<(String, String)> {
    let dir_ref = normalize_path(dir_ref);
    let Ok(rel) = ensure_safe_relative_path(&dir_ref) else {
        return Vec::new();
    };
    let dir = knowledge_root_read(project_path).join(rel);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        let existing_ref = format!("{dir_ref}/{name}");
        let Ok(Some(obj)) = read_stored_object(&entry.path()) else {
            continue;
        };
        if obj.kind != "recent_fact" {
            continue;
        }
        if obj.status == "archived" {
            continue;
        }
        let Some(summary) = obj.fields.get("summary").and_then(|v| v.as_str()) else {
            continue;
        };
        let key = normalize_summary_key(summary);
        if key.is_empty() {
            continue;
        }
        out.push((existing_ref, key));
        if out.len() >= 200 {
            break;
        }
    }
    out
}

