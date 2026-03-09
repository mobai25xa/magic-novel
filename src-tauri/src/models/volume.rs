use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetadata {
    pub schema_version: i32,
    pub volume_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub chapter_order: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl VolumeMetadata {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            volume_id: uuid::Uuid::new_v4().to_string(),
            title,
            summary: None,
            chapter_order: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}
