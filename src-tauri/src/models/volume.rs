use serde::{Deserialize, Serialize};

pub const VOLUME_SCHEMA_VERSION: i32 = 2;
pub const DEFAULT_VOLUME_TARGET_WORDS: i32 = 75_000;

fn default_volume_target_words() -> i32 {
    DEFAULT_VOLUME_TARGET_WORDS
}

fn default_dramatic_goal() -> String {
    String::new()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VolumeStatus {
    #[default]
    Planned,
    Drafting,
    Complete,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VolumeMetadata {
    pub schema_version: i32,
    pub volume_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub target_words: i32,
    pub dramatic_goal: String,
    pub status: VolumeStatus,
    pub chapter_order: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct VolumeMetadataDe {
    schema_version: i32,
    volume_id: String,
    title: String,
    summary: Option<String>,
    target_words: i32,
    dramatic_goal: String,
    status: VolumeStatus,
    chapter_order: Vec<String>,
    created_at: i64,
    updated_at: i64,
}

impl<'de> Deserialize<'de> for VolumeMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = VolumeMetadataDe::deserialize(deserializer)?;
        if raw.schema_version != VOLUME_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported volume schema_version {}; expected {}",
                raw.schema_version, VOLUME_SCHEMA_VERSION
            )));
        }

        Ok(Self {
            schema_version: raw.schema_version,
            volume_id: raw.volume_id,
            title: raw.title,
            summary: raw.summary,
            target_words: raw.target_words,
            dramatic_goal: raw.dramatic_goal,
            status: raw.status,
            chapter_order: raw.chapter_order,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

impl VolumeMetadata {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: VOLUME_SCHEMA_VERSION,
            volume_id: uuid::Uuid::new_v4().to_string(),
            title,
            summary: None,
            target_words: default_volume_target_words(),
            dramatic_goal: default_dramatic_goal(),
            status: VolumeStatus::Planned,
            chapter_order: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_legacy_volume_schema() {
        let raw = serde_json::json!({
            "schema_version": 1,
            "volume_id": "volume-1",
            "title": "卷一",
            "target_words": DEFAULT_VOLUME_TARGET_WORDS,
            "dramatic_goal": "",
            "status": "planned",
            "chapter_order": [],
            "created_at": 100,
            "updated_at": 200
        });

        let err = serde_json::from_value::<VolumeMetadata>(raw).expect_err("legacy volume");
        assert!(err
            .to_string()
            .contains("unsupported volume schema_version"));
    }

    #[test]
    fn rejects_missing_current_volume_fields() {
        let raw = serde_json::json!({
            "schema_version": VOLUME_SCHEMA_VERSION,
            "volume_id": "volume-1",
            "title": "卷一",
            "chapter_order": [],
            "created_at": 100,
            "updated_at": 200
        });

        let err =
            serde_json::from_value::<VolumeMetadata>(raw).expect_err("missing current fields");
        assert!(err.to_string().contains("target_words"));
    }
}
