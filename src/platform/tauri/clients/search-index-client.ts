import { invokeTauri } from './core'

export type SearchIndexState = 'missing' | 'building' | 'ready' | 'failed' | 'stale' | 'unknown'

export interface SearchIndexProgress {
  stage?: string
  done?: number
  total?: number
}

export interface SearchIndexStatus {
  state: SearchIndexState
  progress?: SearchIndexProgress
  last_error?: string
  updated_at?: number
}

export interface RebuildSearchIndexInput {
  projectPath: string
  force?: boolean
}

function asRecord(input: unknown): Record<string, unknown> {
  if (input && typeof input === 'object' && !Array.isArray(input)) {
    return input as Record<string, unknown>
  }
  return {}
}

function asNumber(input: unknown): number | undefined {
  return typeof input === 'number' && Number.isFinite(input) ? input : undefined
}

function asString(input: unknown): string | undefined {
  return typeof input === 'string' && input.trim() ? input : undefined
}

function asState(input: unknown): SearchIndexState {
  const value = String(input || '').trim().toLowerCase()
  if (value === 'missing' || value === 'building' || value === 'ready' || value === 'failed' || value === 'stale') {
    return value
  }
  return 'unknown'
}

function normalizeStatus(input: unknown): SearchIndexStatus {
  const data = asRecord(input)
  const progress = asRecord(data.progress)
  const lastError = asString(data.last_error) || asString(data.lastError)

  return {
    state: asState(data.state || data.status),
    progress: {
      stage: asString(progress.stage),
      done: asNumber(progress.done),
      total: asNumber(progress.total),
    },
    last_error: lastError,
    updated_at: asNumber(data.updated_at || data.updatedAt),
  }
}

function unwrapStatusPayload(input: unknown): unknown {
  const data = asRecord(input)

  const resultRecord = asRecord(data.result)
  if (Object.keys(resultRecord).length > 0) {
    return resultRecord
  }

  const statusRecord = asRecord(data.status)
  if (Object.keys(statusRecord).length > 0) {
    return statusRecord
  }

  return input
}

async function invokeSearchIndexCommand<T>(
  command: string,
  payloads: Array<Record<string, unknown>>,
): Promise<T> {
  let lastError: unknown

  for (const payload of payloads) {
    try {
      return await invokeTauri<T>(command, payload)
    } catch (error) {
      lastError = error
    }
  }

  throw lastError instanceof Error ? lastError : new Error(String(lastError || `${command} failed`))
}

function buildProjectPayloads(projectPath: string) {
  return [
    { projectPath },
    { project_path: projectPath },
    { input: { projectPath } },
    { input: { project_path: projectPath } },
  ]
}

export async function getSearchIndexStatusClient(projectPath: string): Promise<SearchIndexStatus> {
  const raw = await invokeSearchIndexCommand<unknown>('search_index_status', buildProjectPayloads(projectPath))
  return normalizeStatus(unwrapStatusPayload(raw))
}

export async function rebuildSearchIndexClient(input: RebuildSearchIndexInput): Promise<SearchIndexStatus> {
  const force = Boolean(input.force)
  const payloads = [
    { projectPath: input.projectPath, force },
    { project_path: input.projectPath, force },
    { input: { projectPath: input.projectPath, force } },
    { input: { project_path: input.projectPath, force } },
  ]

  const raw = await invokeSearchIndexCommand<unknown>('search_index_rebuild', payloads)
  return getSearchIndexStatusClient(input.projectPath).catch(() => normalizeStatus(unwrapStatusPayload(raw)))
}

export async function cancelSearchIndexClient(projectPath: string): Promise<SearchIndexStatus> {
  const raw = await invokeSearchIndexCommand<unknown>('search_index_cancel', buildProjectPayloads(projectPath))
  return getSearchIndexStatusClient(projectPath).catch(() => normalizeStatus(unwrapStatusPayload(raw)))
}
