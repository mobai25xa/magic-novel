/// Budget helpers for tool outputs (character-based, CJK-safe).

/// Truncate `text` to at most `budget_chars` characters.
/// Returns `(truncated_text, truncated_flag)`.
pub fn truncate_to_budget_chars(text: &str, budget_chars: usize) -> (String, bool) {
    let total = text.chars().count();
    if total <= budget_chars {
        return (text.to_string(), false);
    }

    if budget_chars == 0 {
        return (String::new(), true);
    }

    let truncated = text
        .char_indices()
        .nth(budget_chars)
        .map(|(byte_idx, _)| text[..byte_idx].to_string())
        .unwrap_or_else(|| text.to_string());

    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_to_budget_noop_when_short() {
        let (out, truncated) = truncate_to_budget_chars("hello", 10);
        assert_eq!(out, "hello");
        assert!(!truncated);
    }

    #[test]
    fn truncate_to_budget_truncates_ascii() {
        let (out, truncated) = truncate_to_budget_chars("hello", 2);
        assert_eq!(out, "he");
        assert!(truncated);
    }

    #[test]
    fn truncate_to_budget_truncates_cjk_safely() {
        let (out, truncated) = truncate_to_budget_chars("你好世界", 2);
        assert_eq!(out, "你好");
        assert!(truncated);
    }

    #[test]
    fn truncate_to_budget_zero_budget_returns_empty_and_truncated() {
        let (out, truncated) = truncate_to_budget_chars("你好", 0);
        assert_eq!(out, "");
        assert!(truncated);
    }
}

