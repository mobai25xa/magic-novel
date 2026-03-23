use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapArtifactKind {
    StoryBlueprint,
    ThemeNotes,
    ProtagonistSeed,
    CounterpartSeed,
    WorldSummary,
    MainPlotline,
    VolumePlan,
    ChapterBacklog,
}

impl BootstrapArtifactKind {
    pub fn all() -> &'static [Self] {
        &[
            Self::StoryBlueprint,
            Self::ThemeNotes,
            Self::ProtagonistSeed,
            Self::CounterpartSeed,
            Self::WorldSummary,
            Self::MainPlotline,
            Self::VolumePlan,
            Self::ChapterBacklog,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StoryBlueprint => "story_blueprint",
            Self::ThemeNotes => "theme_notes",
            Self::ProtagonistSeed => "protagonist_seed",
            Self::CounterpartSeed => "counterpart_seed",
            Self::WorldSummary => "world_summary",
            Self::MainPlotline => "main_plotline",
            Self::VolumePlan => "volume_plan",
            Self::ChapterBacklog => "chapter_backlog",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::StoryBlueprint => "Story Blueprint",
            Self::ThemeNotes => "Theme Notes",
            Self::ProtagonistSeed => "Protagonist Seed",
            Self::CounterpartSeed => "Counterpart Seed",
            Self::WorldSummary => "World Summary",
            Self::MainPlotline => "Main Plotline",
            Self::VolumePlan => "Volume Plan",
            Self::ChapterBacklog => "Chapter Backlog",
        }
    }

    pub fn knowledge_path(&self) -> &'static str {
        match self {
            Self::StoryBlueprint => ".magic_novel/planning/story_blueprint.md",
            Self::ThemeNotes => ".magic_novel/planning/theme_notes.md",
            Self::ProtagonistSeed => ".magic_novel/characters/protagonist.md",
            Self::CounterpartSeed => ".magic_novel/characters/counterpart.md",
            Self::WorldSummary => ".magic_novel/world/world_summary.md",
            Self::MainPlotline => ".magic_novel/plot/main_plotline.md",
            Self::VolumePlan => ".magic_novel/planning/volume_plan.md",
            Self::ChapterBacklog => ".magic_novel/planning/chapter_backlog.md",
        }
    }

    pub fn order_key(&self) -> usize {
        match self {
            Self::StoryBlueprint => 0,
            Self::ThemeNotes => 1,
            Self::ProtagonistSeed => 2,
            Self::CounterpartSeed => 3,
            Self::WorldSummary => 4,
            Self::MainPlotline => 5,
            Self::VolumePlan => 6,
            Self::ChapterBacklog => 7,
        }
    }

    pub fn from_step_name(raw: &str) -> Option<Self> {
        match raw.trim() {
            "story_blueprint" => Some(Self::StoryBlueprint),
            "theme_notes" => Some(Self::ThemeNotes),
            "protagonist_seed" => Some(Self::ProtagonistSeed),
            "counterpart_seed" => Some(Self::CounterpartSeed),
            "world_summary" => Some(Self::WorldSummary),
            "main_plotline" => Some(Self::MainPlotline),
            "volume_plan" => Some(Self::VolumePlan),
            "chapter_backlog" => Some(Self::ChapterBacklog),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapPromptInput {
    pub project_name: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub genres: Vec<String>,
    pub target_total_words: i32,
    pub planned_volumes: i32,
    pub target_words_per_volume: i32,
    pub target_words_per_chapter: i32,
    pub narrative_pov: String,
    #[serde(default)]
    pub tone: Vec<String>,
    pub audience: String,
    pub creation_brief: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protagonist_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterpart_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_direction: Option<String>,
}

impl BootstrapPromptInput {
    pub fn from_project(
        project: &crate::models::ProjectMetadata,
        creation_brief: String,
        protagonist_seed: Option<String>,
        counterpart_seed: Option<String>,
        world_seed: Option<String>,
        ending_direction: Option<String>,
    ) -> Self {
        Self {
            project_name: project.name.clone(),
            author: project.author.clone(),
            description: project.description.clone().unwrap_or_default(),
            genres: project.project_type.clone(),
            target_total_words: project.target_total_words,
            planned_volumes: project.planned_volumes,
            target_words_per_volume: project.target_words_per_volume,
            target_words_per_chapter: project.target_words_per_chapter,
            narrative_pov: project.narrative_pov.clone(),
            tone: project.tone.clone(),
            audience: project.audience.clone(),
            creation_brief,
            protagonist_seed,
            counterpart_seed,
            world_seed,
            ending_direction,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapChapterPlan {
    pub title: String,
    pub summary: String,
    pub plot_goal: String,
    pub emotional_goal: String,
    pub target_words: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapVolumePlan {
    pub title: String,
    pub summary: String,
    pub dramatic_goal: String,
    pub target_words: i32,
    #[serde(default)]
    pub chapters: Vec<BootstrapChapterPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedArtifact {
    pub kind: BootstrapArtifactKind,
    pub title: String,
    pub content: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapArtifactFailure {
    pub kind: BootstrapArtifactKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapGenerationResult {
    pub generator_name: String,
    #[serde(default)]
    pub artifacts: Vec<GeneratedArtifact>,
    #[serde(default)]
    pub volumes: Vec<BootstrapVolumePlan>,
    #[serde(default)]
    pub failures: Vec<BootstrapArtifactFailure>,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapCreativePayload {
    pub story_blueprint: String,
    pub theme_notes: String,
    pub protagonist_seed: String,
    pub counterpart_seed: String,
    pub world_summary: String,
    pub main_plotline: String,
    #[serde(default)]
    pub volumes: Vec<BootstrapVolumePlan>,
    pub recommended_next_action: String,
}

impl BootstrapCreativePayload {
    pub fn materialize(
        &self,
        requested_kinds: &[BootstrapArtifactKind],
        generator_name: &str,
    ) -> BootstrapGenerationResult {
        let mut artifacts = Vec::new();

        for kind in requested_kinds {
            let content = match kind {
                BootstrapArtifactKind::StoryBlueprint => self.story_blueprint.clone(),
                BootstrapArtifactKind::ThemeNotes => self.theme_notes.clone(),
                BootstrapArtifactKind::ProtagonistSeed => self.protagonist_seed.clone(),
                BootstrapArtifactKind::CounterpartSeed => self.counterpart_seed.clone(),
                BootstrapArtifactKind::WorldSummary => self.world_summary.clone(),
                BootstrapArtifactKind::MainPlotline => self.main_plotline.clone(),
                BootstrapArtifactKind::VolumePlan => render_volume_plan_markdown(&self.volumes),
                BootstrapArtifactKind::ChapterBacklog => {
                    render_chapter_backlog_markdown(&self.volumes)
                }
            };

            artifacts.push(GeneratedArtifact {
                kind: *kind,
                title: kind.title().to_string(),
                content,
                status: "draft".to_string(),
                summary: None,
            });
        }

        BootstrapGenerationResult {
            generator_name: generator_name.to_string(),
            artifacts,
            volumes: self.volumes.clone(),
            failures: Vec::new(),
            recommended_next_action: self.recommended_next_action.clone(),
        }
    }
}

fn render_volume_plan_markdown(volumes: &[BootstrapVolumePlan]) -> String {
    let mut output = String::from("# Volume Plan\n");
    for (index, volume) in volumes.iter().enumerate() {
        output.push_str(&format!(
            "\n## 卷{}：{}\n- 目标字数：{}\n- 叙事任务：{}\n- 摘要：{}\n",
            index + 1,
            volume.title,
            volume.target_words,
            volume.dramatic_goal,
            volume.summary
        ));
    }
    output
}

fn render_chapter_backlog_markdown(volumes: &[BootstrapVolumePlan]) -> String {
    let mut output = String::from("# Chapter Backlog\n");
    for (volume_index, volume) in volumes.iter().enumerate() {
        output.push_str(&format!("\n## 卷{}：{}\n", volume_index + 1, volume.title));
        for (chapter_index, chapter) in volume.chapters.iter().enumerate() {
            output.push_str(&format!(
                "- 第{}章：{}｜目标字数：{}｜情节目标：{}｜情绪目标：{}\n",
                chapter_index + 1,
                chapter.title,
                chapter.target_words,
                chapter.plot_goal,
                chapter.emotional_goal
            ));
        }
    }
    output
}
