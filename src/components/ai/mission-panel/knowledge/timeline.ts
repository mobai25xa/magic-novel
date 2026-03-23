import { readTextFile } from '@tauri-apps/plugin-fs'

import { asNumber, asRecord, asString, asStringArray, isMissingFileError, normalizeFsPath } from '../utils'

export type KnowledgeTimelineEntry = {
  key: string
  ts: number
  label: string
  detail?: string
}

function unwrapMaybeWrapped(value: unknown, key: string): unknown {
  const record = asRecord(value)
  if (!record) {
    return value
  }

  const wrapped = record[key]
  return wrapped === undefined ? value : wrapped
}

function parseJsonl(content: string, maxLines = 400): unknown[] {
  const lines = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(-maxLines)

  const parsed: unknown[] = []
  for (const line of lines) {
    try {
      parsed.push(JSON.parse(line))
    } catch {
      // ignore malformed JSONL rows
    }
  }

  return parsed
}

function normalizeKnowledgeBundleCandidate(raw: unknown) {
  let value = raw
  value = unwrapMaybeWrapped(value, 'bundle')
  value = unwrapMaybeWrapped(value, 'proposal_bundle')
  value = unwrapMaybeWrapped(value, 'knowledge_bundle')
  value = unwrapMaybeWrapped(value, 'latest')
  return asRecord(value)
}

function normalizeKnowledgeDeltaCandidate(raw: unknown) {
  let value = raw
  value = unwrapMaybeWrapped(value, 'delta')
  value = unwrapMaybeWrapped(value, 'knowledge_delta')
  value = unwrapMaybeWrapped(value, 'latest')
  return asRecord(value)
}

function toKnowledgeTimelineEntryFromBundle(raw: unknown): KnowledgeTimelineEntry | null {
  const record = normalizeKnowledgeBundleCandidate(raw)
  if (!record) {
    return null
  }

  const bundleId = asString(record.bundle_id)
  const generatedAt = asNumber(record.generated_at)

  if (!bundleId || generatedAt === undefined) {
    return null
  }

  const scopeRef = asString(record.scope_ref)
  const proposals = Array.isArray(record.proposal_items) ? record.proposal_items.length : 0

  return {
    key: `bundle:${bundleId}:${generatedAt}`,
    ts: generatedAt,
    label: 'proposed',
    detail: `${proposals} items${scopeRef ? ` · ${scopeRef}` : ''}`,
  }
}

function toKnowledgeTimelineEntryFromDelta(raw: unknown): KnowledgeTimelineEntry | null {
  const record = normalizeKnowledgeDeltaCandidate(raw)
  if (!record) {
    return null
  }

  const deltaId = asString(record.knowledge_delta_id)
  const generatedAt = asNumber(record.generated_at)
  const appliedAt = asNumber(record.applied_at)

  const ts = appliedAt ?? generatedAt
  if (!deltaId || ts === undefined) {
    return null
  }

  const status = asString(record.status) ?? 'proposed'
  const conflicts = Array.isArray(record.conflicts) ? record.conflicts.length : 0
  const accepted = asStringArray(record.accepted_item_ids).length
  const rejected = asStringArray(record.rejected_item_ids).length
  const scopeRef = asString(record.scope_ref)

  const label = conflicts > 0
    ? 'blocked'
    : status === 'applied' || appliedAt !== undefined
      ? 'applied'
      : status === 'accepted'
        ? 'accepted'
        : status === 'rejected'
          ? 'rejected'
          : status

  const parts = [
    scopeRef,
    accepted > 0 ? `accepted ${accepted}` : null,
    rejected > 0 ? `rejected ${rejected}` : null,
    conflicts > 0 ? `conflicts ${conflicts}` : null,
  ].filter((item): item is string => Boolean(item))

  return {
    key: `delta:${deltaId}:${ts}`,
    ts,
    label,
    detail: parts.join(' · ') || undefined,
  }
}

export async function loadKnowledgeTimelineFromArtifacts(input: {
  projectPath: string
  missionId: string
}): Promise<KnowledgeTimelineEntry[]> {
  const projectPath = normalizeFsPath(input.projectPath)
  const base = `${projectPath}/magic_novel/missions/${input.missionId}/knowledge`
  const bundlePath = `${base}/bundles/bundles.jsonl`
  const deltaPath = `${base}/deltas/deltas.jsonl`

  const readOrEmpty = async (path: string) => {
    try {
      return await readTextFile(path)
    } catch (error) {
      if (isMissingFileError(error)) {
        return ''
      }
      throw error
    }
  }

  const [bundlesText, deltasText] = await Promise.all([
    readOrEmpty(bundlePath),
    readOrEmpty(deltaPath),
  ])

  const bundleRows = bundlesText ? parseJsonl(bundlesText) : []
  const deltaRows = deltasText ? parseJsonl(deltasText) : []

  const entries = [
    ...bundleRows
      .map((row) => toKnowledgeTimelineEntryFromBundle(row))
      .filter((item): item is KnowledgeTimelineEntry => Boolean(item)),
    ...deltaRows
      .map((row) => toKnowledgeTimelineEntryFromDelta(row))
      .filter((item): item is KnowledgeTimelineEntry => Boolean(item)),
  ]

  return entries
    .sort((a, b) => b.ts - a.ts)
    .slice(0, 60)
}

