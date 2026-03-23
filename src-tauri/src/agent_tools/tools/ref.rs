use crate::agent_tools::contracts::FaultDomain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Book,
    Volume,
    Chapter,
    Knowledge,
    Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRef {
    pub kind: RefKind,
    /// Normalized, project-relative path using `/`. For `book`, may be empty.
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct RefError {
    pub code: &'static str,
    pub fault_domain: FaultDomain,
    pub message: String,
}

pub fn parse_tool_ref(raw: &str) -> Result<ToolRef, RefError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref is empty".to_string(),
        });
    }

    if looks_like_unc_path(raw) || looks_like_windows_drive_path(raw) {
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref must be project-relative (not an OS absolute path) and must be in '<kind>:<project_relative_path>' format"
                .to_string(),
        });
    }

    // Be forgiving about Windows-style separators in model output; normalize to `/`.
    let normalized = if raw.contains('\\') {
        raw.replace('\\', "/")
    } else {
        raw.to_string()
    };
    let normalized = normalized.trim();

    let (kind, path_raw) = match normalized.split_once(':') {
        Some((kind_raw, path_raw)) => {
            let kind = match kind_raw.trim() {
                "book" => RefKind::Book,
                "volume" => RefKind::Volume,
                "chapter" => RefKind::Chapter,
                "knowledge" => RefKind::Knowledge,
                "artifact" => RefKind::Artifact,
                other => {
                    return Err(RefError {
                        code: "E_REF_INVALID",
                        fault_domain: FaultDomain::Validation,
                        message: format!("unsupported ref kind: '{other}'"),
                    })
                }
            };
            (kind, path_raw)
        }
        None => {
            let kind = infer_ref_kind_from_shorthand_path(normalized).ok_or_else(|| RefError {
                code: "E_REF_INVALID",
                fault_domain: FaultDomain::Validation,
                message: "ref must be in the form '<kind>:<project_relative_path>'".to_string(),
            })?;
            (kind, normalized)
        }
    };

    let path = normalize_project_relative_path(path_raw, kind == RefKind::Book)?;

    Ok(ToolRef { kind, path })
}

fn looks_like_unc_path(s: &str) -> bool {
    let trimmed = s.trim();
    trimmed.starts_with("\\\\") || trimmed.starts_with("//")
}

fn looks_like_windows_drive_path(s: &str) -> bool {
    let trimmed = s.trim();
    let bytes = trimmed.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

fn infer_ref_kind_from_shorthand_path(path: &str) -> Option<RefKind> {
    let p = path.trim().trim_end_matches('/').trim_start_matches("./");
    if p.is_empty() {
        return None;
    }

    if p == ".magic_novel" || p.starts_with(".magic_novel/") {
        return Some(RefKind::Knowledge);
    }

    if p.ends_with(".json") {
        if p.ends_with("/volume.json") || p == "volume.json" {
            return Some(RefKind::Volume);
        }
        return Some(RefKind::Chapter);
    }

    if p.starts_with("manuscripts/") {
        return Some(RefKind::Volume);
    }

    None
}

pub fn normalize_project_relative_path(raw: &str, allow_empty: bool) -> Result<String, RefError> {
    let mut path = raw.trim().to_string();
    while path.starts_with("./") {
        path = path.trim_start_matches("./").to_string();
    }

    if path.is_empty() {
        if allow_empty {
            return Ok(String::new());
        }
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref path is empty".to_string(),
        });
    }

    if path.starts_with('/') {
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref path must be project-relative (must not start with '/')".to_string(),
        });
    }

    if path.contains(':') {
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref path must not contain ':'".to_string(),
        });
    }

    let parts: Vec<&str> = path
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.iter().any(|p| *p == "..") {
        return Err(RefError {
            code: "E_REF_INVALID",
            fault_domain: FaultDomain::Validation,
            message: "ref path must not contain '..'".to_string(),
        });
    }

    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ref_normalizes_backslashes() {
        let parsed = parse_tool_ref("chapter:manuscripts\\vol_1\\ch.json").expect("ref");
        assert_eq!(parsed.kind, RefKind::Chapter);
        assert_eq!(parsed.path, "manuscripts/vol_1/ch.json");
    }

    #[test]
    fn parse_ref_rejects_dotdot() {
        let err = parse_tool_ref("chapter:manuscripts/../secret.txt").unwrap_err();
        assert_eq!(err.code, "E_REF_INVALID");
    }

    #[test]
    fn parse_ref_accepts_magic_novel_paths() {
        let parsed = parse_tool_ref("knowledge:.magic_novel/characters/a.md").expect("ref");
        assert_eq!(parsed.kind, RefKind::Knowledge);
        assert_eq!(parsed.path, ".magic_novel/characters/a.md");
    }

    #[test]
    fn parse_ref_accepts_shorthand_chapter_path() {
        let parsed = parse_tool_ref("manuscripts/vol_1/ch_1.json").expect("ref");
        assert_eq!(parsed.kind, RefKind::Chapter);
        assert_eq!(parsed.path, "manuscripts/vol_1/ch_1.json");
    }

    #[test]
    fn parse_ref_accepts_shorthand_volume_path() {
        let parsed = parse_tool_ref("manuscripts/vol_1").expect("ref");
        assert_eq!(parsed.kind, RefKind::Volume);
        assert_eq!(parsed.path, "manuscripts/vol_1");
    }

    #[test]
    fn parse_ref_accepts_shorthand_magic_novel_path() {
        let parsed = parse_tool_ref(".magic_novel/terms/foo.md").expect("ref");
        assert_eq!(parsed.kind, RefKind::Knowledge);
        assert_eq!(parsed.path, ".magic_novel/terms/foo.md");
    }
}
