#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefParts {
    pub kind: String,
    pub path: String,
}

pub(crate) fn parse_ref(input: &str) -> Result<RefParts, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("ref must be a non-empty string".to_string());
    }

    let Some((kind, path)) = raw.split_once(':') else {
        return Err("ref must be in '<kind>:<path>' format".to_string());
    };

    let kind = kind.trim();
    if kind.is_empty() {
        return Err("ref kind must be non-empty".to_string());
    }

    let path = path.trim();
    if kind != "book" && path.is_empty() {
        return Err("ref path must be non-empty for non-book refs".to_string());
    }

    validate_project_relative_path(path)?;

    Ok(RefParts {
        kind: kind.to_string(),
        path: path.to_string(),
    })
}

fn validate_project_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }

    if path.contains('\\') {
        return Err("ref path must use '/' (backslash is forbidden)".to_string());
    }

    if path.starts_with('/') {
        return Err("ref path must be project-relative (must not start with '/')".to_string());
    }

    if path.contains(':') {
        return Err("ref path must not contain ':'".to_string());
    }

    if path.starts_with("//") {
        return Err("ref path must not be a UNC-like path".to_string());
    }

    for segment in path.split('/') {
        if segment.is_empty() {
            return Err("ref path must not contain empty segments ('//')".to_string());
        }
        if segment == ".." {
            return Err("ref path must not contain '..' segments".to_string());
        }
    }

    Ok(())
}

pub(crate) fn ensure_ref_kind<'a>(
    ref_parts: &'a RefParts,
    expected: &str,
) -> Result<&'a str, String> {
    if ref_parts.kind != expected {
        return Err(format!(
            "ref kind '{}' is not supported here (expected '{}')",
            ref_parts.kind, expected
        ));
    }
    Ok(ref_parts.path.as_str())
}

pub(crate) fn volume_path_from_ref(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("ref must be a non-empty string".to_string());
    }

    let normalized = if raw.contains('\\') {
        raw.replace('\\', "/")
    } else {
        raw.to_string()
    };

    let path = if normalized.contains(':') {
        let parts = parse_ref(normalized.as_str())?;
        ensure_ref_kind(&parts, "volume")?.to_string()
    } else {
        normalized
    };

    let path = path.trim().trim_start_matches("./");
    if path.is_empty() {
        return Err("ref path must be non-empty for non-book refs".to_string());
    }
    validate_project_relative_path(path)?;

    let without_suffix = path
        .strip_suffix("/volume.json")
        .unwrap_or(path)
        .trim_end_matches('/');

    let rest = without_suffix
        .strip_prefix("manuscripts/")
        .unwrap_or(without_suffix);
    if rest.trim().is_empty() {
        return Err("volume ref is missing volume directory".to_string());
    }
    Ok(rest.to_string())
}

pub(crate) fn chapter_path_from_ref(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("ref must be a non-empty string".to_string());
    }

    let normalized = if raw.contains('\\') {
        raw.replace('\\', "/")
    } else {
        raw.to_string()
    };

    let path = if normalized.contains(':') {
        let parts = parse_ref(normalized.as_str())?;
        ensure_ref_kind(&parts, "chapter")?.to_string()
    } else {
        normalized
    };

    let path = path.trim().trim_start_matches("./");
    if path.is_empty() {
        return Err("ref path must be non-empty for non-book refs".to_string());
    }
    validate_project_relative_path(path)?;

    let rest = path.strip_prefix("manuscripts/").unwrap_or(path);
    if !rest.contains('/') || !rest.ends_with(".json") {
        return Err("chapter ref must include '<volume>/<chapter>.json'".to_string());
    }
    Ok(rest.to_string())
}

pub(crate) fn volume_ref(volume_path: &str) -> String {
    format!("volume:manuscripts/{volume_path}/volume.json")
}

pub(crate) fn chapter_ref(chapter_path: &str) -> String {
    format!("chapter:manuscripts/{chapter_path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_path_from_ref_accepts_shorthand_paths() {
        assert_eq!(
            volume_path_from_ref("volume:manuscripts/vol_1/volume.json").expect("ref"),
            "vol_1"
        );
        assert_eq!(
            volume_path_from_ref("manuscripts/vol_1").expect("ref"),
            "vol_1"
        );
        assert_eq!(volume_path_from_ref("volume:vol_1").expect("ref"), "vol_1");
    }

    #[test]
    fn chapter_path_from_ref_accepts_shorthand_paths() {
        for input in [
            "chapter:manuscripts/vol_1/ch_1.json",
            "manuscripts/vol_1/ch_1.json",
            "chapter:vol_1/ch_1.json",
            "vol_1/ch_1.json",
            r"chapter:manuscripts\vol_1\ch_1.json",
        ] {
            assert_eq!(
                chapter_path_from_ref(input).expect("ref"),
                "vol_1/ch_1.json",
                "{input}"
            );
        }
    }

    #[test]
    fn chapter_path_from_ref_rejects_non_json_paths() {
        let err = chapter_path_from_ref("chapter:vol_1/ch_1").unwrap_err();
        assert!(err.contains("<volume>/<chapter>.json"));
    }
}
