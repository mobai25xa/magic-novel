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

import {
  AiPanelCardShell,
  AiPanelIconButton,
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
  missionReviewListFeature,
  missionReviewGetPendingDecisionFeature,
  missionReviewAnswerFeature,
} from '@/features/agent-chat'

import { AiStatusBadge } from './status-badge'
import { WorkerStepCard } from './worker-step-card'
import { Layer1ArtifactsCard } from './layer1-artifacts-card'
import { ContextPackCard } from './contextpack-card'
import { MissionReviewSection } from './mission-review-section'
import type { ReviewReportLike } from './review-report-card'

// ── Sub-components ───────────────────────────────────────────────
function ProgressLog({ entries }: { entries: Array<{ ts: number; message: string }> }) {
  if (entries.length === 0) return null
  return (
    <div className="mt-2 max-h-28 overflow-y-auto space-y-0.5 text-xs font-mono">
      {entries
        .slice()
        .reverse()
        .map((e, i) => (
          <div key={i}>
            <span className="opacity-50">{new Date(e.ts).toLocaleTimeString()} </span>
            {e.message}
          </div>
        ))}
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
type ReviewHistoryPayload = Awaited<ReturnType<typeof missionReviewListFeature>>
type ReviewDecisionPayload = Awaited<ReturnType<typeof missionReviewGetPendingDecisionFeature>>

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

function resolveWorkersDefaultOpen(input: {
  liveState: string
  workerEntries: Array<[string, { status: string }]>
  failedHandoffs: number
}) {
  if (input.failedHandoffs > 0) {
    return true
  }

  const hasRunning = input.workerEntries.some(([, info]) => info.status === 'running')
  if (hasRunning) {
    return true
  }

  return input.liveState === 'running' || input.liveState === 'initializing'
}

function computeIssueCountByWorkerId(handoffs: MissionStatusPayload['handoffs']) {
  const counts: Record<string, number> = {}
  for (const entry of handoffs) {
    const wid = String(entry.worker_id ?? '')
    if (!wid) {
      continue
    }

    const issues = Array.isArray(entry.issues) ? entry.issues.length : 0
    counts[wid] = (counts[wid] ?? 0) + issues
  }
  return counts
}

export function MissionPanel({ projectPath, missionId, onClose }: MissionPanelProps) {
  const lastLayer1UpdatedAtRef = useRef<number>(0)
  const lastContextPackBuiltAtRef = useRef<number>(0)
  const lastReviewUpdatedAtRef = useRef<number>(0)
  const pendingAutoRefreshRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const reviewDecisionRef = useRef<HTMLDivElement | null>(null)

  const [missionUi, setMissionUi] = useState<MissionUiState | null>(
    getMissionUiState,
  )
  const [statusDetail, setStatusDetail] = useState<MissionStatusPayload | null>(null)
  const [layer1, setLayer1] = useState<Layer1SnapshotPayload | null>(null)
  const [contextPack, setContextPack] = useState<ContextPackPayload>(null)
  const [reviewReport, setReviewReport] = useState<ReviewReportPayload>(null)
  const [reviewHistory, setReviewHistory] = useState<ReviewHistoryPayload>(null)
  const [reviewDecision, setReviewDecision] = useState<ReviewDecisionPayload>(null)
  const [layer1Error, setLayer1Error] = useState<string | null>(null)
  const [contextPackError, setContextPackError] = useState<string | null>(null)
  const [reviewError, setReviewError] = useState<string | null>(null)
  const [buildingContextPack, setBuildingContextPack] = useState(false)
  const [reviewActionLoading, setReviewActionLoading] = useState(false)
  const [reviewActionError, setReviewActionError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [initialLoading, setInitialLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [featuresOpen, setFeaturesOpen] = useState(false)
  const [workersOpenOverride, setWorkersOpenOverride] = useState<boolean | null>(null)
  const [handoffsOpenOverride, setHandoffsOpenOverride] = useState<boolean | null>(null)
  const [progressOpen, setProgressOpen] = useState(false)
  const [handoffOpenByKey, setHandoffOpenByKey] = useState<Record<string, boolean>>({})

  useEffect(() => {
    setInitialLoading(true)
    setStatusDetail(null)
    setLayer1(null)
    setContextPack(null)
    setReviewReport(null)
    setReviewHistory(null)
    setReviewDecision(null)
    setLayer1Error(null)
    setContextPackError(null)
    setReviewError(null)
    setBuildingContextPack(false)
    setReviewActionLoading(false)
    setReviewActionError(null)
    setMissionUi(getMissionUiState())
    setFeaturesOpen(false)
    setWorkersOpenOverride(null)
    setHandoffsOpenOverride(null)
    setProgressOpen(false)
    setHandoffOpenByKey({})

    lastLayer1UpdatedAtRef.current = 0
    lastContextPackBuiltAtRef.current = 0
    lastReviewUpdatedAtRef.current = 0
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
    const [statusRes, layer1Res, packRes, reviewRes, reviewListRes, decisionRes] = await Promise.allSettled([
      missionGetStatusFeature(projectPath, missionId),
      missionLayer1GetFeature(projectPath, missionId),
      missionContextpackGetLatestFeature(projectPath, missionId),
      missionReviewGetLatestFeature(projectPath, missionId),
      missionReviewListFeature(projectPath, missionId),
      missionReviewGetPendingDecisionFeature(projectPath, missionId),
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
      setReviewError(null)
    } else {
      console.warn('[MissionPanel] review latest fetch failed:', reviewRes.reason)
      setReviewReport(null)
      setReviewError(String(reviewRes.reason))
    }

    if (reviewListRes.status === 'fulfilled') {
      const list = Array.isArray(reviewListRes.value) ? reviewListRes.value : []
      const sorted = [...list].sort((a, b) => {
        const left = typeof a?.generated_at === 'number' ? a.generated_at : 0
        const right = typeof b?.generated_at === 'number' ? b.generated_at : 0
        return right - left
      })
      setReviewHistory(sorted)
    } else {
      console.warn('[MissionPanel] review list fetch failed:', reviewListRes.reason)
      setReviewHistory(null)
    }

    if (decisionRes.status === 'fulfilled') {
      setReviewDecision(decisionRes.value)
    } else {
      console.warn('[MissionPanel] review decision fetch failed:', decisionRes.reason)
      setReviewDecision(null)
    }

    setInitialLoading(false)
  }, [projectPath, missionId])

  useEffect(() => {
    refreshStatus()
  }, [refreshStatus])

  // Optional P1: auto-refresh when backend emits Layer1/ContextPack events.
  useEffect(() => {
    const layer1Ts = missionUi?.layer1UpdatedAt ?? 0
    const packTs = missionUi?.contextPackBuiltAt ?? 0
    const reviewTs = missionUi?.reviewUpdatedAt ?? 0

    const layer1Changed = layer1Ts > 0 && layer1Ts !== lastLayer1UpdatedAtRef.current
    const packChanged = packTs > 0 && packTs !== lastContextPackBuiltAtRef.current
    const reviewChanged = reviewTs > 0 && reviewTs !== lastReviewUpdatedAtRef.current

    if (!layer1Changed && !packChanged && !reviewChanged) {
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

    if (pendingAutoRefreshRef.current) {
      return
    }

    pendingAutoRefreshRef.current = setTimeout(() => {
      pendingAutoRefreshRef.current = null
      void refreshStatus()
    }, 120)
  }, [missionUi?.layer1UpdatedAt, missionUi?.contextPackBuiltAt, missionUi?.reviewUpdatedAt, refreshStatus])

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

  const handleReviewAnswerOption = useCallback(async (selectedOption: string) => {
    const reviewId = reviewDecision?.review_id || reviewReport?.review_id
    if (!reviewId) {
      setReviewActionError('Missing review_id for decision answer')
      return
    }

    setReviewActionError(null)
    setReviewActionLoading(true)
    try {
      await missionReviewAnswerFeature({
        project_path: projectPath,
        mission_id: missionId,
        answer: {
          schema_version: 1,
          review_id: reviewId,
          selected_option: selectedOption,
          answered_at: Date.now(),
        },
      })
      await refreshStatus()
    } catch (e) {
      setReviewActionError(String(e))
    } finally {
      setReviewActionLoading(false)
    }
  }, [projectPath, missionId, refreshStatus, reviewDecision?.review_id, reviewReport?.review_id])

  // ── Derived state ────────────────────────────────────────────

  const liveState = statusDetail?.state.state ?? missionUi?.state ?? 'awaiting_input'
  const features = statusDetail?.features.features ?? []
  const handoffs = statusDetail?.handoffs ?? []
  const workerStatuses = missionUi?.workerStatuses ?? {}
  const progressLog = missionUi?.progressLog ?? []
  const workerEntries = Object.entries(workerStatuses).sort(([, left], [, right]) => right.updatedAt - left.updatedAt)

  const completedFeatureCount = features.filter((f) => f.status === 'completed').length
  const failedFeatureCount = features.filter((f) => f.status === 'failed').length
  const runningWorkersCount = workerEntries.filter(([, info]) => info.status === 'running').length
  const failedWorkersCount = workerEntries.filter(([, info]) => info.status === 'failed').length
  const failedHandoffCount = handoffs.filter((h) => !h.ok).length
  const issueCountByWorkerId = computeIssueCountByWorkerId(handoffs)
  const lastProgress = progressLog.length > 0 ? progressLog[progressLog.length - 1] : null

  const isRunning = liveState === 'running' || liveState === 'initializing'
  const isPaused = liveState === 'paused'
  const isCompleted = liveState === 'completed'
  const canStart = liveState === 'awaiting_input' || liveState === 'orchestrator_turn'
  const canPause = isRunning
  const canCancel = !isCompleted

  const workersDefaultOpen = resolveWorkersDefaultOpen({
    liveState,
    workerEntries,
    failedHandoffs: failedHandoffCount,
  })

  const workersOpen = workersOpenOverride ?? workersDefaultOpen
  const handoffsOpen = handoffsOpenOverride ?? (failedHandoffCount > 0)

  const chapterCard = layer1?.chapter_card ?? null
  const recentFacts = layer1?.recent_facts ?? null
  const activeCast = layer1?.active_cast ?? null
  const layer1LastUpdatedAt = maxUpdatedAt(layer1)
  const contextPackGeneratedAt = contextPack?.generated_at ?? 0
  const contextPackStale = contextPack != null && layer1LastUpdatedAt > contextPackGeneratedAt

  const reviewReportLike = reviewReport as unknown as ReviewReportLike | null
  const reviewHistoryLike = reviewHistory as unknown as ReviewReportLike[] | null
  const waitingDecision = Boolean(reviewDecision) || Boolean(missionUi?.reviewDecisionRequired)
  const decisionReason = reviewDecision?.question
  const decisionUpdatedAt = reviewDecision?.created_at

  const fixInProgress = Boolean(missionUi?.fixupInProgress)
  const fixAttempt = missionUi?.fixupAttempt
  const fixMessage = missionUi?.fixupMessage
  const fixUpdatedAt = missionUi?.fixupUpdatedAt

  // ── Render ───────────────────────────────────────────────────

  return (
    <AiPanelCardShell className="p-3 bg-background">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-foreground">Mission</span>
          <AiStatusBadge status={liveState} />
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

      <div className="rounded-md border border-border/60 bg-muted/20 px-2.5 py-2 text-xs">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span className="text-muted-foreground">Features</span>
          <span className="font-medium text-foreground">
            {completedFeatureCount}/{features.length}
          </span>
          {failedFeatureCount > 0 ? (
            <span className="text-destructive">{`failed ${failedFeatureCount}`}</span>
          ) : null}

          <span className="text-muted-foreground">Workers</span>
          <span className="font-medium text-foreground">
            {workerEntries.length}
          </span>
          {runningWorkersCount > 0 ? (
            <span className="text-ai-status-running">{`running ${runningWorkersCount}`}</span>
          ) : null}
          {failedWorkersCount > 0 ? (
            <span className="text-destructive">{`failed ${failedWorkersCount}`}</span>
          ) : null}

          {handoffs.length > 0 ? (
            <>
              <span className="text-muted-foreground">Handoffs</span>
              <span className="font-medium text-foreground">{handoffs.length}</span>
              {failedHandoffCount > 0 ? (
                <span className="text-destructive">{`failed ${failedHandoffCount}`}</span>
              ) : null}
            </>
          ) : null}
        </div>

        {lastProgress?.message ? (
          <div className="mt-1.5 text-muted-foreground truncate" title={lastProgress.message}>
            {lastProgress.message}
          </div>
        ) : null}
      </div>

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

      {/* M2: Layer1 / ContextPack */}
      <div className="space-y-2">
        {layer1Error ? (
          <p className="text-xs text-muted-foreground">Layer1 unavailable: {layer1Error}</p>
        ) : null}

        <Layer1ArtifactsCard
          chapter_card={chapterCard}
          recent_facts={recentFacts}
          active_cast={activeCast}
          onBuildContextPack={handleBuildContextPack}
        />

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
          <p className="text-xs text-muted-foreground">ContextPack unavailable: {contextPackError}</p>
        ) : null}

        <ContextPackCard
          contextpack={contextPack}
          stale={contextPackStale}
        />
      </div>

      <div className="space-y-2">
        {reviewError ? (
          <p className="text-xs text-muted-foreground">Review unavailable: {reviewError}</p>
        ) : null}

        <MissionReviewSection
          report={reviewReportLike}
          history={reviewHistoryLike}
          historyMaxItems={5}
          showWhenEmpty
          fixInProgress={fixInProgress}
          fixAttempt={fixAttempt}
          fixMaxAttempts={2}
          fixUpdatedAt={fixUpdatedAt}
          fixMessage={fixMessage}
          waitingDecision={waitingDecision}
          decisionReason={decisionReason}
          decisionUpdatedAt={decisionUpdatedAt}
          onFix={reviewDecision?.options?.includes('auto_fix')
            ? () => handleReviewAnswerOption('auto_fix')
            : undefined}
          onDecide={waitingDecision
            ? () => {
              reviewDecisionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
            }
            : undefined}
        />

        {reviewActionError ? (
          <p className="text-xs text-muted-foreground">Review action failed: {reviewActionError}</p>
        ) : null}

        {reviewDecision ? (
          <div
            ref={reviewDecisionRef}
            className="rounded-md border border-border/60 bg-warning/5 px-2.5 py-2 text-xs"
          >
            <div className="font-medium text-secondary-foreground">Decision required</div>
            <div className="mt-1 whitespace-pre-wrap text-muted-foreground">{reviewDecision.question}</div>

            {reviewDecision.context_summary?.length ? (
              <div className="mt-2 whitespace-pre-wrap text-muted-foreground">
                {reviewDecision.context_summary.join('\n')}
              </div>
            ) : null}

            {reviewDecision.options?.length ? (
              <div className="mt-2 space-y-2">
                {reviewDecision.options.map((option) => (
                  <Button
                    key={option}
                    type="button"
                    size="sm"
                    variant="outline"
                    className="w-full justify-start text-xs font-medium disabled:opacity-50"
                    onClick={() => handleReviewAnswerOption(option)}
                    disabled={reviewActionLoading}
                  >
                    {option.replace(/_/g, ' ')}
                  </Button>
                ))}
              </div>
            ) : (
              <div className="mt-2 text-muted-foreground">No options provided.</div>
            )}
          </div>
        ) : waitingDecision && missionUi?.reviewDecision ? (
          <div
            ref={reviewDecisionRef}
            className="rounded-md border border-border/60 bg-warning/5 px-2.5 py-2 text-xs"
          >
            <div className="font-medium text-secondary-foreground">Decision required</div>
            <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
              {JSON.stringify(missionUi.reviewDecision, null, 2)}
            </pre>
          </div>
        ) : null}
      </div>

      {/* Features list */}
      {features.length > 0 ? (
        <details
          className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
          open={featuresOpen}
          onToggle={(event) => setFeaturesOpen(event.currentTarget.open)}
        >
          <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
            {`Features (${completedFeatureCount}/${features.length})`}
          </summary>

          <div className="mt-2 space-y-1">
            {features.map((f) => (
              <div key={f.id} className="flex items-start gap-2 text-xs py-0.5">
                <span className="mt-0.5">
                  {f.status === 'completed' ? '✓' :
                   f.status === 'failed' ? '✗' :
                   f.status === 'in_progress' ? '▶' :
                   f.status === 'cancelled' ? '–' : '○'}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="opacity-80 truncate block" title={f.description}>{f.description}</span>
                    <AiStatusBadge status={f.status} />
                  </div>
                  {f.skill ? (
                    <div className="mt-0.5 font-mono text-[11px] text-muted-foreground truncate" title={f.skill}>
                      {f.skill}
                    </div>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        </details>
      ) : null}

      {/* Active workers */}
      {workerEntries.length > 0 ? (
        <details
          className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
          open={workersOpen}
          onToggle={(event) => setWorkersOpenOverride(event.currentTarget.open)}
        >
          <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
            {`Workers (${workerEntries.length})`}
          </summary>

          <div className="mt-2 space-y-2 max-h-60 overflow-y-auto pr-1">
            {workerEntries.map(([wid, info]) => {
              const issueCount = issueCountByWorkerId[wid] ?? 0
              const summary = issueCount > 0
                ? [info.summary, `issues ${issueCount}`].filter(Boolean).join(' · ')
                : info.summary

              return (
                <WorkerStepCard
                  key={wid}
                  workerId={wid}
                  status={info.status}
                  featureId={info.featureId}
                  summary={summary}
                  updatedAt={info.updatedAt}
                />
              )
            })}
          </div>
        </details>
      ) : null}

      {/* Recent handoffs */}
      {handoffs.length > 0 ? (
        <details
          className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
          open={handoffsOpen}
          onToggle={(event) => setHandoffsOpenOverride(event.currentTarget.open)}
        >
          <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
            {`Handoffs (${handoffs.length})`}
          </summary>

          <div className="mt-2 space-y-2 max-h-56 overflow-y-auto pr-1">
            {handoffs.map((h, i) => {
              const issues = Array.isArray(h.issues) ? h.issues : []
              const artifacts = Array.isArray(h.artifacts) ? h.artifacts : []
              const commandsRun = Array.isArray(h.commands_run) ? h.commands_run : []
              const ok = Boolean(h.ok)

              const handoffKey = `${h.worker_id}-${h.feature_id}-${i}`
              const defaultEntryOpen = !ok || issues.length > 0
              const entryOpen = handoffOpenByKey[handoffKey] ?? defaultEntryOpen

              return (
                <details
                  key={handoffKey}
                  className="rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs"
                  open={entryOpen}
                  onToggle={(event) => {
                    const next = event.currentTarget.open
                    setHandoffOpenByKey((prev) => {
                      if (prev[handoffKey] === next) {
                        return prev
                      }
                      return {
                        ...prev,
                        [handoffKey]: next,
                      }
                    })
                  }}
                >
                  <summary className="cursor-pointer select-none">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className={ok ? 'text-ai-status-success' : 'text-destructive'}>
                            {ok ? '✓' : '✗'}
                          </span>
                          <span className="font-mono text-muted-foreground truncate" title={h.worker_id}>{h.worker_id}</span>
                          <span className="truncate opacity-80" title={h.feature_id}>{h.feature_id}</span>
                        </div>
                        <div className="mt-0.5 text-muted-foreground truncate" title={h.summary}>
                          {h.summary}
                        </div>
                      </div>

                      <AiStatusBadge
                        status={ok ? 'completed' : 'failed'}
                        label={issues.length > 0 ? `issues ${issues.length}` : undefined}
                      />
                    </div>
                  </summary>

                  {issues.length > 0 ? (
                    <div className="mt-2">
                      <div className="text-[11px] font-medium text-secondary-foreground">Issues</div>
                      <ul className="mt-1 space-y-1 text-muted-foreground">
                        {issues.map((issue, idx) => (
                          <li key={idx} className="break-words">{issue}</li>
                        ))}
                      </ul>
                    </div>
                  ) : null}

                  {artifacts.length > 0 ? (
                    <div className="mt-2">
                      <div className="text-[11px] font-medium text-secondary-foreground">Artifacts</div>
                      <ul className="mt-1 space-y-1 font-mono text-[11px] text-muted-foreground">
                        {artifacts.map((path, idx) => (
                          <li key={idx} className="break-all">{path}</li>
                        ))}
                      </ul>
                    </div>
                  ) : null}

                  {commandsRun.length > 0 ? (
                    <div className="mt-2">
                      <div className="text-[11px] font-medium text-secondary-foreground">Commands</div>
                      <ul className="mt-1 space-y-1 font-mono text-[11px] text-muted-foreground">
                        {commandsRun.map((cmd, idx) => (
                          <li key={idx} className="break-words">{cmd}</li>
                        ))}
                      </ul>
                    </div>
                  ) : null}
                </details>
              )
            })}
          </div>
        </details>
      ) : null}

      {/* Progress log */}
      {progressLog.length > 0 ? (
        <details
          className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
          open={progressOpen}
          onToggle={(event) => setProgressOpen(event.currentTarget.open)}
        >
          <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
            Progress
          </summary>
          <ProgressLog entries={progressLog} />
        </details>
      ) : null}
    </AiPanelCardShell>
  )
}

export default MissionPanel
