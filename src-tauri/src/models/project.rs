use serde::{Deserialize, Serialize};

pub const PROJECT_SCHEMA_VERSION: i32 = 3;
pub const DEFAULT_TARGET_TOTAL_WORDS: i32 = 300_000;
pub const DEFAULT_PLANNED_VOLUMES: i32 = 4;
pub const DEFAULT_TARGET_WORDS_PER_CHAPTER: i32 = 3_000;

fn normalize_genres(mut input: Vec<String>) -> Vec<String> {
    for s in input.iter_mut() {
        *s = s.trim().to_string();
    }
    input.retain(|s| !s.is_empty());

    let mut out: Vec<String> = Vec::new();
    for s in input {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

fn normalize_labels(mut input: Vec<String>) -> Vec<String> {
    for label in input.iter_mut() {
        *label = label.trim().to_string();
    }
    input.retain(|label| !label.is_empty());

    let mut out = Vec::new();
    for label in input {
        if !out.contains(&label) {
            out.push(label);
        }
    }

    out
}

fn default_target_total_words() -> i32 {
    DEFAULT_TARGET_TOTAL_WORDS
}

fn default_planned_volumes() -> i32 {
    DEFAULT_PLANNED_VOLUMES
}

fn default_target_words_per_volume() -> i32 {
    DEFAULT_TARGET_TOTAL_WORDS / DEFAULT_PLANNED_VOLUMES
}

fn default_target_words_per_chapter() -> i32 {
    DEFAULT_TARGET_WORDS_PER_CHAPTER
}

fn default_narrative_pov() -> String {
    "third_limited".to_string()
}

fn default_audience() -> String {
    "general".to_string()
}

fn computed_target_words_per_volume(total_words: i32, planned_volumes: i32) -> i32 {
    let total_words = if total_words > 0 {
        total_words
    } else {
        DEFAULT_TARGET_TOTAL_WORDS
    };
    let planned_volumes = if planned_volumes > 0 {
        planned_volumes
    } else {
        DEFAULT_PLANNED_VOLUMES
    };

    (total_words / planned_volumes.max(1)).max(1)
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
    pub planned_volumes: i32,
    pub target_words_per_volume: i32,
    pub target_words_per_chapter: i32,
    pub narrative_pov: String,
    pub tone: Vec<String>,
    pub audience: String,
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
struct ProjectMetadataDe {
    schema_version: i32,
    project_id: String,
    name: String,
    author: String,
    description: Option<String>,
    cover_image: Option<String>,
    project_type: Vec<String>,
    target_total_words: i32,
    planned_volumes: i32,
    target_words_per_volume: i32,
    target_words_per_chapter: i32,
    narrative_pov: String,
    tone: Vec<String>,
    audience: String,
    bootstrap_state: ProjectBootstrapState,
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
        let raw = ProjectMetadataDe::deserialize(deserializer)?;
        if raw.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported project schema_version {}; expected {}",
                raw.schema_version, PROJECT_SCHEMA_VERSION
            )));
        }

        Ok(Self {
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
            bootstrap_state: raw.bootstrap_state,
            bootstrap_updated_at: raw.bootstrap_updated_at,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            app_min_version: raw.app_min_version,
            last_opened_at: raw.last_opened_at,
        })
    }
}

impl ProjectMetadata {
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
            project_type: normalize_genres(project_type.unwrap_or_default()),
            target_total_words: default_target_total_words(),
            planned_volumes: default_planned_volumes(),
            target_words_per_volume: default_target_words_per_volume(),
            target_words_per_chapter: default_target_words_per_chapter(),
            narrative_pov: default_narrative_pov(),
            tone: vec![],
            audience: default_audience(),
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
        let should_recompute_target_words_per_volume = target_words_per_volume.is_none()
            && (target_total_words.is_some() || planned_volumes.is_some());

