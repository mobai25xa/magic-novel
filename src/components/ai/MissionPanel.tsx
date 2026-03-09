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

import { useEffect, useState, useCallback } from 'react'

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

export function MissionPanel({ projectPath, missionId, onClose }: MissionPanelProps) {
  const [missionUi, setMissionUi] = useState<MissionUiState | null>(
    getMissionUiState,
  )
  const [statusDetail, setStatusDetail] = useState<MissionStatusPayload | null>(null)
  const [loading, setLoading] = useState(false)
  const [initialLoading, setInitialLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setInitialLoading(true)
    setStatusDetail(null)
    setMissionUi(getMissionUiState())
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
    try {
      const s = await missionGetStatusFeature(projectPath, missionId)
      setStatusDetail(s)
    } catch (e) {
      // Non-fatal: live events will keep UI updated
      console.warn('[MissionPanel] status fetch failed:', e)
    } finally {
      setInitialLoading(false)
    }
  }, [projectPath, missionId])

  useEffect(() => {
    refreshStatus()
  }, [refreshStatus])

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
