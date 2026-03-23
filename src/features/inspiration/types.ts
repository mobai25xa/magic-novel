export type InspirationMessageRole = 'system' | 'user' | 'assistant' | 'tool'

export interface InspirationTextContentBlock {
  type: 'text'
  text: string
}

export interface InspirationThinkingContentBlock {
  type: 'thinking'
  text: string
}

export interface InspirationToolCallContentBlock {
  type: 'tool_call'
  id: string
  name: string
  input: unknown
}

export interface InspirationToolResultContentBlock {
  type: 'tool_result'
  tool_call_id: string
  tool_name?: string
  content: string
  is_error: boolean
}

export type InspirationMessageContentBlock =
  | InspirationTextContentBlock
  | InspirationThinkingContentBlock
  | InspirationToolCallContentBlock
  | InspirationToolResultContentBlock

export interface InspirationAgentMessage {
  id: string
  role: InspirationMessageRole
  blocks: InspirationMessageContentBlock[]
  ts: number
}

export type ConsensusFieldId =
  | 'story_core'
  | 'premise'
  | 'genre_tone'
  | 'protagonist'
  | 'worldview'
  | 'core_conflict'
  | 'selling_points'
  | 'audience'
  | 'ending_direction'

export type ConsensusValue = string | string[]

export interface ConsensusField {
  field_id: ConsensusFieldId
  draft_value?: ConsensusValue
  confirmed_value?: ConsensusValue
  locked: boolean
  updated_at: number
  last_source_turn_id?: number
}

export interface InspirationConsensusState {
  story_core: ConsensusField
  premise: ConsensusField
  genre_tone: ConsensusField
  protagonist: ConsensusField
  worldview: ConsensusField
  core_conflict: ConsensusField
  selling_points: ConsensusField
  audience: ConsensusField
  ending_direction: ConsensusField
}

export type OpenQuestionImportance = 'high' | 'medium' | 'low'
export type OpenQuestionStatus = 'open' | 'resolved' | 'dismissed'

export interface OpenQuestion {
  question_id: string
  question: string
  importance: OpenQuestionImportance
  status: OpenQuestionStatus
}

export type MetadataVariantId = 'balanced' | 'hook' | 'setting'

export interface MetadataVariant {
  variant_id: MetadataVariantId
  label: string
  title: string
  one_liner: string
  short_synopsis: string
  long_synopsis: string
  setting_summary: string
  protagonist_summary: string
  tags: string[]
  tone: string[]
  audience: string
  protagonist_seed: string
  counterpart_seed: string
  world_seed: string
  ending_direction: string
}

export interface GenerateMetadataVariantsOutput {
  shared_story_core: string
  variants: MetadataVariant[]
}

export interface CreateProjectHandoffDraft {
  name: string
  description: string
  project_type: string[]
  tone: string[]
  audience: string
  protagonist_seed?: string
  counterpart_seed?: string
  world_seed?: string
  ending_direction?: string
}
