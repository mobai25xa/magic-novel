use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String, // YYYY-MM-DD format
    pub word_count: i32,
    pub writing_duration_secs: i64,
    pub thinking_duration_secs: i64,
    pub sessions: Vec<WritingSession>,
}

impl DailyStats {
    pub fn new(date: String) -> Self {
        Self {
            date,
            word_count: 0,
            writing_duration_secs: 0,
            thinking_duration_secs: 0,
            sessions: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingSession {
    pub session_id: String,
    pub project_path: String,
    pub chapter_path: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub start_word_count: i32,
    pub end_word_count: Option<i32>,
    pub active_duration_secs: i64,
    pub idle_duration_secs: i64,
}

impl WritingSession {
    pub fn new(project_path: String, chapter_path: Option<String>, start_word_count: i32) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            project_path,
            chapter_path,
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
            start_word_count,
            end_word_count: None,
            active_duration_secs: 0,
            idle_duration_secs: 0,
        }
    }

    pub fn words_written(&self) -> i32 {
        match self.end_word_count {
            Some(end) => (end - self.start_word_count).max(0),
            None => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingStatsStore {
    pub schema_version: i32,
    pub daily_stats: Vec<DailyStats>,
    pub current_session: Option<WritingSession>,
    pub consecutive_days: i32,
    pub last_writing_date: Option<String>,
}

impl Default for WritingStatsStore {
    fn default() -> Self {
        Self {
            schema_version: 1,
            daily_stats: vec![],
            current_session: None,
            consecutive_days: 0,
            last_writing_date: None,
        }
    }
}

impl WritingStatsStore {
    pub fn get_daily_stats(&self, date: &str) -> Option<&DailyStats> {
        self.daily_stats.iter().find(|s| s.date == date)
    }

    pub fn get_or_create_daily_stats(&mut self, date: &str) -> &mut DailyStats {
        if !self.daily_stats.iter().any(|s| s.date == date) {
            self.daily_stats.push(DailyStats::new(date.to_string()));
        }
        self.daily_stats
            .iter_mut()
            .find(|s| s.date == date)
            .unwrap()
    }

    pub fn calculate_consecutive_days(&mut self) {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        match &self.last_writing_date {
            Some(last_date) => {
                if last_date == &today {
                    // Already counted today
                } else if last_date == &yesterday {
                    // Continue streak
                    self.consecutive_days += 1;
                    self.last_writing_date = Some(today);
                } else {
                    // Streak broken
                    self.consecutive_days = 1;
                    self.last_writing_date = Some(today);
                }
            }
            None => {
                self.consecutive_days = 1;
                self.last_writing_date = Some(today);
            }
        }
    }
}
