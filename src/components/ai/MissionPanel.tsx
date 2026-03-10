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
} from '@/features/agent-chat'

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

export function MissionPanel({ projectPath, missionId, onClose }: MissionPanelProps) {
  const lastLayer1UpdatedAtRef = useRef<number>(0)
  const lastContextPackBuiltAtRef = useRef<number>(0)
  const pendingAutoRefreshRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const [missionUi, setMissionUi] = useState<MissionUiState | null>(
    getMissionUiState,
  )
  const [statusDetail, setStatusDetail] = useState<MissionStatusPayload | null>(null)
  const [layer1, setLayer1] = useState<Layer1SnapshotPayload | null>(null)
  const [contextPack, setContextPack] = useState<ContextPackPayload>(null)
  const [layer1Error, setLayer1Error] = useState<string | null>(null)
  const [contextPackError, setContextPackError] = useState<string | null>(null)
  const [buildingContextPack, setBuildingContextPack] = useState(false)
  const [layer1Open, setLayer1Open] = useState(false)
  const [layer1UserToggled, setLayer1UserToggled] = useState(false)
  const [contextPackOpen, setContextPackOpen] = useState(false)
  const [contextPackUserToggled, setContextPackUserToggled] = useState(false)
  const [loading, setLoading] = useState(false)
  const [initialLoading, setInitialLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setInitialLoading(true)
    setStatusDetail(null)
    setLayer1(null)
    setContextPack(null)
    setLayer1Error(null)
    setContextPackError(null)
    setBuildingContextPack(false)
    setLayer1UserToggled(false)
    setContextPackUserToggled(false)
    setMissionUi(getMissionUiState())

    lastLayer1UpdatedAtRef.current = 0
    lastContextPackBuiltAtRef.current = 0
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
    const [statusRes, layer1Res, packRes] = await Promise.allSettled([
      missionGetStatusFeature(projectPath, missionId),
      missionLayer1GetFeature(projectPath, missionId),
      missionContextpackGetLatestFeature(projectPath, missionId),
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

    setInitialLoading(false)
  }, [projectPath, missionId])

  useEffect(() => {
    refreshStatus()
  }, [refreshStatus])

  // Optional P1: auto-refresh when backend emits Layer1/ContextPack events.
  useEffect(() => {
    const layer1Ts = missionUi?.layer1UpdatedAt ?? 0
    const packTs = missionUi?.contextPackBuiltAt ?? 0

    const layer1Changed = layer1Ts > 0 && layer1Ts !== lastLayer1UpdatedAtRef.current
    const packChanged = packTs > 0 && packTs !== lastContextPackBuiltAtRef.current

    if (!layer1Changed && !packChanged) {
      return
    }

    if (layer1Changed) {
      lastLayer1UpdatedAtRef.current = layer1Ts
    }
    if (packChanged) {
      lastContextPackBuiltAtRef.current = packTs
    }

    if (pendingAutoRefreshRef.current) {
      return
    }

    pendingAutoRefreshRef.current = setTimeout(() => {
      pendingAutoRefreshRef.current = null
      void refreshStatus()
    }, 120)
  }, [missionUi?.layer1UpdatedAt, missionUi?.contextPackBuiltAt, refreshStatus])

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

      {/* M2: Layer1 / ContextPack */}
      <div className="space-y-2">
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
