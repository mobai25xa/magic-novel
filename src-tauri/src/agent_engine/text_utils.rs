pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    for (char_count, (byte_pos, _)) in s.char_indices().enumerate() {
        if char_count >= max_chars {
            return format!("{}...", &s[..byte_pos]);
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn truncate_ascii() {
        assert_eq!(truncate_chars("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_chinese() {
        assert_eq!(truncate_chars("你好世界", 2), "你好...");
    }

    #[test]
    fn truncate_mixed() {
        assert_eq!(truncate_chars("ab你好cd", 4), "ab你好...");
    }

    #[test]
    fn truncate_emoji() {
        assert_eq!(truncate_chars("A🙂B🙂C", 3), "A🙂B...");
    }

    #[test]
    fn truncate_empty() {
        assert_eq!(truncate_chars("", 3), "");
    }

    #[test]
    fn truncate_no_change_when_shorter() {
        assert_eq!(truncate_chars("abc", 10), "abc");
    }

    #[test]
    fn truncate_zero_chars() {
        assert_eq!(truncate_chars("你好", 0), "...");
    }

    #[test]
    fn truncate_result_is_valid_utf8_boundary() {
        let out = truncate_chars("中文🙂emoji", 3);
        let trimmed = out.strip_suffix("...").unwrap_or(&out);
        assert!("中文🙂emoji".starts_with(trimmed));
        assert!(std::str::from_utf8(trimmed.as_bytes()).is_ok());
    }
}
