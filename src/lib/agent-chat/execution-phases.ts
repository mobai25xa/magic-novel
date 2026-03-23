import type { AgentUiTimelineEvent, AgentUiToolStep } from './types'

export type ExecutionPhase =
  | 'orchestrator'
  | 'context'
  | 'draft'
  | 'review'
  | 'knowledge'
  | 'other'

export type PhaseSegment = {
  phase: ExecutionPhase
  key: string
  startedAt?: number
  endedAt?: number
}

export type PhaseGroup = PhaseSegment & {
  callIds: string[]
}

export type ResolveExecutionPhasesInput = {
  events: AgentUiTimelineEvent[]
  toolSteps: AgentUiToolStep[]
  includeOrchestrator?: boolean
}

function asRecord(input: unknown): Record<string, unknown> | null {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return null
  }

  return input as Record<string, unknown>
}

function asText(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }

  const value = input.trim()
  return value ? value : undefined
}

function normalizeNumber(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function fallbackSeq(event: AgentUiTimelineEvent, index: number) {
  const raw = normalizeNumber(event.seq)
  if (raw === undefined) {
    return index + 1
  }
  return Math.max(1, Math.floor(raw))
}

function fallbackTs(event: AgentUiTimelineEvent, index: number) {
  return normalizeNumber(event.ts) ?? index + 1
}

export function resolveExecutionPhaseFromWorkerType(workerType: unknown): ExecutionPhase {
  const normalized = asText(workerType)?.toLowerCase()
  if (!normalized) {
    return 'other'
  }

  if (normalized === 'orchestrator') return 'orchestrator'
  if (normalized === 'context') return 'context'
  if (normalized === 'draft') return 'draft'
  if (normalized === 'review') return 'review'
  if (normalized === 'knowledge') return 'knowledge'
  if (normalized === 'other') return 'other'
  return 'other'
}

function resolveFirstPlanStarted(events: AgentUiTimelineEvent[]) {
  let best: AgentUiTimelineEvent | null = null
  let bestSeq = 0
  let bestTs = 0
  let bestIndex = 0

  events.forEach((event, index) => {
    if (event.type !== 'PLAN_STARTED') {
      return
    }

    const seq = fallbackSeq(event, index)
    const ts = fallbackTs(event, index)
    if (!best) {
      best = event
      bestSeq = seq
      bestTs = ts
      bestIndex = index
      return
    }

    if (seq !== bestSeq) {
      if (seq < bestSeq) {
        best = event
        bestSeq = seq
        bestTs = ts
        bestIndex = index
      }
      return
    }

    if (ts !== bestTs) {
      if (ts < bestTs) {
        best = event
        bestSeq = seq
        bestTs = ts
        bestIndex = index
      }
      return
    }

    if (index < bestIndex) {
      best = event
      bestSeq = seq
      bestTs = ts
      bestIndex = index
    }
  })

  if (!best) {
    return null
  }

  return {
    id: best.id,
    seq: bestSeq,
    ts: bestTs,
  }
}

export function resolveExecutionPhaseFromToolName(toolName: string): ExecutionPhase {
  if (!toolName) {
    return 'other'
  }

  const normalized = toolName.trim()
  if (normalized.startsWith('context_')) return 'context'
  if (normalized.startsWith('layer1_')) return 'context'
  if (normalized.startsWith('contextpack_')) return 'context'
  if (normalized.startsWith('draft_')) return 'draft'
  if (normalized.startsWith('review_')) return 'review'
  if (normalized.startsWith('knowledge_')) return 'knowledge'
  return 'other'
}

function sortEventsBySeq(events: AgentUiTimelineEvent[]) {
  return events
    .map((event, index) => ({
      event,
      seq: fallbackSeq(event, index),
      ts: fallbackTs(event, index),
      index,
    }))
    .sort((a, b) => {
      if (a.seq !== b.seq) {
        return a.seq - b.seq
      }

      if (a.ts !== b.ts) {
        return a.ts - b.ts
      }

      return a.index - b.index
    })
}

function resolveWorkerSegments(input: {
  events: AgentUiTimelineEvent[]
  includeOrchestrator: boolean
}): Array<{
  phase: ExecutionPhase
  key: string
  startedAt: number
  endedAt?: number
}> {
  const segments: Array<{
    phase: ExecutionPhase
    key: string
    startedAt: number
    endedAt?: number
    workerSessionId?: string
  }> = []

  const sorted = sortEventsBySeq(input.events).map((item) => item.event)
  const hasWorkerStart = sorted.some((event) => event.type === 'WORKER_STARTED')
  if (!hasWorkerStart) {
    return []
  }

  const segmentByWorkerSessionId = new Map<string, (typeof segments)[number]>()

  for (const event of sorted) {
    if (event.type !== 'WORKER_STARTED' && event.type !== 'WORKER_COMPLETED') {
      continue
    }

    const meta = asRecord(event.meta)
    const phase = resolveExecutionPhaseFromWorkerType(meta?.worker_type ?? meta?.workerType)
    if (!input.includeOrchestrator && phase === 'orchestrator') {
      continue
    }

    const workerSessionId = asText(meta?.worker_session_id)

    if (event.type === 'WORKER_STARTED') {
      const key = workerSessionId
        ? `worker_${phase}_${workerSessionId}`
        : `worker_${phase}_${event.seq}_${event.id}`

      const segment = {
        phase,
        key,
        startedAt: event.ts,
        endedAt: undefined as number | undefined,
        workerSessionId,
      }

      segments.push(segment)
      if (workerSessionId) {
        segmentByWorkerSessionId.set(workerSessionId, segment)
      }
      continue
    }

    let target: (typeof segments)[number] | undefined
    if (workerSessionId) {
      target = segmentByWorkerSessionId.get(workerSessionId)
    }

    if (!target) {
      for (let i = segments.length - 1; i >= 0; i -= 1) {
        const candidate = segments[i]
        if (candidate.phase !== phase || typeof candidate.endedAt === 'number') {
          continue
        }
        target = candidate
        break
      }
    }

    if (target) {
      target.endedAt = event.ts
    }
  }

  const merged: Array<{
    phase: ExecutionPhase
    key: string
    startedAt: number
    endedAt?: number
  }> = []

  for (const segment of segments) {
    const last = merged[merged.length - 1]
    if (last && last.phase === segment.phase) {
      if (typeof last.endedAt === 'number' && typeof segment.endedAt === 'number') {
        last.endedAt = Math.max(last.endedAt, segment.endedAt)
      } else {
        last.endedAt = undefined
      }
      continue
    }

    merged.push({
      phase: segment.phase,
      key: segment.key,
      startedAt: segment.startedAt,
      endedAt: segment.endedAt,
    })
  }

  return merged
}

function resolveExecutionPhaseGroupsFromWorkerEvents(input: ResolveExecutionPhasesInput): PhaseGroup[] {
  const segments = resolveWorkerSegments({
    events: input.events,
    includeOrchestrator: input.includeOrchestrator ?? true,
  })

  if (segments.length === 0) {
    return []
  }

  const groups: PhaseGroup[] = segments.map((segment) => ({
    phase: segment.phase,
    key: segment.key,
    startedAt: segment.startedAt,
    endedAt: segment.endedAt,
    callIds: [],
  }))

  const sortedToolSteps = [...input.toolSteps].sort((a, b) => {
    if (a.startedAt !== b.startedAt) {
      return a.startedAt - b.startedAt
    }
    return a.callId.localeCompare(b.callId)
  })

  let groupIndex = 0

  for (const step of sortedToolSteps) {
    while (groupIndex + 1 < groups.length && step.startedAt >= (groups[groupIndex + 1].startedAt ?? 0)) {
      groupIndex += 1
    }

    const group = groups[groupIndex]
    if (!group) {
      continue
    }

    if (!group.callIds.includes(step.callId)) {
      group.callIds.push(step.callId)
    }
  }

  return groups
}

export function resolveExecutionPhaseGroups(input: ResolveExecutionPhasesInput): PhaseGroup[] {
  const workerGroups = resolveExecutionPhaseGroupsFromWorkerEvents(input)
  if (workerGroups.length > 0) {
    return workerGroups
  }

  const groups: PhaseGroup[] = []
  const includeOrchestrator = input.includeOrchestrator ?? true

  const planStarted = includeOrchestrator ? resolveFirstPlanStarted(input.events) : null
  const orchestratorKey = planStarted ? `orchestrator_${planStarted.seq}_${planStarted.id}` : null

  const entries: Array<
    | { kind: 'plan'; ts: number; key: string }
    | { kind: 'tool'; ts: number; step: AgentUiToolStep }
  > = input.toolSteps.map((step) => ({ kind: 'tool', ts: step.startedAt, step }))

  if (planStarted && orchestratorKey) {
    entries.push({ kind: 'plan', ts: planStarted.ts, key: orchestratorKey })
  }

  entries.sort((a, b) => {
    if (a.ts !== b.ts) {
      return a.ts - b.ts
    }

    if (a.kind !== b.kind) {
      return a.kind === 'plan' ? -1 : 1
    }

    if (a.kind === 'tool' && b.kind === 'tool') {
      return a.step.callId.localeCompare(b.step.callId)
    }

    if (a.kind === 'plan' && b.kind === 'plan') {
      return a.key.localeCompare(b.key)
    }

    return 0
  })

  for (const entry of entries) {
    const phase = entry.kind === 'plan' ? 'orchestrator' : resolveExecutionPhaseFromToolName(entry.step.toolName)
    const callId = entry.kind === 'tool' ? entry.step.callId : null
    const startedAt = entry.ts
    const endedAt = entry.kind === 'tool' ? entry.step.finishedAt : undefined
    const groupKey = entry.kind === 'plan' ? entry.key : `${phase}_${callId}`
    const last = groups[groups.length - 1]

    if (last && last.phase === phase) {
      if (callId && !last.callIds.includes(callId)) {
        last.callIds.push(callId)
      }
      if (typeof endedAt === 'number') {
        last.endedAt = typeof last.endedAt === 'number' ? Math.max(last.endedAt, endedAt) : endedAt
      }
      continue
    }

    groups.push({
      phase,
      key: groupKey,
      callIds: callId ? [callId] : [],
      startedAt,
      endedAt,
    })
  }

  return groups
}

export function resolveExecutionPhases(input: ResolveExecutionPhasesInput): PhaseSegment[] {
  return resolveExecutionPhaseGroups(input).map((group) => ({
    phase: group.phase,
    key: group.key,
    startedAt: group.startedAt,
    endedAt: group.endedAt,
  }))
}
