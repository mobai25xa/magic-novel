import type { LightweightChatMessage } from '@/components/ai/lightweight-chat/lightweight-chat-types'
import type {
  ConsensusField,
  ConsensusFieldId,
  ConsensusValue,
  CreateProjectHandoffDraft,
  InspirationAgentMessage,
  InspirationConsensusState,
} from '@/features/inspiration/types'

import type { CreateProjectDraft } from '../types'

export const INSPIRATION_TOOL_WHITELIST = [
  'inspiration_consensus_patch',
  'inspiration_open_questions_patch',
] as const

export const CONSENSUS_FIELD_IDS: ConsensusFieldId[] = [
  'story_core',
  'premise',
  'genre_tone',
  'protagonist',
  'worldview',
  'core_conflict',
  'selling_points',
  'audience',
  'ending_direction',
]

export const REQUIRED_VARIANT_FIELD_IDS: ConsensusFieldId[] = [
  'story_core',
  'premise',
  'genre_tone',
  'protagonist',
  'core_conflict',
]

function createEmptyConsensusField(fieldId: ConsensusFieldId): ConsensusField {
  return {
    field_id: fieldId,
    draft_value: undefined,
    confirmed_value: undefined,
    locked: false,
    updated_at: 0,
    last_source_turn_id: undefined,
  }
}

export function createEmptyConsensusState(): InspirationConsensusState {
  return {
    story_core: createEmptyConsensusField('story_core'),
    premise: createEmptyConsensusField('premise'),
    genre_tone: createEmptyConsensusField('genre_tone'),
    protagonist: createEmptyConsensusField('protagonist'),
    worldview: createEmptyConsensusField('worldview'),
    core_conflict: createEmptyConsensusField('core_conflict'),
    selling_points: createEmptyConsensusField('selling_points'),
    audience: createEmptyConsensusField('audience'),
    ending_direction: createEmptyConsensusField('ending_direction'),
  }
}

export function createEmptyCreateHandoffDraft(): CreateProjectHandoffDraft {
  return {
    name: '',
    description: '',
    project_type: [],
    tone: [],
    audience: '',
    protagonist_seed: undefined,
    counterpart_seed: undefined,
    world_seed: undefined,
    ending_direction: undefined,
  }
}

export function getConsensusField(
  consensus: InspirationConsensusState,
  fieldId: ConsensusFieldId,
): ConsensusField {
  switch (fieldId) {
    case 'story_core':
      return consensus.story_core
    case 'premise':
      return consensus.premise
    case 'genre_tone':
      return consensus.genre_tone
    case 'protagonist':
      return consensus.protagonist
    case 'worldview':
      return consensus.worldview
    case 'core_conflict':
      return consensus.core_conflict
    case 'selling_points':
      return consensus.selling_points
    case 'audience':
      return consensus.audience
    case 'ending_direction':
      return consensus.ending_direction
  }
}

export function updateConsensusField(
  consensus: InspirationConsensusState,
  fieldId: ConsensusFieldId,
  updater: (field: ConsensusField) => ConsensusField,
): InspirationConsensusState {
  const nextField = updater(getConsensusField(consensus, fieldId))

  switch (fieldId) {
    case 'story_core':
      return { ...consensus, story_core: nextField }
    case 'premise':
      return { ...consensus, premise: nextField }
    case 'genre_tone':
      return { ...consensus, genre_tone: nextField }
    case 'protagonist':
      return { ...consensus, protagonist: nextField }
    case 'worldview':
      return { ...consensus, worldview: nextField }
    case 'core_conflict':
      return { ...consensus, core_conflict: nextField }
    case 'selling_points':
      return { ...consensus, selling_points: nextField }
    case 'audience':
      return { ...consensus, audience: nextField }
    case 'ending_direction':
      return { ...consensus, ending_direction: nextField }
  }
}

export function resolveConsensusValue(field: ConsensusField): ConsensusValue | undefined {
  return field.confirmed_value ?? field.draft_value
}

export function hasConsensusValue(field: ConsensusField): boolean {
  const resolved = resolveConsensusValue(field)
  if (!resolved) {
    return false
  }

  if (typeof resolved === 'string') {
    return resolved.trim().length > 0
  }

  return resolved.length > 0
}

