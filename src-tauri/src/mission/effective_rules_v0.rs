//! Mission system - EffectiveRules(v0) parsing.
//!
//! v0 scope: only `hard_constraints.chapter_words`.

use std::path::Path;

use crate::services::global_config;

use super::contextpack_staleness::compute_rules_fingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesSource {
    Guidelines,
    GlobalRules,
}

impl RulesSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            RulesSource::Guidelines => "guidelines",
            RulesSource::GlobalRules => "global_rules",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterWordsConstraint {
    pub min: i32,
    pub max: i32,
    pub target: Option<i32>,
    pub source: RulesSource,
}

#[derive(Debug, Clone)]
pub struct EffectiveRulesV0 {
    pub chapter_words: Option<ChapterWordsConstraint>,
    pub version_fingerprint: String,
}

pub fn load_effective_rules_v0(project_path: &Path) -> EffectiveRulesV0 {
    let guidelines = read_guidelines(project_path);
    let global = global_config::load_global_rules()
        .map(|r| r.content)
        .unwrap_or_default();

    let chapter_words = parse_chapter_words_from_sources(&guidelines, &global);
    let fingerprint = compute_rules_fingerprint(project_path).to_string();

    EffectiveRulesV0 {
        chapter_words,
        version_fingerprint: fingerprint,
    }
}

fn read_guidelines(project_path: &Path) -> String {
    let path = project_path.join(".magic_novel").join("guidelines.md");
    std::fs::read_to_string(&path).unwrap_or_default()
}

fn parse_chapter_words_from_sources(
    guidelines: &str,
    global_rules: &str,
) -> Option<ChapterWordsConstraint> {
    if let Some(mut c) = parse_chapter_words_from_text(guidelines) {
        c.source = RulesSource::Guidelines;
        return Some(c);
    }
    if let Some(mut c) = parse_chapter_words_from_text(global_rules) {
        c.source = RulesSource::GlobalRules;
        return Some(c);
    }
    None
}

fn parse_chapter_words_from_text(text: &str) -> Option<ChapterWordsConstraint> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if !lower.starts_with("chapter_words") {
            continue;
        }

        let idx = trimmed.find(':').or_else(|| trimmed.find('：'))?;
        let rhs = trimmed[idx + 1..].trim();
        let (min, max) = parse_min_max(rhs)?;
        if min <= 0 || max <= 0 || min > max {
            continue;
        }

        let target = parse_target(rhs);

        return Some(ChapterWordsConstraint {
            min,
            max,
            target,
            source: RulesSource::Guidelines, // placeholder; overridden by caller
        });
    }

    None
}

fn parse_min_max(s: &str) -> Option<(i32, i32)> {
    let re = regex::Regex::new(
        r"(?x)
        (?P<min>\d{1,6})
        \s*[-~～]\s*
        (?P<max>\d{1,6})
    ",
    )
    .ok()?;
    let caps = re.captures(s)?;
    let min: i32 = caps.name("min")?.as_str().parse().ok()?;
    let max: i32 = caps.name("max")?.as_str().parse().ok()?;
    Some((min, max))
}

fn parse_target(s: &str) -> Option<i32> {
    let re = regex::Regex::new(r"(?i)target\s*[:=]?\s*(\d{1,6})").ok()?;
    let caps = re.captures(s)?;
    let v: i32 = caps.get(1)?.as_str().parse().ok()?;
    if v > 0 {
        Some(v)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_words_parses_range_and_target() {
        let c = parse_chapter_words_from_text("chapter_words: 2000-3000 (target 2400)").unwrap();
        assert_eq!(c.min, 2000);
        assert_eq!(c.max, 3000);
        assert_eq!(c.target, Some(2400));
    }

    #[test]
    fn guidelines_override_global() {
        let guidelines = "chapter_words: 2000-3000 (target 2400)";
        let global = "chapter_words: 100-200 (target 150)";
        let c = parse_chapter_words_from_sources(guidelines, global).unwrap();
        assert_eq!(c.min, 2000);
        assert_eq!(c.max, 3000);
        assert_eq!(c.target, Some(2400));
        assert_eq!(c.source, RulesSource::Guidelines);
    }

    #[test]
    fn falls_back_to_global_when_guidelines_missing() {
        let guidelines = "no match";
        let global = "chapter_words: 100-200";
        let c = parse_chapter_words_from_sources(guidelines, global).unwrap();
        assert_eq!(c.min, 100);
        assert_eq!(c.max, 200);
        assert_eq!(c.target, None);
        assert_eq!(c.source, RulesSource::GlobalRules);
    }

    #[test]
    fn returns_none_when_no_match() {
        assert!(parse_chapter_words_from_sources("x", "y").is_none());
    }
}
