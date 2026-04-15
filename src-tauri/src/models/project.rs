use serde::{Deserialize, Serialize};

pub const PROJECT_SCHEMA_VERSION: i32 = 3;
pub const DEFAULT_TARGET_TOTAL_WORDS: i32 = 300_000;
pub const DEFAULT_PLANNED_VOLUMES: i32 = 4;
pub const DEFAULT_TARGET_WORDS_PER_CHAPTER: i32 = 3_000;

fn normalize_genres(mut input: Vec<String>) -> Vec<String> {
    for item in &mut input {
        *item = item.trim().to_string();
    }
    input.retain(|item| !item.is_empty());

    let mut out = Vec::new();
    for item in input {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

fn normalize_project_type(input: Vec<String>) -> Vec<String> {
    let mut normalized = normalize_genres(input);
    normalized.retain(|genre| {
        !matches!(
            genre.as_str(),
            "novel" | "script" | "essay" | "diary" | "business" | "academic" | "other"
        )
    });
    normalized
}

fn deserialize_project_type<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let genres = match value {
        serde_json::Value::Null => vec![],
        serde_json::Value::String(item) => vec![item],
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(|value| value.to_string()))
            .collect(),
        _ => vec![],
    };

    Ok(normalize_project_type(genres))
}

fn normalize_labels(mut input: Vec<String>) -> Vec<String> {
    for item in &mut input {
        *item = item.trim().to_string();
    }
    input.retain(|item| !item.is_empty());

    let mut out = Vec::new();
    for item in input {
        if !out.contains(&item) {
            out.push(item);
        }
    }

    out
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn sanitize_positive(value: Option<i32>) -> Option<i32> {
    value.filter(|candidate| *candidate > 0)
}

fn default_target_total_words() -> i32 {
    DEFAULT_TARGET_TOTAL_WORDS
}

fn default_narrative_pov() -> String {
    "third_limited".to_string()
}

fn default_audience() -> String {
    "general".to_string()
}

fn computed_target_words_per_volume(total_words: i32, planned_volumes: Option<i32>) -> Option<i32> {
    planned_volumes
        .filter(|volumes| *volumes > 0)
        .map(|volumes| (total_words.max(1) / volumes.max(1)).max(1))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectBootstrapState {
    #[default]
    ScaffoldReady,
    BootstrapRunning,
    PartiallyGenerated,
    ReadyForReview,
    ReadyToWrite,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub schema_version: i32,
    pub project_id: String,
    pub name: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    pub project_type: Vec<String>,
    pub target_total_words: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned_volumes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_words_per_volume: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_words_per_chapter: Option<i32>,
    pub narrative_pov: String,
    pub tone: Vec<String>,
    pub audience: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_core: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protagonist_anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_inspiration_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning_bundle_version: Option<i32>,
    pub bootstrap_state: ProjectBootstrapState,
    pub bootstrap_updated_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_min_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProjectMetadataV2De {
    schema_version: i32,
    project_id: String,
    name: String,
    author: String,
    description: Option<String>,
    cover_image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_project_type")]
    project_type: Vec<String>,
    created_at: i64,
    updated_at: i64,
    app_min_version: Option<String>,
    last_opened_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProjectMetadataV3De {
    schema_version: i32,
    project_id: String,
    name: String,
    author: String,
    description: Option<String>,
    cover_image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_project_type")]
    project_type: Vec<String>,
    #[serde(default = "default_target_total_words")]
    target_total_words: i32,
    #[serde(default)]
    planned_volumes: Option<i32>,
    #[serde(default)]
    target_words_per_volume: Option<i32>,
    #[serde(default)]
    target_words_per_chapter: Option<i32>,
    #[serde(default = "default_narrative_pov")]
    narrative_pov: String,
    #[serde(default)]
    tone: Vec<String>,
    #[serde(default = "default_audience")]
    audience: String,
    #[serde(default)]
    story_core: Option<String>,
    #[serde(default)]
    protagonist_anchor: Option<String>,
    #[serde(default)]
    conflict_anchor: Option<String>,
    #[serde(default)]
    origin_inspiration_session_id: Option<String>,
    #[serde(default)]
    planning_bundle_version: Option<i32>,
    #[serde(default)]
    bootstrap_state: ProjectBootstrapState,
    #[serde(default)]
    bootstrap_updated_at: i64,
    created_at: i64,
    updated_at: i64,
    app_min_version: Option<String>,
    last_opened_at: Option<i64>,
}

impl<'de> Deserialize<'de> for ProjectMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let schema_version = raw
            .get("schema_version")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| serde::de::Error::missing_field("schema_version"))?;

        match schema_version {
            2 => {
                let legacy: ProjectMetadataV2De =
                    serde_json::from_value(raw).map_err(serde::de::Error::custom)?;
                Ok(Self::from_v2(legacy))
            }
            version if version == i64::from(PROJECT_SCHEMA_VERSION) => {
                let current: ProjectMetadataV3De =
                    serde_json::from_value(raw).map_err(serde::de::Error::custom)?;
                Ok(Self::from_v3(current))
            }
            _ => Err(serde::de::Error::custom(format!(
                "unsupported project schema_version {}; expected 2 or {}",
                schema_version, PROJECT_SCHEMA_VERSION
            ))),
        }
    }
}

impl ProjectMetadata {
    fn from_v2(raw: ProjectMetadataV2De) -> Self {
        debug_assert_eq!(raw.schema_version, 2);
        let mut project = Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: raw.project_id,
            name: raw.name,
            author: raw.author,
            description: raw.description,
            cover_image: raw.cover_image,
            project_type: raw.project_type,
            target_total_words: default_target_total_words(),
            planned_volumes: None,
            target_words_per_volume: None,
            target_words_per_chapter: None,
            narrative_pov: default_narrative_pov(),
            tone: Vec::new(),
            audience: default_audience(),
            story_core: None,
            protagonist_anchor: None,
            conflict_anchor: None,
            origin_inspiration_session_id: None,
            planning_bundle_version: None,
            bootstrap_state: ProjectBootstrapState::ScaffoldReady,
            bootstrap_updated_at: raw.updated_at.max(raw.created_at),
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            app_min_version: raw.app_min_version,
            last_opened_at: raw.last_opened_at,
        };
        project.ensure_defaults();
        project
    }

    fn from_v3(raw: ProjectMetadataV3De) -> Self {
        let mut project = Self {
            schema_version: raw.schema_version,
            project_id: raw.project_id,
            name: raw.name,
            author: raw.author,
            description: raw.description,
            cover_image: raw.cover_image,
            project_type: raw.project_type,
            target_total_words: raw.target_total_words,
            planned_volumes: raw.planned_volumes,
            target_words_per_volume: raw.target_words_per_volume,
            target_words_per_chapter: raw.target_words_per_chapter,
            narrative_pov: raw.narrative_pov,
            tone: raw.tone,
            audience: raw.audience,
            story_core: raw.story_core,
            protagonist_anchor: raw.protagonist_anchor,
            conflict_anchor: raw.conflict_anchor,
            origin_inspiration_session_id: raw.origin_inspiration_session_id,
            planning_bundle_version: raw.planning_bundle_version,
            bootstrap_state: raw.bootstrap_state,
            bootstrap_updated_at: raw.bootstrap_updated_at,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            app_min_version: raw.app_min_version,
            last_opened_at: raw.last_opened_at,
        };
        project.ensure_defaults();
        project
    }

    pub fn new(
        name: String,
        author: String,
        project_type: Option<Vec<String>>,
        cover_image: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let mut project = Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: uuid::Uuid::new_v4().to_string(),
            name,
            author,
            description: None,
            cover_image,
            project_type: normalize_project_type(project_type.unwrap_or_default()),
            target_total_words: default_target_total_words(),
            planned_volumes: None,
            target_words_per_volume: None,
            target_words_per_chapter: None,
            narrative_pov: default_narrative_pov(),
            tone: Vec::new(),
            audience: default_audience(),
            story_core: None,
            protagonist_anchor: None,
            conflict_anchor: None,
            origin_inspiration_session_id: None,
            planning_bundle_version: None,
            bootstrap_state: ProjectBootstrapState::ScaffoldReady,
            bootstrap_updated_at: now,
            created_at: now,
            updated_at: now,
            app_min_version: None,
            last_opened_at: Some(now),
        };
        project.ensure_defaults();
        project
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_creation_inputs(
        &mut self,
        description: Option<String>,
        target_total_words: Option<i32>,
        planned_volumes: Option<i32>,
        target_words_per_volume: Option<i32>,
        target_words_per_chapter: Option<i32>,
        narrative_pov: Option<String>,
        tone: Option<Vec<String>>,
        audience: Option<String>,
    ) {
        let next_total_words =
            sanitize_positive(target_total_words).unwrap_or(self.target_total_words);
        let next_planned_volumes = sanitize_positive(planned_volumes).or(self.planned_volumes);

        if description.is_some() {
            self.description = normalize_optional_string(description);
        }
        if let Some(target_total_words) = sanitize_positive(target_total_words) {
            self.target_total_words = target_total_words;
        }
        if planned_volumes.is_some() {
            self.planned_volumes = sanitize_positive(planned_volumes);
        }
        if target_words_per_chapter.is_some() {
            self.target_words_per_chapter = sanitize_positive(target_words_per_chapter);
        }
        if let Some(narrative_pov) = narrative_pov {
            let trimmed = narrative_pov.trim();
            if !trimmed.is_empty() {
                self.narrative_pov = trimmed.to_string();
            }
        }
        if let Some(tone) = tone {
            self.tone = normalize_labels(tone);
        }
        if let Some(audience) = audience {
            let trimmed = audience.trim();
            if !trimmed.is_empty() {
                self.audience = trimmed.to_string();
            }
        }

        if target_words_per_volume.is_some() {
            self.target_words_per_volume = sanitize_positive(target_words_per_volume);
        } else if target_total_words.is_some() || planned_volumes.is_some() {
            self.target_words_per_volume =
                computed_target_words_per_volume(next_total_words, next_planned_volumes);
        }

        let now = chrono::Utc::now().timestamp_millis();
        self.bootstrap_state = ProjectBootstrapState::ScaffoldReady;
        self.bootstrap_updated_at = now;
        self.updated_at = now;
        self.ensure_defaults();
    }

    pub fn ensure_defaults(&mut self) {
        self.schema_version = PROJECT_SCHEMA_VERSION;
        self.project_type = normalize_project_type(std::mem::take(&mut self.project_type));
        self.tone = normalize_labels(std::mem::take(&mut self.tone));
        self.description = normalize_optional_string(self.description.take());
        self.story_core = normalize_optional_string(self.story_core.take());
        self.protagonist_anchor = normalize_optional_string(self.protagonist_anchor.take());
        self.conflict_anchor = normalize_optional_string(self.conflict_anchor.take());
        self.origin_inspiration_session_id =
            normalize_optional_string(self.origin_inspiration_session_id.take());

        if self.target_total_words <= 0 {
            self.target_total_words = default_target_total_words();
        }
        self.planned_volumes = sanitize_positive(self.planned_volumes);
        self.target_words_per_volume =
            sanitize_positive(self.target_words_per_volume).or_else(|| {
                computed_target_words_per_volume(self.target_total_words, self.planned_volumes)
            });
        self.target_words_per_chapter = sanitize_positive(self.target_words_per_chapter);
        if self.narrative_pov.trim().is_empty() {
            self.narrative_pov = default_narrative_pov();
        }
        if self.audience.trim().is_empty() {
            self.audience = default_audience();
        }
        if self.bootstrap_updated_at <= 0 {
            self.bootstrap_updated_at = self.updated_at.max(self.created_at);
        }
    }

    pub fn mark_opened_now(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();
        self.last_opened_at = Some(now);
        if self.bootstrap_updated_at <= 0 {
            self.bootstrap_updated_at = now;
        }
        self.ensure_defaults();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FileNode {
    #[serde(rename = "dir")]
    Dir {
        name: String,
        path: String,
        children: Vec<FileNode>,
        created_at: i64,
        updated_at: i64,
    },
    #[serde(rename = "chapter")]
    Chapter {
        name: String,
        path: String,
        chapter_id: String,
        title: String,
        text_length_no_whitespace: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        word_count: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        created_at: i64,
        updated_at: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub project: ProjectMetadata,
    pub path: String,
    pub tree: Vec<FileNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_legacy_project_schema_with_nullable_planning_fields() {
        let raw = serde_json::json!({
            "schema_version": 2,
            "project_id": "project-1",
            "name": "Legacy",
            "author": "Tester",
            "project_type": "mystery",
            "created_at": 100,
            "updated_at": 200,
            "last_opened_at": 300
        });

        let project = serde_json::from_value::<ProjectMetadata>(raw).expect("legacy project");

        assert_eq!(project.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(project.project_type, vec!["mystery".to_string()]);
        assert_eq!(project.target_total_words, DEFAULT_TARGET_TOTAL_WORDS);
        assert_eq!(project.planned_volumes, None);
        assert_eq!(project.target_words_per_volume, None);
        assert_eq!(project.target_words_per_chapter, None);
        assert_eq!(project.narrative_pov, "third_limited");
        assert_eq!(project.audience, "general");
        assert_eq!(
            project.bootstrap_state,
            ProjectBootstrapState::ScaffoldReady
        );
        assert_eq!(project.bootstrap_updated_at, 200);
    }

    #[test]
    fn normalizes_legacy_project_type_shapes_and_drops_obsolete_values() {
        let raw = serde_json::json!({
            "schema_version": PROJECT_SCHEMA_VERSION,
            "project_id": "project-1",
            "name": "Legacy",
            "author": "Tester",
            "project_type": [" fantasy ", "novel", "fantasy", ""],
            "created_at": 100,
            "updated_at": 200,
            "last_opened_at": 300
        });

        let project =
            serde_json::from_value::<ProjectMetadata>(raw).expect("legacy project_type shape");
        assert_eq!(project.project_type, vec!["fantasy".to_string()]);
    }

    #[test]
    fn rejects_unknown_project_schema() {
        let raw = serde_json::json!({
            "schema_version": 99,
            "project_id": "project-1",
            "name": "Legacy",
            "author": "Tester",
            "project_type": ["mystery"],
            "created_at": 100,
            "updated_at": 200
        });

        let err = serde_json::from_value::<ProjectMetadata>(raw).expect_err("unknown schema");
        assert!(err
            .to_string()
            .contains("unsupported project schema_version"));
    }

    #[test]
    fn apply_creation_inputs_normalizes_and_recomputes_targets() {
        let mut project = ProjectMetadata::new(
            "Novel".to_string(),
            "Tester".to_string(),
            Some(vec![" mystery ".to_string(), "mystery".to_string()]),
            None,
        );

        project.apply_creation_inputs(
            Some("  premise  ".to_string()),
            Some(240_000),
            Some(6),
            None,
            Some(4_000),
            Some(" first_person ".to_string()),
            Some(vec![
                " tense ".to_string(),
                "tense".to_string(),
                "".to_string(),
            ]),
            Some(" young_adult ".to_string()),
        );

        assert_eq!(project.description.as_deref(), Some("premise"));
        assert_eq!(project.target_total_words, 240_000);
        assert_eq!(project.planned_volumes, Some(6));
        assert_eq!(project.target_words_per_volume, Some(40_000));
        assert_eq!(project.target_words_per_chapter, Some(4_000));
        assert_eq!(project.narrative_pov, "first_person");
        assert_eq!(project.tone, vec!["tense".to_string()]);
        assert_eq!(project.audience, "young_adult");
        assert_eq!(
            project.bootstrap_state,
            ProjectBootstrapState::ScaffoldReady
        );
    }

    #[test]
    fn new_project_leaves_planning_shape_undecided() {
        let project = ProjectMetadata::new(
            "Novel".to_string(),
            "Tester".to_string(),
            Some(vec!["mystery".to_string()]),
            None,
        );

        assert_eq!(project.planned_volumes, None);
        assert_eq!(project.target_words_per_volume, None);
        assert_eq!(project.target_words_per_chapter, None);
    }
}
