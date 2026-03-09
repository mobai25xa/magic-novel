import {
  cancelSearchIndex,
  getConsecutiveDays,
  getMonthStats,
  getSearchIndexStatus,
  getWritingStats,
  getYearStats,
  rebuildSearchIndex,
  type SearchIndexStatus,
} from '@/lib/tauri-commands'
import { toolGateway } from '@/lib/tool-gateway/gateway'
import { createCallId } from '@/lib/tool-gateway/utils'
import { useSettingsStore } from '@/stores/settings-store'

export type SearchMode = 'keyword' | 'semantic' | 'hybrid'

export type GlobalSearchResult = {
  path: string
  chapterId?: string
  chapterTitle?: string
  snippet: string
  score: number
  metadata?: Record<string, unknown>
}

export type SearchGlobalFeatureOutput = {
  hits: GlobalSearchResult[]
  mode: SearchMode
  degraded: boolean
  degradedReason?: string
}

export type SearchIndexFeatureStatus = SearchIndexStatus

function normalizeScopePath(path: string) {
  const normalized = String(path || '').trim().replace(/\\/g, '/').replace(/^\.\//, '').replace(/^\//, '')
  if (!normalized || normalized === '.') return ''
  if (normalized.startsWith('magic_assets/')) {
    return `.magic_novel/${normalized.slice('magic_assets/'.length)}`
  }
  if (normalized === 'magic_assets') {
    return '.magic_novel'
  }
  return normalized.replace(/\/+$/, '')
}

function normalizeScopePaths(scopePaths?: string[]) {
  if (!scopePaths?.length) return undefined
  const next = scopePaths
    .map(normalizeScopePath)
    .filter((item, index, arr) => Boolean(item) && arr.indexOf(item) === index)
  return next.length ? { paths: next } : undefined
}

function inferSearchMeta(input: {
  mode: SearchMode
  hits: GlobalSearchResult[]
}) {
  const degradedHit = input.hits.find((hit) => Boolean(hit.metadata?.degraded))
  const degraded = Boolean(degradedHit)
  const reason = degradedHit && typeof hitReason(degradedHit) === 'string'
    ? hitReason(degradedHit)
    : undefined

  const effectiveMode = degraded && input.mode !== 'keyword' ? 'keyword' : input.mode

  return {
    mode: effectiveMode,
    degraded,
    degradedReason: reason,
  }
}

function hitReason(hit: GlobalSearchResult) {
  const value = hit.metadata?.degraded_reason
  return typeof value === 'string' ? value : undefined
}

export async function searchGlobalFeature(input: {
  projectPath: string
  query: string
  mode?: SearchMode
  topK?: number
  scopePaths?: string[]
}): Promise<SearchGlobalFeatureOutput> {
  const requestedMode = input.mode ?? 'keyword'
  const semanticModesEnabled = Boolean(useSettingsStore.getState().openaiEmbeddingEnabled)
  const mode: SearchMode = !semanticModesEnabled && requestedMode !== 'keyword'
    ? 'keyword'
    : requestedMode

  const result = await toolGateway.grep({
    project_path: input.projectPath,
    query: input.query,
    mode,
    top_k: input.topK ?? 20,
    scope: normalizeScopePaths(input.scopePaths),
    call_id: createCallId('grep'),
  })

  if (!result.ok || !result.data) {
    throw new Error(result.error?.message || 'Search failed')
  }

  const forcedKeywordDegrade = !semanticModesEnabled && requestedMode !== 'keyword'

  const hits = result.data.hits.map((hit) => {
    const chapterId = typeof hit.metadata?.chapter_id === 'string' ? hit.metadata.chapter_id : undefined
    const chapterTitle = typeof hit.metadata?.title === 'string' ? hit.metadata.title : undefined

    const metadata = forcedKeywordDegrade
      ? {
          ...(hit.metadata || {}),
          degraded: true,
          degraded_reason: 'embedding_disabled',
          semantic_unavailable: true,
        }
      : hit.metadata

    return {
      path: hit.path,
      chapterId,
      chapterTitle,
      snippet: hit.snippet,
      score: hit.score,
      metadata,
    }
  })

  const meta = inferSearchMeta({ mode: requestedMode, hits })

  return {
    hits,
    mode: forcedKeywordDegrade ? 'keyword' : meta.mode,
    degraded: forcedKeywordDegrade || meta.degraded,
    degradedReason: forcedKeywordDegrade ? 'embedding_disabled' : meta.degradedReason,
  }
}

export async function getSearchIndexStatusFeature(projectPath: string): Promise<SearchIndexFeatureStatus | null> {
  try {
    return await getSearchIndexStatus(projectPath)
  } catch {
    return null
  }
}

export async function rebuildSearchIndexFeature(projectPath: string, force = false): Promise<SearchIndexFeatureStatus | null> {
  try {
    return await rebuildSearchIndex({ projectPath, force })
  } catch {
    return null
  }
}

export async function cancelSearchIndexFeature(projectPath: string): Promise<SearchIndexFeatureStatus | null> {
  try {
    return await cancelSearchIndex(projectPath)
  } catch {
    return null
  }
}

export async function loadDiscoverWeekStats(days: number, rootDir?: string) {
  return getWritingStats(days, rootDir)
}

export async function loadDiscoverMonthStats(year: number, month: number, rootDir?: string) {
  return getMonthStats(year, month, rootDir)
}

export async function loadDiscoverYearStats(year: number, rootDir?: string) {
  return getYearStats(year, rootDir)
}

export async function loadDiscoverConsecutiveDays(rootDir?: string) {
  return getConsecutiveDays(rootDir)
}
