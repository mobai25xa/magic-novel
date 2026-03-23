use serde::{Deserialize, Serialize};

pub const CHAPTER_SCHEMA_VERSION: i32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChapterStatus {
    Planned,
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

#[derive(Debug, Clone, Serialize)]
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
    pub plot_goal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotional_goal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_assets: Option<Vec<ChapterAssetRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cursor_position: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct ChapterDe {
    schema_version: i32,
    id: String,
    title: String,
    content: serde_json::Value,
    counts: ChapterCounts,
    target_words: Option<i32>,
    status: Option<ChapterStatus>,
    summary: Option<String>,
    plot_goal: Option<String>,
    emotional_goal: Option<String>,
    tags: Option<Vec<String>>,
    pinned_assets: Option<Vec<ChapterAssetRef>>,
    last_cursor_position: Option<i32>,
    created_at: i64,
    updated_at: i64,
}

impl<'de> Deserialize<'de> for Chapter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = ChapterDe::deserialize(deserializer)?;
        if raw.schema_version != CHAPTER_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported chapter schema_version {}; expected {}",
                raw.schema_version, CHAPTER_SCHEMA_VERSION
            )));
        }

        Ok(Self {
            schema_version: raw.schema_version,
            id: raw.id,
            title: raw.title,
            content: raw.content,
            counts: raw.counts,
            target_words: raw.target_words,
            status: raw.status,
            summary: raw.summary,
            plot_goal: raw.plot_goal,
            emotional_goal: raw.emotional_goal,
            tags: raw.tags,
            pinned_assets: raw.pinned_assets,
            last_cursor_position: raw.last_cursor_position,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

impl Chapter {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: CHAPTER_SCHEMA_VERSION,
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
            plot_goal: None,
            emotional_goal: None,
            tags: None,
            pinned_assets: None,
            last_cursor_position: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_legacy_chapter_schema() {
        let raw = serde_json::json!({
            "schema_version": 1,
            "id": "chapter-1",
            "title": "第一章",
            "content": { "type": "doc", "content": [] },
            "counts": {
                "text_length_no_whitespace": 0,
                "word_count": null,
                "algorithm_version": 1,
                "last_calculated_at": 100
            },
            "created_at": 100,
            "updated_at": 200
        });

        let err = serde_json::from_value::<Chapter>(raw).expect_err("legacy chapter");
        assert!(err
            .to_string()
            .contains("unsupported chapter schema_version"));
    }
}