        if let Some(description) = description {
            let description = description.trim();
            self.description = if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            };
        }
        if let Some(target_total_words) = target_total_words.filter(|value| *value > 0) {
            self.target_total_words = target_total_words;
        }
        if let Some(planned_volumes) = planned_volumes.filter(|value| *value > 0) {
            self.planned_volumes = planned_volumes;
        }
        if let Some(target_words_per_chapter) = target_words_per_chapter.filter(|value| *value > 0)
        {
            self.target_words_per_chapter = target_words_per_chapter;
        }
        if let Some(narrative_pov) = narrative_pov {
            let narrative_pov = narrative_pov.trim();
            if !narrative_pov.is_empty() {
                self.narrative_pov = narrative_pov.to_string();
            }
        }
        if let Some(tone) = tone {
            self.tone = normalize_labels(tone);
        }
        if let Some(audience) = audience {
            let audience = audience.trim();
            if !audience.is_empty() {
                self.audience = audience.to_string();
            }
        }

        if let Some(target_words_per_volume) = target_words_per_volume.filter(|value| *value > 0) {
            self.target_words_per_volume = target_words_per_volume;
        } else if should_recompute_target_words_per_volume {
            self.target_words_per_volume =
                computed_target_words_per_volume(self.target_total_words, self.planned_volumes);
        }

        let now = chrono::Utc::now().timestamp_millis();
        self.bootstrap_state = ProjectBootstrapState::ScaffoldReady;
        self.bootstrap_updated_at = now;
        self.updated_at = now;
        self.ensure_defaults();
    }

    pub fn ensure_defaults(&mut self) {
        self.schema_version = PROJECT_SCHEMA_VERSION;
        self.project_type = normalize_genres(std::mem::take(&mut self.project_type));
        self.tone = normalize_labels(std::mem::take(&mut self.tone));

        if self.target_total_words <= 0 {
            self.target_total_words = default_target_total_words();
        }
        if self.planned_volumes <= 0 {
            self.planned_volumes = default_planned_volumes();
        }
        if self.target_words_per_volume <= 0 {
            self.target_words_per_volume =
                computed_target_words_per_volume(self.target_total_words, self.planned_volumes);
        }
        if self.target_words_per_chapter <= 0 {
            self.target_words_per_chapter = default_target_words_per_chapter();
        }
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
    fn rejects_legacy_project_schema() {
        let raw = serde_json::json!({
            "schema_version": 2,
            "project_id": "project-1",
            "name": "Legacy",
            "author": "Tester",
            "project_type": ["mystery"],
            "target_total_words": DEFAULT_TARGET_TOTAL_WORDS,
            "planned_volumes": DEFAULT_PLANNED_VOLUMES,
            "target_words_per_volume": DEFAULT_TARGET_TOTAL_WORDS / DEFAULT_PLANNED_VOLUMES,
            "target_words_per_chapter": DEFAULT_TARGET_WORDS_PER_CHAPTER,
            "narrative_pov": "third_limited",
            "tone": [],
            "audience": "general",
            "bootstrap_state": "scaffold_ready",
            "bootstrap_updated_at": 100,
            "created_at": 100,
            "updated_at": 200,
            "last_opened_at": 300
        });

        let err = serde_json::from_value::<ProjectMetadata>(raw).expect_err("legacy project");
        assert!(err
            .to_string()
            .contains("unsupported project schema_version"));
    }

    #[test]
    fn rejects_legacy_project_type_shape() {
        let raw = serde_json::json!({
            "schema_version": PROJECT_SCHEMA_VERSION,
            "project_id": "project-1",
            "name": "Legacy",
            "author": "Tester",
            "project_type": "mystery",
            "target_total_words": DEFAULT_TARGET_TOTAL_WORDS,
            "planned_volumes": DEFAULT_PLANNED_VOLUMES,
            "target_words_per_volume": DEFAULT_TARGET_TOTAL_WORDS / DEFAULT_PLANNED_VOLUMES,
            "target_words_per_chapter": DEFAULT_TARGET_WORDS_PER_CHAPTER,
            "narrative_pov": "third_limited",
            "tone": [],
            "audience": "general",
            "bootstrap_state": "scaffold_ready",
            "bootstrap_updated_at": 100,
            "created_at": 100,
            "updated_at": 200,
            "last_opened_at": 300
        });

        let err =
            serde_json::from_value::<ProjectMetadata>(raw).expect_err("legacy project_type shape");
        assert!(err.to_string().contains("invalid type"));
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
        assert_eq!(project.planned_volumes, 6);
        assert_eq!(project.target_words_per_volume, 40_000);
        assert_eq!(project.target_words_per_chapter, 4_000);
        assert_eq!(project.narrative_pov, "first_person");
        assert_eq!(project.tone, vec!["tense".to_string()]);
        assert_eq!(project.audience, "young_adult");
        assert_eq!(
            project.bootstrap_state,
            ProjectBootstrapState::ScaffoldReady
        );
    }
}
