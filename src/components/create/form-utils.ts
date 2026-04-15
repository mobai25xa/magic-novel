import type { HomeCreateProjectInput } from '@/components/home/page/home-page-types'

import type {
  CreateProjectDraft,
  CreateProjectFormErrors,
  CreateProjectNarrativePov,
} from './types'

const DEFAULT_POV: CreateProjectNarrativePov = 'third_limited'

function splitCommaSeparated(input: string) {
  return input
    .split(/[,，\n]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function unique(values: string[]) {
  return values.filter((value, index) => values.indexOf(value) === index)
}

function parseOptionalPositiveInteger(input: string) {
  const normalized = input.trim()
  if (!normalized) {
    return undefined
  }

  const value = Number(normalized)
  if (!Number.isInteger(value) || value <= 0) {
    return null
  }

  return value
}

export function createDefaultProjectDraft(): CreateProjectDraft {
  return {
    name: '',
    author: '',
    description: '',
    coverImage: '',
    selectedGenres: [],
    customGenres: '',
    targetTotalWords: '',
    plannedVolumes: '',
    targetWordsPerChapter: '',
    narrativePov: DEFAULT_POV,
    tone: '',
    audience: '',
    protagonistSeed: '',
    counterpartSeed: '',
    worldSeed: '',
    endingDirection: '',
    aiAssist: false,
  }
}

export function validateCreateProjectDraft(draft: CreateProjectDraft): CreateProjectFormErrors {
  const errors: CreateProjectFormErrors = {}

  if (!draft.name.trim()) {
    errors.name = 'name'
  }

  if (!draft.author.trim()) {
    errors.author = 'author'
  }

  if (!draft.description.trim()) {
    errors.description = 'description'
  }

  return errors
}

export function buildCreateProjectInput(draft: CreateProjectDraft): HomeCreateProjectInput {
  const targetTotalWords = parseOptionalPositiveInteger(draft.targetTotalWords)
  if (targetTotalWords == null) {
    throw new Error('targetTotalWords must be a positive integer')
  }

  const plannedVolumes = parseOptionalPositiveInteger(draft.plannedVolumes)
  const targetWordsPerChapter = parseOptionalPositiveInteger(draft.targetWordsPerChapter)

  return {
    name: draft.name.trim(),
    author: draft.author.trim(),
    description: draft.description.trim(),
    coverImage: draft.coverImage.trim() || undefined,
    projectType: unique([
      ...draft.selectedGenres,
      ...splitCommaSeparated(draft.customGenres),
    ]),
    targetTotalWords,
    plannedVolumes: plannedVolumes ?? undefined,
    targetWordsPerChapter: targetWordsPerChapter ?? undefined,
    narrativePov: draft.narrativePov || DEFAULT_POV,
    tone: unique(splitCommaSeparated(draft.tone)),
    audience: draft.audience.trim() || undefined,
    protagonistSeed: draft.protagonistSeed.trim() || undefined,
    counterpartSeed: draft.counterpartSeed.trim() || undefined,
    worldSeed: draft.worldSeed.trim() || undefined,
    endingDirection: draft.endingDirection.trim() || undefined,
    aiAssist: draft.aiAssist,
  }
}
