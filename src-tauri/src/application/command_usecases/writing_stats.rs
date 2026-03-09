use crate::models::{AppError, DailyStats, WritingSession, WritingStatsStore};
use crate::services::{ensure_dir, read_json, write_json};
use chrono::Datelike;
use std::path::PathBuf;
use tauri::command;

const STATS_DIR: &str = ".magic-novel";
const STATS_FILE: &str = "writing_stats.json";

fn get_stats_path(root_dir: Option<&str>) -> Result<PathBuf, AppError> {
    let stats_dir = if let Some(root) = root_dir {
        let root_path = PathBuf::from(root);
        root_path.join(STATS_DIR)
    } else {
        // Fallback to home directory for backward compatibility
        let home =
            dirs::home_dir().ok_or_else(|| AppError::internal("Cannot find home directory"))?;
        home.join(STATS_DIR)
    };

    ensure_dir(&stats_dir)?;
    Ok(stats_dir.join(STATS_FILE))
}

fn load_stats(root_dir: Option<&str>) -> Result<WritingStatsStore, AppError> {
    let path = get_stats_path(root_dir)?;
    if path.exists() {
        read_json(&path)
    } else {
        Ok(WritingStatsStore::default())
    }
}

fn save_stats(stats: &WritingStatsStore, root_dir: Option<&str>) -> Result<(), AppError> {
    let path = get_stats_path(root_dir)?;
    write_json(&path, stats)
}

#[command]
pub async fn start_writing_session(
    project_path: String,
    chapter_path: Option<String>,
    current_word_count: i32,
    root_dir: Option<String>,
) -> Result<String, AppError> {
    let root = root_dir.as_deref();
    let mut stats = load_stats(root)?;

    // End any existing session first
    if stats.current_session.is_some() {
        end_current_session(&mut stats, root)?;
    }

    let session = WritingSession::new(project_path, chapter_path, current_word_count);
    let session_id = session.session_id.clone();
    stats.current_session = Some(session);

    save_stats(&stats, root)?;
    Ok(session_id)
}

#[command]
pub async fn update_writing_session(
    current_word_count: i32,
    active_duration_secs: i64,
    idle_duration_secs: i64,
    root_dir: Option<String>,
) -> Result<(), AppError> {
    let root = root_dir.as_deref();
    let mut stats = load_stats(root)?;

    if let Some(ref mut session) = stats.current_session {
        session.end_word_count = Some(current_word_count);
        session.active_duration_secs = active_duration_secs;
        session.idle_duration_secs = idle_duration_secs;
        save_stats(&stats, root)?;
    }

    Ok(())
}

#[command]
pub async fn end_writing_session(
    final_word_count: i32,
    root_dir: Option<String>,
) -> Result<(), AppError> {
    let root = root_dir.as_deref();
    let mut stats = load_stats(root)?;

    if let Some(ref mut session) = stats.current_session {
        session.end_time = Some(chrono::Utc::now().timestamp_millis());
        session.end_word_count = Some(final_word_count);
    }

    end_current_session(&mut stats, root)?;
    save_stats(&stats, root)?;

    Ok(())
}

fn end_current_session(
    stats: &mut WritingStatsStore,
    _root_dir: Option<&str>,
) -> Result<(), AppError> {
    if let Some(session) = stats.current_session.take() {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let daily = stats.get_or_create_daily_stats(&today);

        let words_written = session.words_written();
        daily.word_count += words_written;
        daily.writing_duration_secs += session.active_duration_secs;
        daily.thinking_duration_secs += session.idle_duration_secs;
        daily.sessions.push(session);

        if words_written > 0 {
            stats.calculate_consecutive_days();
        }
    }
    Ok(())
}

#[command]
pub async fn record_words_written(
    word_count: i32,
    root_dir: Option<String>,
) -> Result<(), AppError> {
    let root = root_dir.as_deref();
    let mut stats = load_stats(root)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let daily = stats.get_or_create_daily_stats(&today);
    daily.word_count += word_count;

    if word_count > 0 {
        stats.calculate_consecutive_days();
    }

    save_stats(&stats, root)?;
    Ok(())
}

