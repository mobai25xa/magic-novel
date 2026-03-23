import {
  getMissionRecoveryEntries,
  getMissionResultEntries,
  getMissionResultEntriesFromJobSnapshot,
  type MissionRecoveryEntry,
  type MissionGetStatusOutput,
  type MissionResultEntry,
} from '@/lib/tauri-commands/mission'
import type { JobBlocker, JobSnapshot } from '@/types/agent-job'
import type { Layer1SnapshotPayload } from './types'

export type JobPhase = 'ready' | 'running' | 'waiting' | 'blocked' | 'completed' | 'failed'

export type JobBlockerView = {
  blockerId: string
  kind: JobBlocker['kind']
  kindLabel: string
  summary: string
  updatedAt: number
}

export type RecoveryTone = 'info' | 'warning' | 'success' | 'error'

export type RecoveryEntryView = {
  key: string
  ts: number
  message: string
  tone: RecoveryTone
}

export type JobStatusView = {
  hasSnapshot: boolean
  phase: JobPhase
  phaseLabel: string
  statusLabel: string
  headline: string
  detail: string | null
  updatedAt: number
  blockerCount: number
  readyTaskCount: number
  runningTaskCount: number
  completedTaskCount: number
  failedTaskCount: number
  blockers: JobBlockerView[]
  recoveryLabel: string | null
  recoveryHint: string | null
  recoveryTone: RecoveryTone | null
  recoveryEntries: RecoveryEntryView[]
  latestRecoveryEntry: RecoveryEntryView | null
  canResume: boolean
  resumeActionLabel: string | null
  shouldOpenRecovery: boolean
}

function humanizeStatus(value: string) {
  return value.replaceAll('_', ' ').trim() || 'unknown'
}

function pluralize(count: number, singular: string, plural = `${singular}s`) {
  return `${count} ${count === 1 ? singular : plural}`
}

function joinSegments(values: string[]) {
  return values.filter(Boolean).join(' · ')
}

function blockerKindLabel(kind: JobBlocker['kind']) {
  switch (kind) {
    case 'review_gate':
      return 'Review gate'
    case 'knowledge_decision':
      return 'Knowledge decision'
    case 'user_clarification':
      return 'User clarification'
    case 'external_dependency':
    default:
      return 'External dependency'
  }
}

function blockerFallbackSummary(kind: JobBlocker['kind']) {
  switch (kind) {
    case 'review_gate':
      return 'Waiting for a review decision before work can continue.'
    case 'knowledge_decision':
      return 'Waiting for a knowledge decision before work can continue.'
    case 'user_clarification':
      return 'Waiting for user clarification before work can continue.'
    case 'external_dependency':
    default:
      return 'Blocked by an external dependency.'
  }
}

function toJobBlockerView(blocker: JobBlocker): JobBlockerView {
  return {
    blockerId: blocker.blocker_id,
    kind: blocker.kind,
    kindLabel: blockerKindLabel(blocker.kind),
    summary: blocker.summary.trim() || blockerFallbackSummary(blocker.kind),
    updatedAt: blocker.updated_at,
  }
}

function classifyRecoveryTone(message: string): RecoveryTone {
  const normalized = message.toLowerCase()

  if (
    normalized.includes('failed')
    || normalized.includes('rejected')
    || normalized.includes('error')
    || normalized.includes('cannot ')
    || normalized.includes('missing')
    || normalized.includes('not found')
  ) {
    return 'error'
  }

  if (
    normalized.includes('succeeded')
    || normalized.includes('recovered')
    || normalized.includes(' resumed')
    || normalized.startsWith('mission resumed')
  ) {
    return 'success'
  }

  if (
    normalized.includes('paused')
    || normalized.includes('blocked')
    || normalized.includes('waiting')
    || normalized.includes('interrupted')
  ) {
    return 'warning'
  }

  return 'info'
}

function toRecoveryEntryView(entry: MissionRecoveryEntry, index: number): RecoveryEntryView {
  return {
    key: `${entry.ts}-${index}-${entry.message}`,
    ts: entry.ts,
    message: entry.message,
    tone: classifyRecoveryTone(entry.message),
  }
}

function shouldCallOutRecovery(message: string) {
  const normalized = message.toLowerCase()
  return normalized.includes('recover')
    || normalized.includes('resume')
    || normalized.includes('paused')
    || normalized.includes('blocked')
    || normalized.includes('interrupt')
    || normalized.includes('failed')
    || normalized.includes('rejected')
}

function resolveJobPhase(status: string): JobPhase {
  switch (status) {
    case 'running':
    case 'initializing':
    case 'orchestrator_turn':
      return 'running'
    case 'waiting_user':
    case 'waiting_review':
    case 'waiting_knowledge_decision':
    case 'paused':
      return 'waiting'
    case 'blocked':
      return 'blocked'
    case 'completed':
      return 'completed'
    case 'failed':
    case 'cancelled':
      return 'failed'
    case 'awaiting_input':
    case 'draft':
    case 'ready':
    default:
      return 'ready'
  }
}

