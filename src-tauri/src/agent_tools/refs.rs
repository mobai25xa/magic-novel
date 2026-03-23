/// Ref parsing and validation (Common Tool Contract v0).
///
/// Ref format: `<kind>:<project_relative_path>`
/// - `kind` is required.
/// - `project_relative_path` is required, except for `book`.
/// - Paths must use `/` and must not contain `..`, drive letters, or UNC paths.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRef {
    pub kind: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefError {
    pub code: &'static str,
    pub message: String,
}

pub fn parse_ref(input: &str) -> Result<ParsedRef, RefError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ref_invalid("ref is empty"));
    }

    let Some((kind_raw, path_raw)) = trimmed.split_once(':') else {
        return Err(ref_invalid("ref must be '<kind>:<path>'"));
    };

    let kind = kind_raw.trim();
    if kind.is_empty() {
        return Err(ref_invalid("ref kind is empty"));
    }

    if !kind
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(ref_invalid("ref kind must be snake_case ascii"));
    }

    let path = path_raw.trim();
    if path.is_empty() {
        if kind == "book" {
            return Ok(ParsedRef {
                kind: kind.to_string(),
                path: None,
            });
        }
        return Err(ref_invalid("ref path is empty"));
    }

    validate_ref_path(path)?;

    Ok(ParsedRef {
        kind: kind.to_string(),
        path: Some(path.to_string()),
    })
}

pub fn ensure_kind_supported(parsed: &ParsedRef, supported: &[&str]) -> Result<(), RefError> {
    if supported.iter().any(|k| *k == parsed.kind) {
        Ok(())
    } else {
        Err(RefError {
            code: "E_REF_KIND_UNSUPPORTED",
            message: format!("unsupported ref kind '{}'", parsed.kind),
        })
    }
}

fn validate_ref_path(path: &str) -> Result<(), RefError> {
    if path.starts_with('/') {
        return Err(ref_invalid("ref path must be project-relative (must not start with '/')"));
    }

    if path.starts_with("//") {
        return Err(ref_invalid("ref path must not start with '//'"));
    }

    if path.contains('\\') {
        return Err(ref_invalid("ref path must use '/' (backslash is not allowed)"));
    }

    if path.contains('\0') {
        return Err(ref_invalid("ref path contains NUL"));
    }

    // Disallow colon to prevent drive-letter paths and other ambiguous forms.
    if path.contains(':') {
        return Err(ref_invalid("ref path must not contain ':'"));
    }

    for seg in path.split('/') {
        if seg.is_empty() {
            return Err(ref_invalid("ref path must not contain empty segments ('//')"));
        }
        if seg == ".." {
            return Err(ref_invalid("ref path must not contain '..' segments"));
        }
    }

    Ok(())
}

fn ref_invalid(msg: &str) -> RefError {
    RefError {
        code: "E_REF_INVALID",
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ref_accepts_chapter_ref() {
        let parsed = parse_ref("chapter:manuscripts/vol_1/ch_01.json").expect("should parse");
        assert_eq!(parsed.kind, "chapter");
        assert_eq!(parsed.path.as_deref(), Some("manuscripts/vol_1/ch_01.json"));
    }

    #[test]
    fn parse_ref_accepts_book_ref_without_path() {
        let parsed = parse_ref("book:").expect("should parse");
        assert_eq!(parsed.kind, "book");
        assert_eq!(parsed.path, None);
    }

    #[test]
    fn parse_ref_rejects_missing_colon() {
        let err = parse_ref("chapter").expect_err("should fail");
        assert_eq!(err.code, "E_REF_INVALID");
    }

    #[test]
    fn parse_ref_rejects_dotdot_segment() {
        let err = parse_ref("chapter:manuscripts/../secrets.txt").expect_err("should fail");
        assert_eq!(err.code, "E_REF_INVALID");
        assert!(err.message.contains(".."));
    }

    #[test]
    fn parse_ref_rejects_backslash() {
        let err = parse_ref(r"chapter:manuscripts\vol_1\ch_01.json").expect_err("should fail");
        assert_eq!(err.code, "E_REF_INVALID");
        assert!(err.message.contains("backslash"));
    }

    #[test]
    fn parse_ref_rejects_drive_letter_like_path() {
        let err = parse_ref("chapter:C:/Users/admin/file.txt").expect_err("should fail");
        assert_eq!(err.code, "E_REF_INVALID");
        assert!(err.message.contains(":"));
    }

    #[test]
    fn ensure_kind_supported_reports_kind_unsupported() {
        let parsed = ParsedRef {
            kind: "artifact".to_string(),
            path: Some("missions/x.json".to_string()),
        };
        let err = ensure_kind_supported(&parsed, &["chapter", "volume"]).expect_err("should fail");
        assert_eq!(err.code, "E_REF_KIND_UNSUPPORTED");
    }
}
