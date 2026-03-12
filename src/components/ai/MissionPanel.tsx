/**
 * MissionPanel.tsx
 *
 * Dev4 (UI/Event Owner) — Mission status panel.
 *
 * Consumes `subscribeMissionUiState` from runtime-backend-events and
 * provides start / pause / cancel controls via Tauri commands.
 *
 * Based on docs/magic_plan/plan_agent_parallel/supplement.md S4.1
 */

import { useEffect, useState, useCallback, useRef } from 'react'

import { readTextFile } from '@tauri-apps/plugin-fs'

import {
  AiPanelCardShell,
  AiPanelIconButton,
  Badge,
  Button,
} from '@/magic-ui/components'
import {
  getMissionUiState,
  subscribeMissionUiState,
  type MissionUiState,
} from '@/lib/agent-chat/runtime-backend-events'
import {
  loadAgentProviderSettings,
  missionStartFeature,
  missionPauseFeature,
  missionResumeFeature,
  missionCancelFeature,
  missionGetStatusFeature,
  missionLayer1GetFeature,
  missionContextpackGetLatestFeature,
  missionContextpackBuildFeature,
  missionReviewGetLatestFeature,
  missionReviewGetPendingDecisionFeature,
  missionReviewFixupStartFeature,
  missionReviewAnswerFeature,
  missionKnowledgeGetLatestFeature,
  missionKnowledgeDecideFeature,
  missionKnowledgeApplyFeature,
  missionKnowledgeRollbackFeature,
} from '@/features/agent-chat'
import type { KnowledgeDelta, KnowledgeProposalBundle } from '@/types/knowledge'

// ── Sub-components ───────────────────────────────────────────────

function StateBadge({ state }: { state: string }) {
  const colorMap: Record<string, 'default' | 'info' | 'success' | 'warning' | 'error'> = {
    awaiting_input: 'default',
    initializing: 'info',
    running: 'info',
    paused: 'warning',
    orchestrator_turn: 'info',
    completed: 'success',
  }
  const color = colorMap[state] ?? 'default'
  return (
    <Badge color={color} variant="soft" size="sm">
      {state.replace('_', ' ')}
    </Badge>
  )
}

function FeatureStatusBadge({ status }: { status: string }) {
  const colorMap: Record<string, 'default' | 'info' | 'success' | 'error'> = {
    pending: 'default',
    in_progress: 'info',
    completed: 'success',
    failed: 'error',
    cancelled: 'default',
  }
  const color = colorMap[status] ?? 'default'
  return (
    <Badge color={color} variant="soft" size="sm" className={status === 'cancelled' ? 'line-through' : undefined}>
      {status.replace('_', ' ')}
    </Badge>
  )
}

function WorkerRow({
  workerId,
  info,
}: {
  workerId: string
  info: { featureId: string; status: string; summary?: string; updatedAt: number }
}) {
  return (
    <div className="flex items-start gap-2 py-1 text-xs border-b border-b-border last:border-0">
      <span className="font-mono text-muted-foreground truncate max-w-[90px]" title={workerId}>
        {workerId.slice(0, 10)}…
      </span>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1">
          <span className="truncate opacity-80">{info.featureId}</span>
          <FeatureStatusBadge status={info.status} />
        </div>
        {info.summary && (
          <p className="text-muted-foreground truncate mt-0.5">{info.summary}</p>
        )}
      </div>
    </div>
  )
}

function ProgressLog({ entries }: { entries: Array<{ ts: number; message: string }> }) {
  if (entries.length === 0) return null
  return (
    <div className="mt-2">
      <p className="text-xs font-medium text-secondary-foreground mb-1">Progress</p>
      <div className="max-h-28 overflow-y-auto space-y-0.5 text-xs font-mono">
        {entries
          .slice()
          .reverse()
          .map((e, i) => (
            <div key={i} className="">
              <span className="opacity-50">{new Date(e.ts).toLocaleTimeString()} </span>
              {e.message}
            </div>
          ))}
      </div>
    </div>
  )
}

// ── Main component ───────────────────────────────────────────────

interface MissionPanelProps {
  projectPath: string
  missionId: string
  /** Optional: called when user requests to close the panel */
  onClose?: () => void
}

type MissionStatusPayload = Awaited<ReturnType<typeof missionGetStatusFeature>>
type Layer1SnapshotPayload = Awaited<ReturnType<typeof missionLayer1GetFeature>>
type ContextPackPayload = Awaited<ReturnType<typeof missionContextpackGetLatestFeature>>
type ReviewReportPayload = Awaited<ReturnType<typeof missionReviewGetLatestFeature>>
type ReviewDecisionPayload = Awaited<ReturnType<typeof missionReviewGetPendingDecisionFeature>>
type KnowledgeLatestPayload = Awaited<ReturnType<typeof missionKnowledgeGetLatestFeature>>

type KnowledgeHistoryEntry = {
  kind: 'bundle' | 'delta'
  ts: number | null
  summary: string
  raw: unknown
}

function joinFsPath(base: string, ...parts: string[]) {
  const trimmedBase = base.replace(/[\\/]+$/, '')
  const sep = trimmedBase.includes('\\') ? '\\' : '/'
  const cleaned = parts
    .filter(Boolean)
    .map((part) => part.replace(/^[\\/]+/, '').replace(/[\\/]+$/, ''))
  return [trimmedBase, ...cleaned].join(sep)
}

function isMissingFileError(reason: unknown): boolean {
  const text = String(reason ?? '').toLowerCase()
  return text.includes('enoent')
    || text.includes('no such file')
    || text.includes('not found')
    || text.includes('os error 2')
}

function parseJsonlTail(text: string, maxLines: number): unknown[] {
  const lines = text
    .split(/\r?\n/g)
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(-maxLines)

  const out: unknown[] = []
  for (const line of lines) {
    try {
      out.push(JSON.parse(line))
    } catch {
      // ignore
    }
  }
  return out
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value : value == null ? '' : String(value)
}

function asNumber(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }
  return null
}

function summarizeKnowledgeBundle(raw: unknown): string {
  const obj = asRecord(raw)
  if (!obj) return 'bundle (invalid)'

  const id = asString(obj.bundle_id)
  const scope = asString(obj.scope_ref)
  const ts = asNumber(obj.generated_at) ?? null
  const items = Array.isArray(obj.proposal_items) ? obj.proposal_items.length : 0

  return `${formatMaybeTime(ts)} · bundle ${id ? id.slice(0, 10) : '—'} · ${items} items${scope ? ` · ${scope}` : ''}`
}