function resolveJobHeadline(status: string, blockers: JobBlockerView[]) {
  switch (status) {
    case 'running':
      return 'Work is actively running.'
    case 'initializing':
      return 'Job is initializing.'
    case 'orchestrator_turn':
      return 'Coordinator turn is running.'
    case 'awaiting_input':
      return 'Job is ready for input.'
    case 'waiting_user':
      return 'Waiting for user clarification.'
    case 'waiting_review':
      return 'Waiting for review decision.'
    case 'waiting_knowledge_decision':
      return 'Waiting for knowledge decision.'
    case 'paused':
      return 'Job is paused.'
    case 'blocked':
      return blockers[0]?.kind === 'external_dependency'
        ? 'Blocked by an external dependency.'
        : 'Blocked until a gate is cleared.'
    case 'completed':
      return 'Job completed.'
    case 'failed':
      return 'Job failed.'
    case 'cancelled':
      return 'Job was cancelled.'
    case 'unknown':
      return 'Job status is unavailable.'
    case 'draft':
    case 'ready':
    default:
      return 'Job is ready.'
  }
}

function resolveJobDetail(input: {
  status: string
  blockers: JobBlockerView[]
  readyTaskCount: number
  runningTaskCount: number
  completedTaskCount: number
  failedTaskCount: number
}) {
  const taskSummary = joinSegments([
    input.runningTaskCount > 0 ? pluralize(input.runningTaskCount, 'running task') : '',
    input.readyTaskCount > 0 ? pluralize(input.readyTaskCount, 'ready task') : '',
    input.completedTaskCount > 0 ? pluralize(input.completedTaskCount, 'completed task') : '',
    input.failedTaskCount > 0 ? pluralize(input.failedTaskCount, 'failed task') : '',
  ])

  if (input.status === 'blocked' || input.status.startsWith('waiting_')) {
    if (input.blockers[0]?.summary) {
      return input.blockers[0].summary
    }
  }

  if (input.status === 'paused') {
    return taskSummary || 'Resume the mission to continue delegated work.'
  }

  if (input.status === 'initializing' || input.status === 'orchestrator_turn' || input.status === 'running') {
    return taskSummary || (input.blockers[0]?.summary ?? null)
  }

  if (input.status === 'completed') {
    return taskSummary || 'All known delegated tasks have finished.'
  }

  if (input.status === 'failed' || input.status === 'cancelled') {
    return input.blockers[0]?.summary || taskSummary || 'Inspect blockers, task results, and worker output for the failure cause.'
  }

  if (input.status === 'awaiting_input' || input.status === 'ready' || input.status === 'draft') {
    return taskSummary || 'No delegated work is running yet.'
  }

  if (input.status === 'unknown') {
    return taskSummary || null
  }

  return taskSummary || (input.blockers[0]?.summary ?? null)
}

function resolveRecoveryState(input: {
  status: string
  blockers: JobBlockerView[]
  recoveryEntries: RecoveryEntryView[]
}) {
  const latest = input.recoveryEntries[0] ?? null
  const latestMessage = latest?.message ?? null

  switch (input.status) {
    case 'paused':
      return {
        label: 'resume available',
        hint: latestMessage || 'Mission is paused. Resume when you are ready to continue.',
        tone: 'warning' as const,
        canResume: true,
        resumeActionLabel: 'Resume',
      }
    case 'blocked':
      return {
        label: 'unblock required',
        hint: latestMessage || input.blockers[0]?.summary || 'Clear the blocker, then retry resume.',
        tone: latest?.tone ?? 'error',
        canResume: true,
        resumeActionLabel: 'Retry Resume',
      }
    case 'waiting_user':
      return {
        label: 'waiting for input',
        hint: latestMessage || 'Provide the required user input before resuming.',
        tone: 'warning' as const,
        canResume: false,
        resumeActionLabel: null,
      }
    case 'waiting_review':
      return {
        label: 'waiting for review',
        hint: latestMessage || 'Resolve the review decision below before resuming.',
        tone: 'warning' as const,
        canResume: false,
        resumeActionLabel: null,
      }
    case 'waiting_knowledge_decision':
      return {
        label: 'waiting for knowledge decision',
        hint: latestMessage || 'Resolve the knowledge decision before resuming.',
        tone: 'warning' as const,
        canResume: false,
        resumeActionLabel: null,
      }
    case 'failed':
    case 'cancelled':
      return {
        label: latestMessage ? 'recovery diagnostics' : 'failure details',
        hint: latestMessage || input.blockers[0]?.summary || 'Inspect recent diagnostics before retrying or restarting.',
        tone: latest?.tone ?? 'error',
        canResume: false,
        resumeActionLabel: null,
      }
    case 'running':
    case 'initializing':
    case 'orchestrator_turn':
      if (latestMessage && shouldCallOutRecovery(latestMessage)) {
        return {
          label: latest?.tone === 'success' ? 'recovered recently' : 'recovery activity',
          hint: latestMessage,
          tone: latest?.tone ?? 'info',
          canResume: false,
          resumeActionLabel: null,
        }
      }
      return {
        label: null,
        hint: null,
        tone: null,
        canResume: false,
        resumeActionLabel: null,
      }
    case 'completed':
      if (latestMessage && shouldCallOutRecovery(latestMessage)) {
        return {
          label: 'recovery history',
          hint: latestMessage,
          tone: latest?.tone ?? 'info',
          canResume: false,
          resumeActionLabel: null,
        }
      }
      return {
        label: null,
        hint: null,
        tone: null,
        canResume: false,
        resumeActionLabel: null,
      }
    default:
      return {
        label: latestMessage ? 'recent diagnostics' : null,
        hint: latestMessage,
        tone: latest?.tone ?? null,
        canResume: false,
        resumeActionLabel: null,
      }
  }
}

