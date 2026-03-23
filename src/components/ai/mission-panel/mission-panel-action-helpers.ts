import type { ActiveCastDraft } from '@/components/ai/active-cast-editor'
import type { ChapterCardDraft } from '@/components/ai/chapter-card-editor'
import type { RecentFactsDraft } from '@/components/ai/recent-facts-editor'
import type { ActiveCast, ChapterCard, RecentFacts } from '@/lib/tauri-commands'

export const LAYER1_SCHEMA_VERSION = 1
export const MANUAL_LAYER1_SOURCE_PREFIX = 'manual:mission-panel'

export function normalizeChapterPath(path: string) {
  return path.replaceAll('\\', '/').replace(/^manuscripts\//, '').trim()
}

export function normalizeScopeRefFromChapterPath(path: string) {
  const normalized = normalizeChapterPath(path)
  const trimmed = normalized.replace(/^\/+/, '')
  const withoutExt = trimmed.replace(/\.[^.]+$/, '')
  const safe = withoutExt
    .replaceAll(/[^a-zA-Z0-9:_-]+/g, ':')
    .replaceAll(/:+/g, ':')
    .replace(/^:/, '')
    .replace(/:$/, '')
  return safe ? `chapter:${safe}` : ''
}

export function resolveTokenBudget(input: {
  workflowKind?: string
  macroBudget?: string
}): 'small' | 'medium' | 'large' {
  const macroBudget = String(input.macroBudget ?? '').trim().toLowerCase()
  if (macroBudget === 'small' || macroBudget === 'medium' || macroBudget === 'large') {
    return macroBudget
  }

  const workflowKind = String(input.workflowKind ?? '').trim().toLowerCase()
  if (workflowKind === 'micro') return 'small'
  if (workflowKind === 'arc' || workflowKind === 'book') return 'large'
  return 'medium'
}

export function buildChapterCardDoc(input: {
  existing: ChapterCard | null
  scopeRef: string
  scopeLocator?: string
  draft: ChapterCardDraft
}): ChapterCard {
  const { existing, scopeRef, scopeLocator, draft } = input

  return {
    ...(existing ?? {}),
    schema_version: existing?.schema_version ?? LAYER1_SCHEMA_VERSION,
    scope_ref: scopeRef,
    ...(scopeLocator ? { scope_locator: scopeLocator } : {}),
    objective: draft.objective,
    hard_constraints: draft.hard_constraints,
    success_criteria: draft.success_criteria,
    workflow_kind: existing?.workflow_kind ?? 'chapter',
    status: existing?.status ?? 'draft',
    updated_at: Date.now(),
  }
}

export function resolveFactSourceRef(sourceRef: string | undefined, scopeRef: string) {
  const normalized = String(sourceRef ?? '').trim()
  if (normalized) {
    return normalized
  }

  return `${MANUAL_LAYER1_SOURCE_PREFIX}:${scopeRef}`
}

export function buildRecentFactsDoc(input: {
  existing: RecentFacts | null
  scopeRef: string
  draft: RecentFactsDraft
}): RecentFacts {
  const { existing, scopeRef, draft } = input

  return {
    ...(existing ?? {}),
    schema_version: existing?.schema_version ?? LAYER1_SCHEMA_VERSION,
    scope_ref: scopeRef,
    facts: draft.facts.map((fact) => ({
      summary: fact.summary,
      source_ref: resolveFactSourceRef(fact.source_ref, scopeRef),
      confidence: fact.confidence === 'accepted' ? 'accepted' : 'proposed',
    })),
    updated_at: Date.now(),
  }
}

export function buildActiveCastDoc(input: {
  existing: ActiveCast | null
  scopeRef: string
  draft: ActiveCastDraft
}): ActiveCast {
  const { existing, scopeRef, draft } = input
  const existingByCharacterRef = new Map(
    (existing?.cast ?? []).map((entry) => [entry.character_ref, entry] as const),
  )

  return {
    ...(existing ?? {}),
    schema_version: existing?.schema_version ?? LAYER1_SCHEMA_VERSION,
    scope_ref: scopeRef,
    cast: draft.cast.map((entry) => ({
      ...(existingByCharacterRef.get(entry.character_ref) ?? {}),
      character_ref: entry.character_ref,
      current_state_summary: entry.current_state_summary,
      must_keep_voice_signals: entry.must_keep_voice_signals,
    })),
    updated_at: Date.now(),
  }
}
