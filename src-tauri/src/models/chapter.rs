use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChapterStatus {
    Draft,
    Revised,
    Final,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterCounts {
    pub text_length_no_whitespace: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<i32>,
    pub algorithm_version: i32,
    pub last_calculated_at: i64,
}

impl Default for ChapterCounts {
    fn default() -> Self {
        Self {
            text_length_no_whitespace: 0,
            word_count: None,
            algorithm_version: 1,
            last_calculated_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterAssetRef {
    pub kind: crate::models::AssetKind,
    pub asset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub schema_version: i32,
    pub id: String,
    pub title: String,
    pub content: serde_json::Value,
    pub counts: ChapterCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_words: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ChapterStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_assets: Option<Vec<ChapterAssetRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cursor_position: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Chapter {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content: serde_json::json!({
                "type": "doc",
                "content": []
            }),
            counts: ChapterCounts::default(),
            target_words: None,
            status: Some(ChapterStatus::Draft),
            summary: None,
            tags: None,
            pinned_assets: None,
            last_cursor_position: None,
            created_at: now,
            updated_at: now,
        }
    }
}