export function maxUpdatedAt(layer1: Layer1SnapshotPayload | null): number {
  if (!layer1) return 0
  const times = [
    layer1.chapter_card?.updated_at,
    layer1.recent_facts?.updated_at,
    layer1.active_cast?.updated_at,
    layer1.active_foreshadowing?.updated_at,
    layer1.previous_summary?.updated_at,
    layer1.risk_ledger?.updated_at,
  ].filter((v): v is number => typeof v === 'number' && Number.isFinite(v))
  return times.length ? Math.max(...times) : 0
}

export function resolveWorkersDefaultOpen(input: {
  liveState: string
  workerEntries: Array<[string, { status: string }]>
  failedResults: number
}) {
  if (input.failedResults > 0) return true
  if (input.workerEntries.some(([, info]) => info.status === 'running')) return true
  return input.liveState === 'running' || input.liveState === 'initializing'
}

export function computeIssueCountByWorkerId(results: MissionResultEntry[]) {
  const counts: Record<string, number> = {}
  for (const entry of results) {
    const wid = String(entry.actor_id ?? '')
    if (!wid) continue
    const issues = Array.isArray(entry.issues) ? entry.issues.length : 0
    counts[wid] = (counts[wid] ?? 0) + issues
  }
  return counts
}

export function resolveTaskResultsFromJobSnapshotFirst(input: {
  jobSnapshot: Pick<JobSnapshot, 'task_results'> | null
  statusDetail: Pick<MissionGetStatusOutput, 'task_results' | 'handoffs'> | null
}): MissionResultEntry[] {
  const snapshotResults = input.jobSnapshot
    ? getMissionResultEntriesFromJobSnapshot(input.jobSnapshot)
    : []

  if (snapshotResults.length > 0) {
    return snapshotResults
  }

  return input.statusDetail ? getMissionResultEntries(input.statusDetail) : []
}

export function resolveJobStatusView(input: {
  jobSnapshot: JobSnapshot | null
  statusDetail: Pick<MissionGetStatusOutput, 'recovery_log'> | null
  fallbackStatus: string
}): JobStatusView {
  const snapshot = input.jobSnapshot
  const status = snapshot?.status ?? input.fallbackStatus ?? 'unknown'
  const blockers = (snapshot?.blockers ?? [])
    .map(toJobBlockerView)
    .sort((left, right) => right.updatedAt - left.updatedAt)
  const recoveryEntries = getMissionRecoveryEntries(input.statusDetail)
    .map((entry, index) => toRecoveryEntryView(entry, index))
  const readyTaskCount = snapshot?.ready_tasks.length ?? 0
  const runningTaskCount = snapshot?.running_tasks.length ?? 0
  const completedTaskCount = snapshot?.completed_tasks.length ?? 0
  const failedTaskCount = snapshot?.failed_tasks.length ?? 0
  const phase = resolveJobPhase(status)
  const recoveryState = resolveRecoveryState({
    status,
    blockers,
    recoveryEntries,
  })

  return {
    hasSnapshot: snapshot != null,
    phase,
    phaseLabel: humanizeStatus(phase),
    statusLabel: humanizeStatus(status),
    headline: resolveJobHeadline(status, blockers),
    detail: resolveJobDetail({
      status,
      blockers,
      readyTaskCount,
      runningTaskCount,
      completedTaskCount,
      failedTaskCount,
    }),
    updatedAt: snapshot?.updated_at ?? 0,
    blockerCount: blockers.length,
    readyTaskCount,
    runningTaskCount,
    completedTaskCount,
    failedTaskCount,
    blockers,
    recoveryLabel: recoveryState.label,
    recoveryHint: recoveryState.hint,
    recoveryTone: recoveryState.tone,
    recoveryEntries,
    latestRecoveryEntry: recoveryEntries[0] ?? null,
    canResume: recoveryState.canResume,
    resumeActionLabel: recoveryState.resumeActionLabel,
    shouldOpenRecovery: recoveryEntries.length > 0 && (phase === 'waiting' || phase === 'blocked' || phase === 'failed'),
  }
}
