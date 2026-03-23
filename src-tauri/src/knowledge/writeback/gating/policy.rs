pub(super) fn kind_allows_auto_if_pass(kind: &str) -> bool {
    matches!(kind, "chapter_summary" | "recent_fact" | "foreshadow")
}

pub(super) fn validate_auto_policy_fields(kind: &str, fields: &serde_json::Value) -> bool {
    match kind {
        "foreshadow" => {
            let Some(obj) = fields.as_object() else {
                return false;
            };

            // M4 P1: only allow lightweight foreshadow status progression to be auto-accepted.
            for k in obj.keys() {
                if !matches!(k.as_str(), "status_label" | "current_notes" | "seed_ref") {
                    return false;
                }
            }

            obj.contains_key("status_label")
        }
        // Spec: character updates must be manual/orchestrator-explicit (never auto).
        "character" => false,
        _ => true,
    }
}

