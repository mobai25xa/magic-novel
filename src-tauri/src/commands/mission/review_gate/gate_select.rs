use crate::mission::artifacts;

pub(super) fn should_enable_terminology_gate(project_path: &std::path::Path, mission_id: &str) -> bool {
    let cc = artifacts::read_layer1_chapter_card(project_path, mission_id)
        .ok()
        .flatten();
    if let Some(cc) = cc {
        let joined = cc
            .hard_constraints
            .iter()
            .chain(cc.success_criteria.iter())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if joined.contains("术语") || joined.contains("专名") || joined.contains("名词") {
            return true;
        }

        let lower = joined.to_ascii_lowercase();
        if lower.contains("terminology") || lower.contains("glossary") {
            return true;
        }
    }

    risk_ledger_has_gate_marker(
        project_path,
        mission_id,
        &["terminology"],
        &["术语", "专名", "名词", "terminology", "glossary"],
    )
}

pub(super) fn should_enable_foreshadow_gate(project_path: &std::path::Path, mission_id: &str) -> bool {
    let path = artifacts::layer1_active_foreshadowing_path(project_path, mission_id);
    if path.exists() {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let v: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return false,
        };

        if let Some(arr) = v.as_array() {
            if !arr.is_empty() {
                return true;
            }
        }
        if let Some(items) = v.get("items").and_then(|x| x.as_array()) {
            if !items.is_empty() {
                return true;
            }
        }
    }

    risk_ledger_has_gate_marker(
        project_path,
        mission_id,
        &["foreshadow"],
        &["伏笔", "foreshadow", "payoff"],
    )
}

pub(super) fn risk_ledger_has_gate_marker(
    project_path: &std::path::Path,
    mission_id: &str,
    review_types: &[&str],
    summary_keywords: &[&str],
) -> bool {
    let path = artifacts::layer1_risk_ledger_path(project_path, mission_id);
    if !path.exists() {
        return false;
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let items = match v.get("items").and_then(|x| x.as_array()) {
        Some(items) => items,
        None => return false,
    };

    let review_types = review_types
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    let summary_keywords = summary_keywords
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    items.iter().any(|item| {
        let review_type = item
            .get("review_type")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if !review_type.is_empty() && review_types.iter().any(|kw| kw == &review_type) {
            return true;
        }

        let summary = item
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        !summary.is_empty()
            && summary_keywords
                .iter()
                .any(|kw| summary.contains(kw.as_str()))
    })
}