function summarizeKnowledgeDelta(raw: unknown): string {
  const obj = asRecord(raw)
  if (!obj) return 'delta (invalid)'

  const id = asString(obj.knowledge_delta_id)
  const status = asString(obj.status)
  const ts = asNumber(obj.applied_at) ?? asNumber(obj.generated_at) ?? null
  const conflicts = Array.isArray(obj.conflicts) ? obj.conflicts.length : 0
  const accepted = Array.isArray(obj.accepted_item_ids) ? obj.accepted_item_ids.length : 0
  const rejected = Array.isArray(obj.rejected_item_ids) ? obj.rejected_item_ids.length : 0

  return `${formatMaybeTime(ts)} · delta ${id ? id.slice(0, 10) : '—'} · ${status || '—'} · ${conflicts} conflicts · ${accepted}/${rejected}`
}

function formatMaybeTime(ts?: number | null): string {
  if (!ts || !Number.isFinite(ts)) return '—'
  try {
    return new Date(ts).toLocaleString()
  } catch {
    return String(ts)
  }
}

function maxUpdatedAt(layer1: Layer1SnapshotPayload | null): number {
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

function looksLikeMissingTauriCommand(reason: unknown): boolean {
  const text = String(reason ?? '').toLowerCase()
  return text.includes('unknown command')
    || text.includes('unknown ipc')
    || text.includes('not found')
    || text.includes('command not found')
}

export function MissionPanel({ projectPath, missionId, onClose }: MissionPanelProps) {
  const lastLayer1UpdatedAtRef = useRef<number>(0)
  const lastContextPackBuiltAtRef = useRef<number>(0)
  const lastReviewUpdatedAtRef = useRef<number>(0)
  const lastKnowledgeUpdatedAtRef = useRef<number>(0)
  const pendingAutoRefreshRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const [missionUi, setMissionUi] = useState<MissionUiState | null>(
    getMissionUiState,
  )
  const [statusDetail, setStatusDetail] = useState<MissionStatusPayload | null>(null)
  const [layer1, setLayer1] = useState<Layer1SnapshotPayload | null>(null)
  const [contextPack, setContextPack] = useState<ContextPackPayload>(null)
  const [reviewReport, setReviewReport] = useState<ReviewReportPayload>(null)
  const [reviewDecision, setReviewDecision] = useState<ReviewDecisionPayload>(null)
  const [knowledgeBundle, setKnowledgeBundle] = useState<KnowledgeProposalBundle | null>(null)
  const [knowledgeDelta, setKnowledgeDelta] = useState<KnowledgeDelta | null>(null)
  const [knowledgeError, setKnowledgeError] = useState<string | null>(null)
  const [knowledgeHistory, setKnowledgeHistory] = useState<KnowledgeHistoryEntry[]>([])
  const [knowledgeHistoryError, setKnowledgeHistoryError] = useState<string | null>(null)
  const [layer1Error, setLayer1Error] = useState<string | null>(null)
  const [contextPackError, setContextPackError] = useState<string | null>(null)
  const [reviewReportError, setReviewReportError] = useState<string | null>(null)
  const [buildingContextPack, setBuildingContextPack] = useState(false)
  const [reviewOpen, setReviewOpen] = useState(false)
  const [reviewUserToggled, setReviewUserToggled] = useState(false)
  const [knowledgeOpen, setKnowledgeOpen] = useState(false)
  const [knowledgeUserToggled, setKnowledgeUserToggled] = useState(false)
  const [layer1Open, setLayer1Open] = useState(false)
  const [layer1UserToggled, setLayer1UserToggled] = useState(false)
  const [contextPackOpen, setContextPackOpen] = useState(false)
  const [contextPackUserToggled, setContextPackUserToggled] = useState(false)
  const [reviewActionLoading, setReviewActionLoading] = useState(false)
  const [reviewActionError, setReviewActionError] = useState<string | null>(null)
  const [knowledgeActionLoading, setKnowledgeActionLoading] = useState(false)
  const [knowledgeActionError, setKnowledgeActionError] = useState<string | null>(null)
  const [knowledgeDecisions, setKnowledgeDecisions] = useState<Record<string, 'accept' | 'reject' | 'unset'>>({})
  const [loading, setLoading] = useState(false)
  const [initialLoading, setInitialLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setInitialLoading(true)
    setStatusDetail(null)
    setLayer1(null)
    setContextPack(null)
    setReviewReport(null)
    setReviewDecision(null)
    setKnowledgeBundle(null)
    setKnowledgeDelta(null)
    setKnowledgeError(null)
    setKnowledgeHistory([])
    setKnowledgeHistoryError(null)
    setLayer1Error(null)
    setContextPackError(null)
    setReviewReportError(null)
    setBuildingContextPack(false)
    setReviewOpen(false)
    setReviewUserToggled(false)
    setKnowledgeOpen(false)
    setKnowledgeUserToggled(false)
    setLayer1UserToggled(false)
    setContextPackUserToggled(false)
    setReviewActionLoading(false)
    setReviewActionError(null)
    setKnowledgeActionLoading(false)
    setKnowledgeActionError(null)
    setKnowledgeDecisions({})
    setMissionUi(getMissionUiState())

    lastLayer1UpdatedAtRef.current = 0
    lastContextPackBuiltAtRef.current = 0
    lastReviewUpdatedAtRef.current = 0
    lastKnowledgeUpdatedAtRef.current = 0
    if (pendingAutoRefreshRef.current) {
      clearTimeout(pendingAutoRefreshRef.current)
      pendingAutoRefreshRef.current = null
    }
  }, [missionId])

  // Subscribe to live mission events
  useEffect(() => {
    const unsub = subscribeMissionUiState((state) => {
      if (!state || state.missionId === missionId) {
        setMissionUi(state)
      }
    })
    return unsub
  }, [missionId])

  // Load initial status from backend
  const refreshStatus = useCallback(async () => {
    const [statusRes, layer1Res, packRes, reviewRes, decisionRes, knowledgeRes, knowledgeHistoryRes] = await Promise.allSettled([
      missionGetStatusFeature(projectPath, missionId),
      missionLayer1GetFeature(projectPath, missionId),
      missionContextpackGetLatestFeature(projectPath, missionId),
      missionReviewGetLatestFeature(projectPath, missionId),
      missionReviewGetPendingDecisionFeature(projectPath, missionId),
      missionKnowledgeGetLatestFeature(projectPath, missionId),
      (async (): Promise<KnowledgeHistoryEntry[]> => {
        const bundlesPath = joinFsPath(
          projectPath,
          'magic_novel',
          'missions',
          missionId,
          'knowledge',
          'bundles',
          'bundles.jsonl',
        )
        const deltasPath = joinFsPath(
          projectPath,
          'magic_novel',
          'missions',
          missionId,
          'knowledge',
          'deltas',
          'deltas.jsonl',
        )

        const readMaybe = async (path: string) => {
          try {
            return await readTextFile(path)
          } catch (error) {
            if (isMissingFileError(error)) return ''
            throw error
          }
        }

        const [bundlesText, deltasText] = await Promise.all([
          readMaybe(bundlesPath),
          readMaybe(deltasPath),
        ])

        const bundleRaw = parseJsonlTail(bundlesText, 40)
        const deltaRaw = parseJsonlTail(deltasText, 40)

        const bundleEntries: KnowledgeHistoryEntry[] = bundleRaw.map((raw) => {
          const obj = asRecord(raw)
          const ts = obj ? asNumber(obj.generated_at) : null
          return {
            kind: 'bundle',
            ts,
            summary: summarizeKnowledgeBundle(raw),
            raw,
          }
        })

        const deltaEntries: KnowledgeHistoryEntry[] = deltaRaw.map((raw) => {
          const obj = asRecord(raw)
          const ts = obj ? (asNumber(obj.applied_at) ?? asNumber(obj.generated_at)) : null
          return {
            kind: 'delta',
            ts,
            summary: summarizeKnowledgeDelta(raw),
            raw,
          }
        })

        return [...bundleEntries, ...deltaEntries]
          .slice()
          .sort((a, b) => (b.ts ?? 0) - (a.ts ?? 0))
      })(),
    ])

    if (statusRes.status === 'fulfilled') {
      setStatusDetail(statusRes.value)
    } else {
      // Non-fatal: live events will keep UI updated
      console.warn('[MissionPanel] status fetch failed:', statusRes.reason)
    }

    if (layer1Res.status === 'fulfilled') {
      setLayer1(layer1Res.value)
      setLayer1Error(null)
    } else {
      console.warn('[MissionPanel] layer1 fetch failed:', layer1Res.reason)
      setLayer1(null)
      setLayer1Error(String(layer1Res.reason))
    }

    if (packRes.status === 'fulfilled') {
      setContextPack(packRes.value)
      setContextPackError(null)
    } else {
      console.warn('[MissionPanel] contextpack fetch failed:', packRes.reason)
      setContextPack(null)
      setContextPackError(String(packRes.reason))
    }

    if (reviewRes.status === 'fulfilled') {
      setReviewReport(reviewRes.value)
      setReviewReportError(null)
    } else {
      console.warn('[MissionPanel] review fetch failed:', reviewRes.reason)
      setReviewReport(null)
      setReviewReportError(String(reviewRes.reason))
    }

    if (decisionRes.status === 'fulfilled') {
      setReviewDecision(decisionRes.value)
    } else {
      // Optional: backend may not implement pending decision command yet
      console.warn('[MissionPanel] pending decision fetch failed:', decisionRes.reason)
      setReviewDecision(null)
    }

    if (knowledgeRes.status === 'fulfilled') {
      const payload = knowledgeRes.value as KnowledgeLatestPayload
      setKnowledgeBundle(payload.bundle)
      setKnowledgeDelta(payload.delta)
      setKnowledgeError(null)
    } else {
      console.warn('[MissionPanel] knowledge fetch failed:', knowledgeRes.reason)
      setKnowledgeBundle(null)
      setKnowledgeDelta(null)
      setKnowledgeError(looksLikeMissingTauriCommand(knowledgeRes.reason) ? null : String(knowledgeRes.reason))
    }

    if (knowledgeHistoryRes.status === 'fulfilled') {
      setKnowledgeHistory(knowledgeHistoryRes.value)
      setKnowledgeHistoryError(null)
    } else {
      console.warn('[MissionPanel] knowledge history read failed:', knowledgeHistoryRes.reason)
      setKnowledgeHistory([])
      setKnowledgeHistoryError(String(knowledgeHistoryRes.reason))
    }

    setInitialLoading(false)
  }, [projectPath, missionId])

  useEffect(() => {
    refreshStatus()
  }, [refreshStatus])

  useEffect(() => {
    setKnowledgeDecisions({})
  }, [knowledgeBundle?.bundle_id])

  // Optional P1: auto-refresh when backend emits Layer1/ContextPack events.
  useEffect(() => {
    const layer1Ts = missionUi?.layer1UpdatedAt ?? 0
    const packTs = missionUi?.contextPackBuiltAt ?? 0
    const reviewTs = missionUi?.reviewUpdatedAt ?? 0
    const knowledgeTs = missionUi?.knowledgeUpdatedAt ?? 0

    const layer1Changed = layer1Ts > 0 && layer1Ts !== lastLayer1UpdatedAtRef.current
    const packChanged = packTs > 0 && packTs !== lastContextPackBuiltAtRef.current
    const reviewChanged = reviewTs > 0 && reviewTs !== lastReviewUpdatedAtRef.current
    const knowledgeChanged = knowledgeTs > 0 && knowledgeTs !== lastKnowledgeUpdatedAtRef.current

    if (!layer1Changed && !packChanged && !reviewChanged && !knowledgeChanged) {
      return
    }

    if (layer1Changed) {
      lastLayer1UpdatedAtRef.current = layer1Ts
    }
    if (packChanged) {
      lastContextPackBuiltAtRef.current = packTs
    }
    if (reviewChanged) {
      lastReviewUpdatedAtRef.current = reviewTs
    }
    if (knowledgeChanged) {
      lastKnowledgeUpdatedAtRef.current = knowledgeTs
    }

    if (pendingAutoRefreshRef.current) {
      return
    }

    pendingAutoRefreshRef.current = setTimeout(() => {
      pendingAutoRefreshRef.current = null
      void refreshStatus()
    }, 120)
  }, [missionUi?.layer1UpdatedAt, missionUi?.contextPackBuiltAt, missionUi?.reviewUpdatedAt, missionUi?.knowledgeUpdatedAt, refreshStatus])

  useEffect(() => {
    return () => {
      if (pendingAutoRefreshRef.current) {
        clearTimeout(pendingAutoRefreshRef.current)
        pendingAutoRefreshRef.current = null
      }
    }
  }, [])

  // ── Actions ──────────────────────────────────────────────────

  const handleStart = useCallback(async () => {
    setError(null)
    setLoading(true)
    try {
      const settings = await loadAgentProviderSettings()
      await missionStartFeature({
        project_path: projectPath,
        mission_id: missionId,
        max_workers: 2,
        provider: 'openai-compatible',
        model: settings.openai_model,
        base_url: settings.openai_base_url,
        api_key: settings.openai_api_key,
      })
      await refreshStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handlePause = useCallback(async () => {
    setError(null)
    setLoading(true)
    try {
      await missionPauseFeature(projectPath, missionId)
      await refreshStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handleResume = useCallback(async () => {
    setError(null)
    setLoading(true)
    try {
      await missionResumeFeature(projectPath, missionId)
      await refreshStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handleCancel = useCallback(async () => {
    if (!window.confirm('Cancel this mission? This will stop all running workers.')) return
    setError(null)
    setLoading(true)
    try {
      await missionCancelFeature(projectPath, missionId)
      await refreshStatus()
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handleBuildContextPack = useCallback(async () => {
    setContextPackError(null)
    setBuildingContextPack(true)
    try {
      const built = await missionContextpackBuildFeature({
        project_path: projectPath,
        mission_id: missionId,
        token_budget: 'medium',
      })

      try {
        const latest = await missionContextpackGetLatestFeature(projectPath, missionId)
        setContextPack(latest ?? built)
      } catch {
        setContextPack(built)
      }
    } catch (e) {
      setContextPackError(String(e))
    } finally {
      setBuildingContextPack(false)
    }
  }, [projectPath, missionId])

  const handleReviewFixupStart = useCallback(async () => {
    setReviewActionError(null)
    setReviewActionLoading(true)
    try {
      await missionReviewFixupStartFeature(projectPath, missionId)
      await refreshStatus()
    } catch (e) {
      setReviewActionError(String(e))
    } finally {
      setReviewActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handleReviewAnswerOption = useCallback(async (optionId: string) => {
    setReviewActionError(null)
    setReviewActionLoading(true)
    try {
      await missionReviewAnswerFeature({
        project_path: projectPath,
        mission_id: missionId,
        review_id: reviewDecision?.review_id ?? reviewReport?.review_id,
        answer: { option_id: optionId },
      })
      await refreshStatus()
    } catch (e) {
      setReviewActionError(String(e))
    } finally {
      setReviewActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus, reviewDecision, reviewReport])

  const setKnowledgeDecision = useCallback((itemId: string, decision: 'accept' | 'reject' | 'unset') => {
    setKnowledgeDecisions((prev) => {
      const next = { ...prev }
      if (decision === 'unset') {
        delete next[itemId]
      } else {
        next[itemId] = decision
      }
      return next
    })
  }, [])

  const handleKnowledgePrefillSafe = useCallback(() => {
    const items = knowledgeBundle?.proposal_items ?? []
    const next: Record<string, 'accept' | 'reject' | 'unset'> = {}
    for (const item of items) {
      if (item.accept_policy === 'auto_if_pass') {
        next[item.item_id] = 'accept'
      }
    }
    setKnowledgeDecisions(next)
  }, [knowledgeBundle?.proposal_items])

  const handleKnowledgePrefillAcceptAll = useCallback(() => {
    const items = knowledgeBundle?.proposal_items ?? []
    const next: Record<string, 'accept' | 'reject' | 'unset'> = {}
    for (const item of items) {
      next[item.item_id] = 'accept'
    }
    setKnowledgeDecisions(next)
  }, [knowledgeBundle?.proposal_items])

  const handleKnowledgePrefillRejectAll = useCallback(() => {
    const items = knowledgeBundle?.proposal_items ?? []
    const next: Record<string, 'accept' | 'reject' | 'unset'> = {}
    for (const item of items) {
      next[item.item_id] = 'reject'
    }
    setKnowledgeDecisions(next)
  }, [knowledgeBundle?.proposal_items])

  const handleKnowledgeDecide = useCallback(async () => {
    if (!knowledgeBundle) return
    setKnowledgeActionError(null)
    setKnowledgeActionLoading(true)
    try {
      const accepted_item_ids = Object.entries(knowledgeDecisions)
        .filter(([, v]) => v === 'accept')
        .map(([k]) => k)
      const rejected_item_ids = Object.entries(knowledgeDecisions)
        .filter(([, v]) => v === 'reject')
        .map(([k]) => k)

      await missionKnowledgeDecideFeature(projectPath, missionId, {
        bundle_id: knowledgeBundle.bundle_id,
        delta_id: knowledgeDelta?.knowledge_delta_id,
        accepted_item_ids,
        rejected_item_ids,
      })
      await refreshStatus()
    } catch (e) {
      setKnowledgeActionError(String(e))
    } finally {
      setKnowledgeActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus, knowledgeBundle, knowledgeDelta?.knowledge_delta_id, knowledgeDecisions])

  const handleKnowledgeApply = useCallback(async () => {
    setKnowledgeActionError(null)
    setKnowledgeActionLoading(true)
    try {
      await missionKnowledgeApplyFeature(projectPath, missionId)
      await refreshStatus()
    } catch (e) {
      setKnowledgeActionError(String(e))
    } finally {
      setKnowledgeActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus])

  const handleKnowledgeRollback = useCallback(async () => {
    setKnowledgeActionError(null)
    setKnowledgeActionLoading(true)
    try {
      await missionKnowledgeRollbackFeature(projectPath, missionId, knowledgeDelta?.rollback?.token)
      await refreshStatus()
    } catch (e) {
      setKnowledgeActionError(String(e))
    } finally {
      setKnowledgeActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus, knowledgeDelta?.rollback?.token])

  // ── Derived state ────────────────────────────────────────────

  const liveState = statusDetail?.state.state ?? missionUi?.state ?? 'awaiting_input'
  const features = statusDetail?.features.features ?? []
  const handoffs = statusDetail?.handoffs ?? []
  const workerStatuses = missionUi?.workerStatuses ?? {}
  const progressLog = missionUi?.progressLog ?? []
  const workerEntries = Object.entries(workerStatuses).sort(([, left], [, right]) => right.updatedAt - left.updatedAt)

  const isRunning = liveState === 'running' || liveState === 'initializing'
  const isPaused = liveState === 'paused'
  const isCompleted = liveState === 'completed'
  const canStart = liveState === 'awaiting_input' || liveState === 'orchestrator_turn'
  const canPause = isRunning
  const canCancel = !isCompleted

  const chapterCard = layer1?.chapter_card ?? null
  const recentFacts = layer1?.recent_facts ?? null
  const activeCast = layer1?.active_cast ?? null
  const layer1IsEmpty = !chapterCard && !recentFacts && !activeCast
  const layer1Missing = !chapterCard || !recentFacts || !activeCast
  const layer1LastUpdatedAt = maxUpdatedAt(layer1)
  const contextPackGeneratedAt = contextPack?.generated_at ?? 0
  const contextPackStale = contextPack != null && layer1LastUpdatedAt > contextPackGeneratedAt

  const layer1HasProblem = !!layer1Error || layer1IsEmpty || layer1Missing
  const contextPackHasProblem = !!contextPackError || !contextPack || contextPackStale

  const reviewIssues = reviewReport?.issues ?? []
  const reviewOverallStatus = reviewReport?.overall_status ?? 'unknown'
  const reviewIssueCounts = reviewIssues.reduce(
    (acc, issue) => {
      acc.total += 1
      if (issue.severity === 'block') acc.block += 1
      else if (issue.severity === 'warn') acc.warn += 1
      else acc.info += 1
      return acc
    },
    { info: 0, warn: 0, block: 0, total: 0 },
  )

  const reviewDecisionPayload = missionUi?.reviewDecision ?? null
  const reviewDecisionRequired = Boolean(
    reviewDecision
      || missionUi?.reviewDecisionRequired
      || reviewDecisionPayload,
  )
  const reviewIsBlock = reviewOverallStatus === 'block' || reviewDecisionRequired

  const knowledgeDecisionPayload = missionUi?.knowledgeDecision ?? null
  const knowledgeDecisionRequired = Boolean(missionUi?.knowledgeDecisionRequired || knowledgeDecisionPayload)
  const knowledgeItems = knowledgeBundle?.proposal_items ?? []
  const knowledgeConflicts = knowledgeDelta?.conflicts ?? []
  const knowledgeBlocked = knowledgeConflicts.length > 0 || knowledgeDecisionRequired

  const knowledgeHasProblem = Boolean(knowledgeError) || knowledgeBlocked

  useEffect(() => {
    if (reviewUserToggled) return
    setReviewOpen(reviewIsBlock)
  }, [reviewIsBlock, reviewUserToggled])

  useEffect(() => {
    if (knowledgeUserToggled) return
    setKnowledgeOpen(knowledgeHasProblem)
  }, [knowledgeHasProblem, knowledgeUserToggled])

  useEffect(() => {
    if (layer1UserToggled) return
    setLayer1Open(layer1HasProblem)
  }, [layer1HasProblem, layer1UserToggled])

  useEffect(() => {
    if (contextPackUserToggled) return
    setContextPackOpen(contextPackHasProblem)
  }, [contextPackHasProblem, contextPackUserToggled])

  // ── Render ───────────────────────────────────────────────────

  return (
    <AiPanelCardShell className="p-3 bg-background">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-foreground">Mission</span>
          <StateBadge state={liveState} />
        </div>
        <div className="flex items-center gap-1">
          <AiPanelIconButton
            onClick={refreshStatus}
            title="Refresh"
          >
            ↻
          </AiPanelIconButton>
          {onClose && (
            <AiPanelIconButton
              onClick={onClose}
              title="Close"
            >
              ✕
            </AiPanelIconButton>
          )}
        </div>
      </div>

      {/* Mission ID */}
      <p className="text-xs text-muted-foreground font-mono truncate" title={missionId}>
        {missionId}
      </p>

      {initialLoading && !statusDetail ? (
        <p className="text-xs text-muted-foreground">Loading mission status…</p>
      ) : null}

      {/* Action buttons */}
      <div className="flex gap-2">
        {canStart && (
          <Button
            className="flex-1 text-xs font-medium disabled:opacity-50 hover:opacity-90"
            size="sm"
            onClick={handleStart}
            disabled={loading}
          >
            Start
          </Button>
        )}
        {isPaused && (
          <Button
            className="flex-1 text-xs font-medium disabled:opacity-50 hover:opacity-90"
            size="sm"
            onClick={handleResume}
            disabled={loading}
          >
            Resume
          </Button>
        )}
        {canPause && (
          <Button
            variant="outline"
            className="flex-1 text-xs font-medium disabled:opacity-50"
            size="sm"
            onClick={handlePause}
            disabled={loading}
          >
            Pause
          </Button>
        )}
        {canCancel && (
          <Button
            variant="destructive"
            className="flex-1 text-xs font-medium disabled:opacity-50"
            size="sm"
            onClick={handleCancel}
            disabled={loading}
          >
            Cancel
          </Button>
        )}
      </div>

      {/* Error */}
      {error && (
        <p className="text-xs text-destructive bg-danger-10 rounded px-2 py-1">
          {error}
        </p>
      )}

      {/* M3: Review Gate + M2: Layer1 / ContextPack */}
      <div className="space-y-2">
        <details
          open={reviewOpen}
          onToggle={(e) => {
            setReviewUserToggled(true)
            setReviewOpen((e.target as HTMLDetailsElement).open)
          }}
          className="rounded border border-border px-2 py-1"
        >
          <summary className="cursor-pointer select-none list-none flex items-center justify-between text-xs font-medium text-secondary-foreground">
            <span>Review</span>
            <span className="flex items-center gap-1">
              {reviewReportError ? (
                <Badge color="error" variant="soft" size="sm">error</Badge>
              ) : !reviewReport && !reviewDecisionRequired ? (
                <Badge color="warning" variant="soft" size="sm">missing</Badge>
              ) : reviewIsBlock ? (
                <Badge color="error" variant="soft" size="sm">block</Badge>
              ) : reviewOverallStatus === 'warn' ? (
                <Badge color="warning" variant="soft" size="sm">warn</Badge>
              ) : reviewOverallStatus === 'pass' ? (
                <Badge color="success" variant="soft" size="sm">pass</Badge>
              ) : (
                <Badge color="default" variant="soft" size="sm">unknown</Badge>
              )}

              {reviewIssueCounts.block ? (
                <Badge color="error" variant="soft" size="sm">{reviewIssueCounts.block} block</Badge>
              ) : null}
              {reviewIssueCounts.warn ? (
                <Badge color="warning" variant="soft" size="sm">{reviewIssueCounts.warn} warn</Badge>
              ) : null}
              {reviewReport?.recommended_action ? (
                <Badge color="default" variant="soft" size="sm">{reviewReport.recommended_action}</Badge>
              ) : null}
            </span>
          </summary>

          <div className="mt-2 text-xs space-y-2">
            {reviewReportError ? (
              <p className="text-muted-foreground">Review unavailable: {reviewReportError}</p>
            ) : null}

            {reviewActionError ? (
              <p className="text-muted-foreground">Action failed: {reviewActionError}</p>
            ) : null}

            {reviewIsBlock ? (
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={handleReviewFixupStart}
                  disabled={reviewActionLoading}
                >
                  {reviewActionLoading ? 'Working…' : 'Fix'}
                </Button>
              </div>
            ) : null}

            {reviewDecision ? (
              <div className="space-y-1">
                <p className="font-medium">Decision required</p>
                <p className="text-muted-foreground whitespace-pre-wrap">{reviewDecision.question}</p>
                {reviewDecision.context_summary ? (
                  <p className="text-muted-foreground whitespace-pre-wrap">{reviewDecision.context_summary}</p>
                ) : null}
                {reviewDecision.options.length ? (
                  <div className="space-y-1">
                    {reviewDecision.options.map((opt) => (
                      <Button
                        key={opt.option_id}
                        variant="outline"
                        size="sm"
                        className="text-xs w-full justify-start"
                        onClick={() => handleReviewAnswerOption(opt.option_id)}
                        disabled={reviewActionLoading}
                      >
                        {opt.label}
                      </Button>
                    ))}
                  </div>
                ) : (
                  <p className="text-muted-foreground">No options provided.</p>
                )}
              </div>
            ) : reviewDecisionPayload ? (
              <div className="space-y-1">
                <p className="font-medium">Decision payload</p>
                <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
                  {JSON.stringify(reviewDecisionPayload, null, 2)}
                </pre>
              </div>
            ) : reviewDecisionRequired ? (
              <p className="text-muted-foreground">Decision required (no payload yet).</p>
            ) : null}

            {reviewReport ? (
              <div className="space-y-2">
                <div className="text-muted-foreground space-y-0.5">
                  <p>
                    <span className="opacity-70">generated:</span>{' '}
                    <span className="font-mono">{formatMaybeTime(reviewReport.generated_at)}</span>
                  </p>
                  <p>
                    <span className="opacity-70">review_types:</span>{' '}
                    <span className="font-mono">{reviewReport.review_types?.join(', ') || '—'}</span>
                  </p>
                </div>

                {reviewIssues.length ? (
                  <div>
                    <p className="font-medium">Issues ({reviewIssues.length})</p>
                    <div className="space-y-1 mt-1">
                      {reviewIssues.slice(0, 20).map((issue) => {
                        const color = issue.severity === 'block'
                          ? 'error'
                          : issue.severity === 'warn'
                            ? 'warning'
                            : 'default'
                        return (
                          <div key={issue.issue_id} className="rounded border border-border p-2">
                            <div className="flex items-center gap-2">
                              <Badge color={color} variant="soft" size="sm">{issue.severity}</Badge>
                              <span className="font-mono opacity-70">{issue.review_type}</span>
                              {issue.auto_fixable ? (
                                <Badge color="info" variant="soft" size="sm">auto</Badge>
                              ) : null}
                            </div>
                            <p className="text-muted-foreground mt-0.5 whitespace-pre-wrap">{issue.summary}</p>
                            {issue.suggested_fix ? (
                              <p className="text-muted-foreground mt-0.5 whitespace-pre-wrap">
                                <span className="opacity-70">fix:</span> {issue.suggested_fix}
                              </p>
                            ) : null}
                            {issue.evidence_refs?.length ? (
                              <p className="text-muted-foreground mt-0.5">
                                <span className="opacity-70">evidence:</span>{' '}
                                <span className="font-mono">{issue.evidence_refs.slice(0, 3).join(', ')}</span>
                              </p>
                            ) : null}
                          </div>
                        )
                      })}
                      {reviewIssues.length > 20 ? (
                        <p className="text-muted-foreground">Showing first 20 issues.</p>
                      ) : null}
                    </div>
                  </div>
                ) : (
                  <p className="text-muted-foreground">No issues.</p>
                )}
              </div>
            ) : !reviewReportError ? (
              <p className="text-muted-foreground">No review report yet.</p>
            ) : null}
          </div>
        </details>

        <details
          open={knowledgeOpen}
          onToggle={(e) => {
            setKnowledgeUserToggled(true)
            setKnowledgeOpen((e.target as HTMLDetailsElement).open)
          }}
          className="rounded border border-border px-2 py-1"
        >
          <summary className="cursor-pointer select-none list-none flex items-center justify-between text-xs font-medium text-secondary-foreground">
            <span>Knowledge</span>
            <span className="flex items-center gap-1">
              {knowledgeError ? (
                <Badge color="error" variant="soft" size="sm">error</Badge>
              ) : !knowledgeBundle && !knowledgeDelta ? (
                <Badge color="warning" variant="soft" size="sm">missing</Badge>
              ) : knowledgeConflicts.length ? (
                <Badge color="error" variant="soft" size="sm">blocked</Badge>
              ) : knowledgeDelta?.status === 'applied' ? (
                <Badge color="success" variant="soft" size="sm">applied</Badge>
              ) : knowledgeDelta?.status === 'accepted' ? (
                <Badge color="info" variant="soft" size="sm">accepted</Badge>
              ) : knowledgeDelta?.status === 'rejected' ? (
                <Badge color="default" variant="soft" size="sm">rejected</Badge>
              ) : (
                <Badge color="default" variant="soft" size="sm">proposed</Badge>
              )}

              {knowledgeItems.length ? (
                <Badge color="default" variant="soft" size="sm">{knowledgeItems.length} items</Badge>
              ) : null}
              {knowledgeConflicts.length ? (
                <Badge color="error" variant="soft" size="sm">{knowledgeConflicts.length} conflicts</Badge>
              ) : null}
              {knowledgeDelta?.accepted_item_ids?.length ? (
                <Badge color="success" variant="soft" size="sm">{knowledgeDelta.accepted_item_ids.length} accepted</Badge>
              ) : null}
            </span>
          </summary>

          <div className="mt-2 text-xs space-y-2">
            {knowledgeError ? (
              <p className="text-muted-foreground">Knowledge unavailable: {knowledgeError}</p>
            ) : null}

            {knowledgeActionError ? (
              <p className="text-muted-foreground">Action failed: {knowledgeActionError}</p>
            ) : null}

            {knowledgeDecisionPayload ? (
              <details className="rounded border border-border px-2 py-1">
                <summary className="cursor-pointer select-none list-none text-xs font-medium text-secondary-foreground">
                  Decision payload (event)
                </summary>
                <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
                  {JSON.stringify(knowledgeDecisionPayload, null, 2)}
                </pre>
              </details>
            ) : null}

            {knowledgeBundle || knowledgeDelta ? (
              <div className="text-muted-foreground space-y-0.5">
                {knowledgeBundle ? (
                  <p>
                    <span className="opacity-70">bundle:</span>{' '}
                    <span className="font-mono">{knowledgeBundle.bundle_id}</span>
                    <span className="opacity-70"> · generated:</span>{' '}
                    <span className="font-mono">{formatMaybeTime(knowledgeBundle.generated_at)}</span>
                  </p>
                ) : null}
                {knowledgeDelta ? (
                  <p>
                    <span className="opacity-70">delta:</span>{' '}
                    <span className="font-mono">{knowledgeDelta.knowledge_delta_id}</span>
                    <span className="opacity-70"> · status:</span>{' '}
                    <span className="font-mono">{knowledgeDelta.status}</span>
                    {typeof knowledgeDelta.applied_at === 'number' ? (
                      <>
                        <span className="opacity-70"> · applied:</span>{' '}
                        <span className="font-mono">{formatMaybeTime(knowledgeDelta.applied_at)}</span>
                      </>
                    ) : null}
                  </p>
                ) : null}
              </div>
            ) : (
              <p className="text-muted-foreground">No knowledge writeback yet.</p>
            )}

            {knowledgeHistoryError ? (
              <p className="text-muted-foreground">History unavailable: {knowledgeHistoryError}</p>
            ) : null}

            {knowledgeHistory.length ? (
              <details className="rounded border border-border px-2 py-1">
                <summary className="cursor-pointer select-none list-none text-xs font-medium text-secondary-foreground">
                  History ({knowledgeHistory.length})
                </summary>
                <div className="mt-2 space-y-1">
                  {knowledgeHistory.slice(0, 12).map((entry, idx) => (
                    <div key={`${entry.kind}_${idx}`} className="flex items-start gap-2">
                      <Badge color="default" variant="soft" size="sm">{entry.kind}</Badge>
                      <span className="text-muted-foreground whitespace-pre-wrap">{entry.summary}</span>
                    </div>
                  ))}
                  {knowledgeHistory.length > 12 ? (
                    <p className="text-muted-foreground">Showing latest 12 entries.</p>
                  ) : null}
                </div>
              </details>
            ) : null}

            {knowledgeBundle ? (
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={handleKnowledgePrefillSafe}
                  disabled={knowledgeActionLoading}
                >
                  Accept safe
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={handleKnowledgePrefillAcceptAll}
                  disabled={knowledgeActionLoading}
                >
                  Accept all
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={handleKnowledgePrefillRejectAll}
                  disabled={knowledgeActionLoading}
                >
                  Reject all
                </Button>
                <Button
                  size="sm"
                  className="text-xs"
                  onClick={handleKnowledgeDecide}
                  disabled={knowledgeActionLoading || Object.keys(knowledgeDecisions).length === 0}
                >
                  {knowledgeActionLoading ? 'Working…' : 'Submit decision'}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={handleKnowledgeApply}
                  disabled={knowledgeActionLoading || knowledgeConflicts.length > 0}
                >
                  Apply
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="text-xs"
                  onClick={async () => {
                    if (!window.confirm('Rollback latest knowledge apply?')) return
                    await handleKnowledgeRollback()
                  }}
                  disabled={knowledgeActionLoading || knowledgeDelta?.status !== 'applied'}
                >
                  Rollback
                </Button>
              </div>
            ) : null}

            {knowledgeConflicts.length ? (
              <div className="space-y-1">
                <p className="font-medium text-secondary-foreground">Conflicts ({knowledgeConflicts.length})</p>
                <ul className="list-disc pl-4 text-muted-foreground space-y-0.5">
                  {knowledgeConflicts.slice(0, 10).map((c, idx) => (
                    <li key={idx}>
                      <span className="font-mono">{c.type}</span>: {c.message}
                    </li>
                  ))}
                </ul>
                {knowledgeConflicts.length > 10 ? (
                  <p className="text-muted-foreground">Showing first 10 conflicts.</p>
                ) : null}
              </div>
            ) : null}

            {knowledgeItems.length ? (
              <div className="space-y-1">
                <p className="font-medium text-secondary-foreground">Proposals</p>
                <div className="space-y-1">
                  {knowledgeItems.slice(0, 20).map((item) => {
                    const decision = knowledgeDecisions[item.item_id] ?? 'unset'
                    return (
                      <div key={item.item_id} className="rounded border border-border p-2">
                        <div className="flex items-center gap-1 flex-wrap">
                          <span className="font-mono opacity-80">{item.kind}</span>
                          <Badge color="default" variant="soft" size="sm">{item.op}</Badge>
                          <Badge color="default" variant="soft" size="sm">{item.accept_policy}</Badge>
                          {decision === 'accept' ? (
                            <Badge color="success" variant="soft" size="sm">accept</Badge>
                          ) : decision === 'reject' ? (
                            <Badge color="error" variant="soft" size="sm">reject</Badge>
                          ) : null}
                          {item.target_ref ? (
                            <span className="font-mono opacity-60 truncate" title={item.target_ref}>
                              {item.target_ref}
                            </span>
                          ) : null}
                        </div>
                        {item.change_reason ? (
                          <p className="text-muted-foreground mt-0.5 whitespace-pre-wrap">{item.change_reason}</p>
                        ) : null}
                        {(item.evidence_refs?.length || item.source_refs?.length) ? (
                          <p className="text-muted-foreground mt-0.5">
                            {item.evidence_refs?.length ? (
                              <>
                                <span className="opacity-70">evidence:</span>{' '}
                                <span className="font-mono">{item.evidence_refs.slice(0, 3).join(', ')}</span>
                              </>
                            ) : null}
                            {item.source_refs?.length ? (
                              <>
                                {' '}
                                <span className="opacity-70">source:</span>{' '}
                                <span className="font-mono">{item.source_refs.slice(0, 2).join(', ')}</span>
                              </>
                            ) : null}
                          </p>
                        ) : null}
                        <div className="flex gap-2 mt-1">
                          <Button
                            variant={decision === 'accept' ? 'outline' : 'outline'}
                            size="sm"
                            className="text-xs"
                            onClick={() => setKnowledgeDecision(item.item_id, 'accept')}
                            disabled={knowledgeActionLoading}
                          >
                            Accept
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            className="text-xs"
                            onClick={() => setKnowledgeDecision(item.item_id, 'reject')}
                            disabled={knowledgeActionLoading}
                          >
                            Reject
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            className="text-xs"
                            onClick={() => setKnowledgeDecision(item.item_id, 'unset')}
                            disabled={knowledgeActionLoading}
                          >
                            Clear
                          </Button>
                        </div>
                      </div>
                    )
                  })}
                  {knowledgeItems.length > 20 ? (
                    <p className="text-muted-foreground">Showing first 20 proposals.</p>
                  ) : null}
                </div>
              </div>
            ) : null}
          </div>
        </details>

        <details
          open={layer1Open}
          onToggle={(e) => {
            setLayer1UserToggled(true)
            setLayer1Open((e.target as HTMLDetailsElement).open)
          }}
          className="rounded border border-border px-2 py-1"
        >
          <summary className="cursor-pointer select-none list-none flex items-center justify-between text-xs font-medium text-secondary-foreground">
            <span>Layer1</span>
            <span className="flex items-center gap-1">
              {layer1Error ? (
                <Badge color="error" variant="soft" size="sm">error</Badge>
              ) : layer1IsEmpty ? (
                <Badge color="warning" variant="soft" size="sm">empty</Badge>
              ) : layer1Missing ? (
                <Badge color="warning" variant="soft" size="sm">missing</Badge>
              ) : (
                <Badge color="success" variant="soft" size="sm">ok</Badge>
              )}
            </span>
          </summary>

          <div className="mt-2 text-xs space-y-2">
            {layer1Error ? (
              <p className="text-muted-foreground">Layer1 unavailable: {layer1Error}</p>
            ) : null}

            <div className="space-y-1">
              <p className="font-medium">chapter_card</p>
              {chapterCard ? (
                <div className="text-muted-foreground space-y-0.5">
                  <p><span className="opacity-70">objective:</span> {chapterCard.objective}</p>
                  <p className="flex gap-2 flex-wrap">
                    <span className="opacity-70">workflow:</span> <span className="font-mono">{chapterCard.workflow_kind}</span>
                    <span className="opacity-70">status:</span> <span className="font-mono">{chapterCard.status}</span>
                    <span className="opacity-70">updated:</span> <span className="font-mono">{formatMaybeTime(chapterCard.updated_at)}</span>
                  </p>
                  {chapterCard.hard_constraints?.length ? (
                    <p><span className="opacity-70">constraints:</span> {chapterCard.hard_constraints.length}</p>
                  ) : null}
                  {chapterCard.success_criteria?.length ? (
                    <p><span className="opacity-70">success:</span> {chapterCard.success_criteria.length}</p>
                  ) : null}
                </div>
              ) : (
                <p className="text-muted-foreground">missing</p>
              )}
            </div>

            <div className="space-y-1">
              <p className="font-medium">recent_facts ({recentFacts?.facts?.length ?? 0})</p>
              {recentFacts?.facts?.length ? (
                <ul className="list-disc pl-4 text-muted-foreground space-y-0.5">
                  {recentFacts.facts.slice(0, 8).map((f, idx) => (
                    <li key={idx}>
                      {f.summary} <span className="font-mono opacity-70">[{f.confidence}]</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted-foreground">{recentFacts ? 'empty' : 'missing'}</p>
              )}
            </div>

            <div className="space-y-1">
              <p className="font-medium">active_cast ({activeCast?.cast?.length ?? 0})</p>
              {activeCast?.cast?.length ? (
                <ul className="list-disc pl-4 text-muted-foreground space-y-0.5">
                  {activeCast.cast.slice(0, 8).map((c, idx) => (
                    <li key={idx}>
                      <span className="font-mono">{c.character_ref}</span>: {c.current_state_summary}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted-foreground">{activeCast ? 'empty' : 'missing'}</p>
              )}
            </div>
          </div>
        </details>

        <details
          open={contextPackOpen}
          onToggle={(e) => {
            setContextPackUserToggled(true)
            setContextPackOpen((e.target as HTMLDetailsElement).open)
          }}
          className="rounded border border-border px-2 py-1"
        >
          <summary className="cursor-pointer select-none list-none flex items-center justify-between text-xs font-medium text-secondary-foreground">
            <span>ContextPack</span>
            <span className="flex items-center gap-1">
              {contextPackError ? (
                <Badge color="error" variant="soft" size="sm">error</Badge>
              ) : !contextPack ? (
                <Badge color="warning" variant="soft" size="sm">missing</Badge>
              ) : contextPackStale ? (
                <Badge color="warning" variant="soft" size="sm">stale</Badge>
              ) : (
                <Badge color="success" variant="soft" size="sm">ok</Badge>
              )}
              {contextPack?.token_budget ? (
                <Badge color="default" variant="soft" size="sm">{contextPack.token_budget}</Badge>
              ) : null}
            </span>
          </summary>

          <div className="mt-2 text-xs space-y-2">
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                className="text-xs"
                onClick={handleBuildContextPack}
                disabled={buildingContextPack}
              >
                {buildingContextPack ? 'Building…' : 'Build/Refresh'}
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="text-xs"
                onClick={async () => {
                  try {
                    const latest = await missionContextpackGetLatestFeature(projectPath, missionId)
                    setContextPack(latest)
                    setContextPackError(null)
                  } catch (e) {
                    setContextPackError(String(e))
                  }
                }}
                disabled={buildingContextPack}
              >
                Fetch latest
              </Button>
            </div>

            {contextPackError ? (
              <p className="text-muted-foreground">ContextPack unavailable: {contextPackError}</p>
            ) : null}

            {contextPack ? (
              <div className="space-y-2">
                <div className="text-muted-foreground space-y-0.5">
                  <p><span className="opacity-70">objective:</span> {contextPack.objective_summary}</p>
                  <p className="flex gap-2 flex-wrap">
                    <span className="opacity-70">generated:</span> <span className="font-mono">{formatMaybeTime(contextPack.generated_at)}</span>
                    {layer1LastUpdatedAt ? (
                      <>
                        <span className="opacity-70">layer1_updated:</span> <span className="font-mono">{formatMaybeTime(layer1LastUpdatedAt)}</span>
                      </>
                    ) : null}
                  </p>
                </div>

                {contextPack.active_constraints?.length ? (
                  <div>
                    <p className="font-medium">constraints</p>
                    <ul className="list-disc pl-4 text-muted-foreground space-y-0.5">
                      {contextPack.active_constraints.slice(0, 10).map((c, idx) => (
                        <li key={idx}>{c}</li>
                      ))}
                    </ul>
                  </div>
                ) : null}

                {contextPack.key_facts?.length ? (
                  <div>
                    <p className="font-medium">key_facts</p>
                    <ul className="list-disc pl-4 text-muted-foreground space-y-0.5">
                      {contextPack.key_facts.slice(0, 10).map((f, idx) => (
                        <li key={idx}>{f}</li>
                      ))}
                    </ul>
                  </div>
                ) : null}

                {contextPack.evidence_snippets?.length ? (
                  <div>
                    <p className="font-medium">evidence (top {Math.min(contextPack.evidence_snippets.length, 6)})</p>
                    <div className="space-y-1">
                      {contextPack.evidence_snippets.slice(0, 6).map((ev, idx) => (
                        <div key={idx} className="rounded border border-border p-2">
                          <p className="font-mono opacity-80">{ev.source_ref}</p>
                          <p className="text-muted-foreground mt-0.5"><span className="opacity-70">reason:</span> {ev.reason}</p>
                          <p className="text-muted-foreground mt-0.5 whitespace-pre-wrap">{ev.snippet}</p>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : null}
              </div>
            ) : (
              <p className="text-muted-foreground">missing</p>
            )}
          </div>
        </details>
      </div>

      {/* Features list */}
      {features.length > 0 && (
        <div>
          <p className="text-xs font-medium text-secondary-foreground mb-1">
            Features ({features.filter((f) => f.status === 'completed').length}/{features.length})
          </p>
          <div className="space-y-1">
            {features.map((f) => (
              <div
                key={f.id}
                className="flex items-start gap-2 text-xs py-0.5"
              >
                <span className="mt-0.5">
                  {f.status === 'completed' ? '✓' :
                   f.status === 'failed' ? '✗' :
                   f.status === 'in_progress' ? '▶' :
                   f.status === 'cancelled' ? '–' : '○'}
                </span>
                <div className="flex-1 min-w-0">
                  <span className="opacity-80 truncate block">{f.description}</span>
                  <FeatureStatusBadge status={f.status} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Active workers */}
      {workerEntries.length > 0 && (
        <div>
          <p className="text-xs font-medium text-secondary-foreground mb-1">Workers ({workerEntries.length})</p>
          <div className="max-h-48 overflow-y-auto pr-1">
            {workerEntries.map(([wid, info]) => (
              <WorkerRow key={wid} workerId={wid} info={info} />
            ))}
          </div>
        </div>
      )}

      {/* Recent handoffs */}
      {handoffs.length > 0 && (
        <div>
          <p className="text-xs font-medium text-secondary-foreground mb-1">Handoffs</p>
          <div className="space-y-1 max-h-24 overflow-y-auto">
            {handoffs.map((h, i) => (
              <div key={i} className="text-xs flex items-start gap-1">
                <span className={h.ok ? 'text-ai-status-success' : 'text-destructive'}>
                  {h.ok ? '✓' : '✗'}
                </span>
                <div className="flex-1 min-w-0">
                  <span className="opacity-70 truncate block">{h.feature_id}</span>
                  <span className="text-muted-foreground truncate block">{h.summary}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Progress log */}
      <ProgressLog entries={progressLog} />
    </AiPanelCardShell>
  )
}

export default MissionPanel