#[command]
pub async fn get_writing_stats(
    days: i32,
    root_dir: Option<String>,
) -> Result<Vec<DailyStats>, AppError> {
    let root = root_dir.as_deref();
    let stats = load_stats(root)?;
    let today = chrono::Utc::now();
    let today_str = today.format("%Y-%m-%d").to_string();

    let mut result = Vec::new();
    for i in 0..days {
        let date = (today - chrono::Duration::days(i as i64))
            .format("%Y-%m-%d")
            .to_string();

        let mut daily = if let Some(d) = stats.get_daily_stats(&date) {
            d.clone()
        } else {
            DailyStats::new(date.clone())
        };

        // Add current session's word count if it's today
        if date == today_str {
            if let Some(ref session) = stats.current_session {
                daily.word_count += session.words_written();
            }
        }

        result.push(daily);
    }

    result.reverse();
    Ok(result)
}

#[command]
pub async fn get_month_stats(
    year: i32,
    month: u32,
    root_dir: Option<String>,
) -> Result<Vec<DailyStats>, AppError> {
    let root = root_dir.as_deref();
    let stats = load_stats(root)?;
    let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Get all days in the month
    let days_in_month = chrono::NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    )
    .unwrap()
    .pred_opt()
    .unwrap()
    .day();

    let mut result = Vec::new();
    for day in 1..=days_in_month {
        let date = format!("{:04}-{:02}-{:02}", year, month, day);
        let mut daily = if let Some(d) = stats.get_daily_stats(&date) {
            d.clone()
        } else {
            DailyStats::new(date.clone())
        };

        // Add current session's word count if it's today
        if date == today_str {
            if let Some(ref session) = stats.current_session {
                daily.word_count += session.words_written();
            }
        }

        result.push(daily);
    }

    Ok(result)
}

#[command]
pub async fn get_year_stats(
    year: i32,
    root_dir: Option<String>,
) -> Result<Vec<MonthSummary>, AppError> {
    let root = root_dir.as_deref();
    let stats = load_stats(root)?;
    let today = chrono::Utc::now();
    let today_str = today.format("%Y-%m-%d").to_string();
    let current_session_words = stats
        .current_session
        .as_ref()
        .map(|s| s.words_written())
        .unwrap_or(0);

    let mut result = Vec::new();
    for month in 1..=12 {
        let prefix = format!("{:04}-{:02}", year, month);
        let month_stats: Vec<_> = stats
            .daily_stats
            .iter()
            .filter(|s| s.date.starts_with(&prefix))
            .collect();

        let mut total_words: i32 = month_stats.iter().map(|s| s.word_count).sum();
        let mut writing_days = month_stats.iter().filter(|s| s.word_count > 0).count() as i32;

        // Create daily word counts for heatmap
        let days_in_month = chrono::NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .unwrap()
        .pred_opt()
        .unwrap()
        .day();

        let mut daily_words = Vec::new();
        for day in 1..=days_in_month {
            let date = format!("{:04}-{:02}-{:02}", year, month, day);
            let mut words = stats
                .get_daily_stats(&date)
                .map(|s| s.word_count)
                .unwrap_or(0);

            // Add current session's word count if it's today
            if date == today_str && current_session_words > 0 {
                let today_saved_words = words;
                words += current_session_words;

                // Update totals for this month
                total_words += current_session_words;
                if today_saved_words == 0 && words > 0 {
                    writing_days += 1;
                }
            }

            daily_words.push(words);
        }

        result.push(MonthSummary {
            year,
            month: month as i32,
            total_words,
            writing_days,
            daily_words,
        });
    }

    Ok(result)
}

#[command]
pub async fn get_consecutive_days(root_dir: Option<String>) -> Result<i32, AppError> {
    let root = root_dir.as_deref();
    let stats = load_stats(root)?;
    Ok(stats.consecutive_days)
}

#[command]
pub async fn clear_writing_stats(root_dir: Option<String>) -> Result<(), AppError> {
    let root = root_dir.as_deref();
    let stats = WritingStatsStore::default();
    save_stats(&stats, root)?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonthSummary {
    pub year: i32,
    pub month: i32,
    pub total_words: i32,
    pub writing_days: i32,
    pub daily_words: Vec<i32>,
}
