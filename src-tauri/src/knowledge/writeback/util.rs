use std::collections::HashSet;

pub(super) fn merge_unique(mut base: Vec<String>, extra: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = base.iter().cloned().collect();
    for s in extra {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        if seen.insert(s.to_string()) {
            base.push(s.to_string());
        }
    }
    base
}