export function formatConsensusValue(value?: ConsensusValue): string {
  if (!value) {
    return ''
  }

  if (typeof value === 'string') {
    return value.trim()
  }

  return value.join(' / ')
}

export function toConsensusItems(value?: ConsensusValue): string[] {
  if (!value) {
    return []
  }

  if (typeof value === 'string') {
    return value.trim() ? [value.trim()] : []
  }

  return value.map((item) => item.trim()).filter(Boolean)
}

export function parseDelimitedItems(input: string): string[] {
  return input
    .split(/[,，\n]/)
    .map((item) => item.trim())
    .filter(Boolean)
    .filter((item, index, values) => values.indexOf(item) === index)
}

function compactTextSections(...sections: Array<string | undefined>) {
  return sections
    .map((section) => section?.trim())
    .filter((section): section is string => Boolean(section))
    .filter((section, index, values) => values.indexOf(section) === index)
}

export function buildCreateHandoffFromConsensus(
  consensus: InspirationConsensusState,
  currentDraft?: CreateProjectHandoffDraft,
): CreateProjectHandoffDraft {
  const projectType = currentDraft?.project_type?.length
    ? currentDraft.project_type
    : toConsensusItems(resolveConsensusValue(consensus.genre_tone))
  const tone = currentDraft?.tone?.length
    ? currentDraft.tone
    : toConsensusItems(resolveConsensusValue(consensus.genre_tone))
  const description = currentDraft?.description?.trim()
    || compactTextSections(
      formatConsensusValue(resolveConsensusValue(consensus.story_core)),
      formatConsensusValue(resolveConsensusValue(consensus.premise)),
      formatConsensusValue(resolveConsensusValue(consensus.core_conflict)),
      formatConsensusValue(resolveConsensusValue(consensus.selling_points)),
    ).join('\n\n')

  return {
    name: currentDraft?.name?.trim() ?? '',
    description,
    project_type: projectType,
    tone,
    audience: currentDraft?.audience?.trim()
      || formatConsensusValue(resolveConsensusValue(consensus.audience)),
    protagonist_seed: currentDraft?.protagonist_seed?.trim()
      || formatConsensusValue(resolveConsensusValue(consensus.protagonist))
      || undefined,
    counterpart_seed: currentDraft?.counterpart_seed?.trim() || undefined,
    world_seed: currentDraft?.world_seed?.trim()
      || formatConsensusValue(resolveConsensusValue(consensus.worldview))
      || undefined,
    ending_direction: currentDraft?.ending_direction?.trim()
      || formatConsensusValue(resolveConsensusValue(consensus.ending_direction))
      || undefined,
  }
}

export function mapInspirationMessagesToChatMessages(
  messages: InspirationAgentMessage[],
): LightweightChatMessage[] {
  return messages
    .flatMap((message) => {
      if (message.role !== 'user' && message.role !== 'assistant') {
        return []
      }

      const content = message.blocks
        .filter((block) => block.type === 'text')
        .map((block) => block.text.trim())
        .filter(Boolean)
        .join('\n\n')

      if (!content) {
        return []
      }

      return [{
        id: message.id,
        role: message.role,
        content,
      }]
    })
}

export function applyCreateHandoffToDraft(
  draft: CreateProjectDraft,
  handoff: CreateProjectHandoffDraft,
  projectGenres: string[],
): CreateProjectDraft {
  const knownGenres = new Set(projectGenres.map((genre) => genre.trim()).filter(Boolean))
  const selectedGenres = handoff.project_type.filter((genre) => knownGenres.has(genre))
  const customGenres = handoff.project_type.filter((genre) => !knownGenres.has(genre))

  return {
    ...draft,
    name: handoff.name.trim() || draft.name,
    description: handoff.description.trim() || draft.description,
    selectedGenres,
    customGenres: customGenres.join(', '),
    tone: handoff.tone.join(', '),
    audience: handoff.audience.trim(),
    protagonistSeed: handoff.protagonist_seed?.trim() ?? '',
    counterpartSeed: handoff.counterpart_seed?.trim() ?? '',
    worldSeed: handoff.world_seed?.trim() ?? '',
    endingDirection: handoff.ending_direction?.trim() ?? '',
  }
}
