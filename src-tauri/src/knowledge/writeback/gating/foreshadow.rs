fn foreshadow_progress_rank(status_label: &str) -> Option<i32> {
    match status_label.trim().to_lowercase().as_str() {
        "seeded" => Some(0),
        "active" => Some(1),
        "partially_paid" => Some(2),
        "paid" => Some(3),
        _ => None,
    }
}

pub(super) fn foreshadow_status_regresses(prev: &str, next: &str) -> bool {
    let prev_norm = prev.trim().to_lowercase();
    let next_norm = next.trim().to_lowercase();

    if prev_norm == "paid" && next_norm != "paid" {
        return true;
    }

    match (
        foreshadow_progress_rank(&prev_norm),
        foreshadow_progress_rank(&next_norm),
    ) {
        (Some(p), Some(n)) => n < p,
        _ => false,
    }
}

