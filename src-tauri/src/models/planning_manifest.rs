use serde::{Deserialize, Serialize};

pub const PLANNING_BUNDLE_VERSION: i32 = 1;
pub const PLANNING_MANIFEST_REL_PATH: &str = ".magic_novel/planning/index.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MaterializationState {
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    AiDraft,
    UserRefined,
    Accepted,
}

impl ApprovalState {
    pub fn meets(self, expected: Self) -> bool {
        self >= expected
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PlanningDocId {
    StoryBrief,
    StoryBlueprint,
    NarrativeContract,
    CharacterCards,
    ForeshadowRegistry,
    ChapterPlanning,
    VolumePlan,
}

impl PlanningDocId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StoryBrief => "story_brief",
            Self::StoryBlueprint => "story_blueprint",
            Self::NarrativeContract => "narrative_contract",
            Self::CharacterCards => "character_cards",
            Self::ForeshadowRegistry => "foreshadow_registry",
            Self::ChapterPlanning => "chapter_planning",
            Self::VolumePlan => "volume_plan",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::StoryBrief => "故事简报",
            Self::StoryBlueprint => "故事蓝图",
            Self::NarrativeContract => "叙事合同",
            Self::CharacterCards => "角色卡",
            Self::ForeshadowRegistry => "伏笔登记表",
            Self::ChapterPlanning => "章节规划",
            Self::VolumePlan => "卷规划",
        }
    }

    pub fn markdown_h1(self) -> &'static str {
        match self {
            Self::StoryBrief => "# 故事简报",
            Self::StoryBlueprint => "# 故事蓝图",
            Self::NarrativeContract => "# 叙事合同",
            Self::CharacterCards => "# 角色卡",
            Self::ForeshadowRegistry => "# 伏笔登记表",
            Self::ChapterPlanning => "# 章节规划",
            Self::VolumePlan => "# 卷规划",
        }
    }

    pub fn relative_path(self) -> &'static str {
        match self {
            Self::StoryBrief => ".magic_novel/planning/story_brief.md",
            Self::StoryBlueprint => ".magic_novel/planning/story_blueprint.md",
            Self::NarrativeContract => ".magic_novel/planning/narrative_contract.md",
            Self::CharacterCards => ".magic_novel/planning/character_cards.md",
            Self::ForeshadowRegistry => ".magic_novel/planning/foreshadow_registry.md",
            Self::ChapterPlanning => ".magic_novel/planning/chapter_planning.md",
            Self::VolumePlan => ".magic_novel/planning/volume_plan.md",
        }
    }

    pub fn required_for_create(self) -> bool {
        !matches!(self, Self::VolumePlan)
    }

    pub fn required_for_write(self) -> bool {
        matches!(self, Self::NarrativeContract | Self::ChapterPlanning)
    }

    pub fn sort_index(self) -> usize {
        match self {
            Self::StoryBrief => 0,
            Self::StoryBlueprint => 1,
            Self::NarrativeContract => 2,
            Self::CharacterCards => 3,
            Self::ForeshadowRegistry => 4,
            Self::ChapterPlanning => 5,
            Self::VolumePlan => 6,
        }
    }

    pub fn core_docs() -> &'static [Self] {
        &[
            Self::StoryBrief,
            Self::StoryBlueprint,
            Self::NarrativeContract,
            Self::CharacterCards,
            Self::ForeshadowRegistry,
            Self::ChapterPlanning,
        ]
    }

    pub fn from_relative_path(path: &str) -> Option<Self> {
        match path.trim() {
            ".magic_novel/planning/story_brief.md" => Some(Self::StoryBrief),
            ".magic_novel/planning/story_blueprint.md" => Some(Self::StoryBlueprint),
            ".magic_novel/planning/narrative_contract.md" => Some(Self::NarrativeContract),
            ".magic_novel/planning/character_cards.md" => Some(Self::CharacterCards),
            ".magic_novel/planning/foreshadow_registry.md" => Some(Self::ForeshadowRegistry),
            ".magic_novel/planning/chapter_planning.md" => Some(Self::ChapterPlanning),
            ".magic_novel/planning/volume_plan.md" => Some(Self::VolumePlan),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningDocEntry {
    pub id: String,
    pub path: String,
    pub required_for_create: bool,
    pub required_for_write: bool,
    pub materialization_state: MaterializationState,
    pub approval_state: ApprovalState,
    pub last_source: String,
    pub updated_at: i64,
}

impl PlanningDocEntry {
    pub fn new(doc_id: PlanningDocId, last_source: String, updated_at: i64) -> Self {
        Self {
            id: doc_id.as_str().to_string(),
            path: doc_id.relative_path().to_string(),
            required_for_create: doc_id.required_for_create(),
            required_for_write: doc_id.required_for_write(),
            materialization_state: MaterializationState::Ready,
            approval_state: ApprovalState::AiDraft,
            last_source,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WritingReadiness {
    pub can_start: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningManifest {
    pub bundle_version: i32,
    pub bundle_status: String,
    pub docs: Vec<PlanningDocEntry>,
    pub writing_readiness: WritingReadiness,
    pub optional_outputs: Vec<String>,
    pub recommended_next_doc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_model: Option<String>,
    pub updated_at: i64,
}

impl PlanningManifest {
    pub fn new(
        docs: Vec<PlanningDocEntry>,
        optional_outputs: Vec<String>,
        generation_source: Option<String>,
        generation_provider: Option<String>,
        generation_model: Option<String>,
        updated_at: i64,
    ) -> Self {
        let mut manifest = Self {
            bundle_version: PLANNING_BUNDLE_VERSION,
            bundle_status: "ready".to_string(),
            docs,
            writing_readiness: WritingReadiness::default(),
            optional_outputs,
            recommended_next_doc: PlanningDocId::NarrativeContract.relative_path().to_string(),
            generation_source,
            generation_provider,
            generation_model,
            updated_at,
        };
        manifest.refresh_derived_fields();
        manifest
    }

    pub fn doc(&self, doc_id: PlanningDocId) -> Option<&PlanningDocEntry> {
        self.docs.iter().find(|entry| entry.id == doc_id.as_str())
    }

    pub fn refresh_derived_fields(&mut self) {
        let mut blockers = Vec::new();

        let narrative_ready = self
            .doc(PlanningDocId::NarrativeContract)
            .map(|entry| entry.approval_state.meets(ApprovalState::UserRefined))
            .unwrap_or(false);
        if !narrative_ready {
            blockers.push("narrative_contract_unconfirmed".to_string());
        }

        let chapter_ready = self
            .doc(PlanningDocId::ChapterPlanning)
            .map(|entry| entry.approval_state.meets(ApprovalState::UserRefined))
            .unwrap_or(false);
        if !chapter_ready {
            blockers.push("chapter_1_detail_unconfirmed".to_string());
        }

        self.writing_readiness = WritingReadiness {
            can_start: blockers.is_empty(),
            blockers,
        };

        self.recommended_next_doc = if !narrative_ready {
            PlanningDocId::NarrativeContract.relative_path().to_string()
        } else if !chapter_ready {
            PlanningDocId::ChapterPlanning.relative_path().to_string()
        } else {
            PlanningDocId::StoryBrief.relative_path().to_string()
        };

        self.bundle_status = if self.docs.iter().all(|entry| {
            !entry.required_for_create
                || matches!(entry.materialization_state, MaterializationState::Ready)
        }) {
            "ready".to_string()
        } else {
            "failed".to_string()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_defaults_to_blocked_until_contracts_are_refined() {
        let now = 1_000;
        let docs = PlanningDocId::core_docs()
            .iter()
            .copied()
            .map(|doc_id| PlanningDocEntry::new(doc_id, "deterministic_fallback".to_string(), now))
            .collect();

        let manifest = PlanningManifest::new(
            docs,
            Vec::new(),
            Some("deterministic_fallback".to_string()),
            None,
            None,
            now,
        );

        assert!(!manifest.writing_readiness.can_start);
        assert_eq!(
            manifest.writing_readiness.blockers,
            vec![
                "narrative_contract_unconfirmed".to_string(),
                "chapter_1_detail_unconfirmed".to_string()
            ]
        );
        assert_eq!(
            manifest.recommended_next_doc,
            PlanningDocId::NarrativeContract.relative_path()
        );
    }

    #[test]
    fn planning_doc_display_metadata_stays_stable() {
        assert_eq!(PlanningDocId::NarrativeContract.display_name(), "叙事合同");
        assert_eq!(PlanningDocId::NarrativeContract.markdown_h1(), "# 叙事合同");
        assert_eq!(
            PlanningDocId::from_relative_path(".magic_novel/planning/narrative_contract.md"),
            Some(PlanningDocId::NarrativeContract)
        );
        assert!(PlanningDocId::NarrativeContract.sort_index() < PlanningDocId::ChapterPlanning.sort_index());
    }
}
