use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusFieldId {
    StoryCore,
    Premise,
    GenreTone,
    Protagonist,
    Worldview,
    CoreConflict,
    SellingPoints,
    Audience,
    EndingDirection,
}

impl ConsensusFieldId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StoryCore => "story_core",
            Self::Premise => "premise",
            Self::GenreTone => "genre_tone",
            Self::Protagonist => "protagonist",
            Self::Worldview => "worldview",
            Self::CoreConflict => "core_conflict",
            Self::SellingPoints => "selling_points",
            Self::Audience => "audience",
            Self::EndingDirection => "ending_direction",
        }
    }

    pub fn expects_list(self) -> bool {
        matches!(self, Self::GenreTone | Self::SellingPoints)
    }
}

impl Default for ConsensusFieldId {
    fn default() -> Self {
        Self::StoryCore
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ConsensusValue {
    Text(String),
    List(Vec<String>),
}

impl ConsensusValue {
    pub fn from_text(value: Option<String>) -> Option<Self> {
        let trimmed = value?.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self::Text(trimmed))
        }
    }

    pub fn from_list(values: Vec<String>) -> Option<Self> {
        let items = normalize_list(values);
        if items.is_empty() {
            None
        } else {
            Some(Self::List(items))
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            Self::List(_) => None,
        }
    }

    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::Text(_) => None,
            Self::List(items) => Some(items.as_slice()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsensusField {
    pub field_id: ConsensusFieldId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_value: Option<ConsensusValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_value: Option<ConsensusValue>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_source_turn_id: Option<u32>,
}

impl ConsensusField {
    pub fn empty(field_id: ConsensusFieldId) -> Self {
        Self {
            field_id,
            draft_value: None,
            confirmed_value: None,
            locked: false,
            updated_at: 0,
            last_source_turn_id: None,
        }
    }

    pub fn resolved_value(&self) -> Option<&ConsensusValue> {
        self.confirmed_value.as_ref().or(self.draft_value.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspirationConsensusState {
    pub story_core: ConsensusField,
    pub premise: ConsensusField,
    pub genre_tone: ConsensusField,
    pub protagonist: ConsensusField,
    pub worldview: ConsensusField,
    pub core_conflict: ConsensusField,
    pub selling_points: ConsensusField,
    pub audience: ConsensusField,
    pub ending_direction: ConsensusField,
}

impl Default for InspirationConsensusState {
    fn default() -> Self {
        Self {
            story_core: ConsensusField::empty(ConsensusFieldId::StoryCore),
            premise: ConsensusField::empty(ConsensusFieldId::Premise),
            genre_tone: ConsensusField::empty(ConsensusFieldId::GenreTone),
            protagonist: ConsensusField::empty(ConsensusFieldId::Protagonist),
            worldview: ConsensusField::empty(ConsensusFieldId::Worldview),
            core_conflict: ConsensusField::empty(ConsensusFieldId::CoreConflict),
            selling_points: ConsensusField::empty(ConsensusFieldId::SellingPoints),
            audience: ConsensusField::empty(ConsensusFieldId::Audience),
            ending_direction: ConsensusField::empty(ConsensusFieldId::EndingDirection),
        }
    }
}

impl InspirationConsensusState {
    pub fn field(&self, field_id: ConsensusFieldId) -> &ConsensusField {
        match field_id {
            ConsensusFieldId::StoryCore => &self.story_core,
            ConsensusFieldId::Premise => &self.premise,
            ConsensusFieldId::GenreTone => &self.genre_tone,
            ConsensusFieldId::Protagonist => &self.protagonist,
            ConsensusFieldId::Worldview => &self.worldview,
            ConsensusFieldId::CoreConflict => &self.core_conflict,
            ConsensusFieldId::SellingPoints => &self.selling_points,
            ConsensusFieldId::Audience => &self.audience,
            ConsensusFieldId::EndingDirection => &self.ending_direction,
        }
    }

    pub fn field_mut(&mut self, field_id: ConsensusFieldId) -> &mut ConsensusField {
        match field_id {
            ConsensusFieldId::StoryCore => &mut self.story_core,
            ConsensusFieldId::Premise => &mut self.premise,
            ConsensusFieldId::GenreTone => &mut self.genre_tone,
            ConsensusFieldId::Protagonist => &mut self.protagonist,
            ConsensusFieldId::Worldview => &mut self.worldview,
            ConsensusFieldId::CoreConflict => &mut self.core_conflict,
            ConsensusFieldId::SellingPoints => &mut self.selling_points,
            ConsensusFieldId::Audience => &mut self.audience,
            ConsensusFieldId::EndingDirection => &mut self.ending_direction,
        }
    }

    pub fn resolved_text(&self, field_id: ConsensusFieldId) -> Option<String> {
        self.field(field_id)
            .resolved_value()
            .and_then(ConsensusValue::as_text)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    pub fn resolved_list(&self, field_id: ConsensusFieldId) -> Vec<String> {
        self.field(field_id)
            .resolved_value()
            .and_then(ConsensusValue::as_list)
            .map(|items| normalize_list(items.to_vec()))
            .unwrap_or_default()
    }

    pub fn resolve_for_variants(&self) -> Result<ResolvedConsensusSnapshot, Vec<ConsensusFieldId>> {
        let mut missing = Vec::new();

        let story_core = self.required_text(ConsensusFieldId::StoryCore, &mut missing);
        let premise = self.required_text(ConsensusFieldId::Premise, &mut missing);
        let genre_tone = self.required_list(ConsensusFieldId::GenreTone, &mut missing);
        let protagonist = self.required_text(ConsensusFieldId::Protagonist, &mut missing);
        let core_conflict = self.required_text(ConsensusFieldId::CoreConflict, &mut missing);

        if !missing.is_empty() {
            return Err(missing);
        }

        Ok(ResolvedConsensusSnapshot {
            story_core: story_core.expect("story_core checked"),
            premise: premise.expect("premise checked"),
            genre_tone: genre_tone.expect("genre_tone checked"),
            protagonist: protagonist.expect("protagonist checked"),
            worldview: self.resolved_text(ConsensusFieldId::Worldview),
            core_conflict: core_conflict.expect("core_conflict checked"),
            selling_points: self.resolved_list(ConsensusFieldId::SellingPoints),
            audience: self.resolved_text(ConsensusFieldId::Audience),
            ending_direction: self.resolved_text(ConsensusFieldId::EndingDirection),
        })
    }

    fn required_text(
        &self,
        field_id: ConsensusFieldId,
        missing: &mut Vec<ConsensusFieldId>,
    ) -> Option<String> {
        let value = self.resolved_text(field_id);
        if value.is_none() {
            missing.push(field_id);
        }
        value
    }

    fn required_list(
        &self,
        field_id: ConsensusFieldId,
        missing: &mut Vec<ConsensusFieldId>,
    ) -> Option<Vec<String>> {
        let value = self.resolved_list(field_id);
        if value.is_empty() {
            missing.push(field_id);
            None
        } else {
            Some(value)
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpenQuestionImportance {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpenQuestionStatus {
    Open,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenQuestion {
    pub question_id: String,
    pub question: String,
    pub importance: OpenQuestionImportance,
    pub status: OpenQuestionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusPatchOperation {
    SetText,
    SetItems,
    AppendItems,
    ClearDraft,
}

impl Default for ConsensusPatchOperation {
    fn default() -> Self {
        Self::SetText
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ApplyConsensusPatchInput {
    pub state: InspirationConsensusState,
    pub field_id: ConsensusFieldId,
    pub operation: ConsensusPatchOperation,
    pub text_value: Option<String>,
    pub items: Vec<String>,
    pub source_turn_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyConsensusPatchOutput {
    pub field_id: ConsensusFieldId,
    pub operation: ConsensusPatchOperation,
    pub updated_field: ConsensusField,
    pub state: InspirationConsensusState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpenQuestionsPatchOperation {
    Add,
    Resolve,
    Dismiss,
}

impl Default for OpenQuestionsPatchOperation {
    fn default() -> Self {
        Self::Add
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ApplyOpenQuestionsPatchInput {
    pub questions: Vec<OpenQuestion>,
    pub operation: OpenQuestionsPatchOperation,
    pub question_id: Option<String>,
    pub question: Option<String>,
    pub importance: Option<OpenQuestionImportance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyOpenQuestionsPatchOutput {
    pub operation: OpenQuestionsPatchOperation,
    pub updated_question: OpenQuestion,
    pub questions: Vec<OpenQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedConsensusSnapshot {
    pub story_core: String,
    pub premise: String,
    pub genre_tone: Vec<String>,
    pub protagonist: String,
    pub worldview: Option<String>,
    pub core_conflict: String,
    pub selling_points: Vec<String>,
    pub audience: Option<String>,
    pub ending_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct GenerateMetadataVariantsInput {
    pub consensus: InspirationConsensusState,
}

impl Default for GenerateMetadataVariantsInput {
    fn default() -> Self {
        Self {
            consensus: InspirationConsensusState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetadataVariantId {
    Balanced,
    Hook,
    Setting,
}

impl MetadataVariantId {
    pub fn ordered() -> &'static [Self] {
        &[Self::Balanced, Self::Hook, Self::Setting]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Balanced => "平衡版",
            Self::Hook => "强钩子版",
            Self::Setting => "设定卖点版",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetadataVariant {
    pub variant_id: MetadataVariantId,
    pub label: String,
    pub title: String,
    pub one_liner: String,
    pub short_synopsis: String,
    pub long_synopsis: String,
    pub setting_summary: String,
    pub protagonist_summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tone: Vec<String>,
    pub audience: String,
    pub protagonist_seed: String,
    pub counterpart_seed: String,
    pub world_seed: String,
    pub ending_direction: String,
}

impl MetadataVariant {
    pub fn normalize(mut self) -> Self {
        self.label = non_empty(self.label, self.variant_id.label());
        self.title = self.title.trim().to_string();
        self.one_liner = self.one_liner.trim().to_string();
        self.short_synopsis = self.short_synopsis.trim().to_string();
        self.long_synopsis = self.long_synopsis.trim().to_string();
        self.setting_summary = self.setting_summary.trim().to_string();
        self.protagonist_summary = self.protagonist_summary.trim().to_string();
        self.tags = normalize_list(self.tags);
        self.tone = normalize_list(self.tone);
        self.audience = self.audience.trim().to_string();
        self.protagonist_seed = self.protagonist_seed.trim().to_string();
        self.counterpart_seed = self.counterpart_seed.trim().to_string();
        self.world_seed = self.world_seed.trim().to_string();
        self.ending_direction = self.ending_direction.trim().to_string();
        self
    }

    pub fn to_create_handoff(&self) -> CreateProjectHandoffDraft {
        CreateProjectHandoffDraft {
            name: self.title.clone(),
            description: self.long_synopsis.clone(),
            project_type: normalize_list(self.tags.clone()),
            tone: normalize_list(self.tone.clone()),
            audience: self.audience.clone(),
            protagonist_seed: non_empty_option(self.protagonist_seed.clone()),
            counterpart_seed: non_empty_option(self.counterpart_seed.clone()),
            world_seed: non_empty_option(self.world_seed.clone()),
            ending_direction: non_empty_option(self.ending_direction.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerateMetadataVariantsOutput {
    pub shared_story_core: String,
    pub variants: Vec<MetadataVariant>,
}

impl GenerateMetadataVariantsOutput {
    pub fn normalize(mut self) -> Self {
        self.shared_story_core = self.shared_story_core.trim().to_string();
        self.variants = self
            .variants
            .into_iter()
            .map(MetadataVariant::normalize)
            .collect();
        self.variants
            .sort_by_key(|variant| match variant.variant_id {
                MetadataVariantId::Balanced => 0,
                MetadataVariantId::Hook => 1,
                MetadataVariantId::Setting => 2,
            });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateProjectHandoffDraft {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub project_type: Vec<String>,
    #[serde(default)]
    pub tone: Vec<String>,
    pub audience: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protagonist_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterpart_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_direction: Option<String>,
}

pub fn normalize_list(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || out.iter().any(|existing| existing == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

pub fn non_empty_option(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn non_empty(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}
