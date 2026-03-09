use crate::agent_tools::contracts::GrepScope;

const MANUSCRIPTS_PREFIX: &str = "manuscripts/";
const MAGIC_ASSETS_PREFIX: &str = "magic_assets/";
const MAGIC_NOVEL_PREFIX: &str = ".magic_novel/";

pub fn normalize_scope_prefixes(scope: Option<&GrepScope>) -> Vec<String> {
    let Some(scope) = scope else {
        return vec![];
    };

    scope
        .paths
        .iter()
        .map(|path| normalize_scope_path(path))
        .filter(|path| !path.is_empty() && path != ".")
        .collect()
}

pub fn in_scope(path: &str, scope_prefixes: &[String]) -> bool {
    if scope_prefixes.is_empty() {
        return true;
    }

    let normalized = path.trim().replace('\\', "/");
    scope_prefixes
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
}

pub fn normalize_scope_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");

    while normalized.starts_with("./") {
        normalized = normalized.trim_start_matches("./").to_string();
    }

    normalized = normalized.trim_start_matches('/').to_string();

    if normalized.starts_with(MANUSCRIPTS_PREFIX) {
        normalized = normalized
            .trim_start_matches(MANUSCRIPTS_PREFIX)
            .to_string();
    }

    if normalized.starts_with(MAGIC_ASSETS_PREFIX) {
        normalized = format!(
            "{MAGIC_NOVEL_PREFIX}{}",
            normalized.trim_start_matches(MAGIC_ASSETS_PREFIX)
        );
    }

    normalized.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_scope_path;

    #[test]
    fn normalizes_manuscripts_prefix() {
        assert_eq!(
            normalize_scope_path("manuscripts/vol1/chap.json"),
            "vol1/chap.json"
        );
    }

    #[test]
    fn normalizes_magic_assets_prefix() {
        assert_eq!(
            normalize_scope_path("magic_assets/characters"),
            ".magic_novel/characters"
        );
    }
}
