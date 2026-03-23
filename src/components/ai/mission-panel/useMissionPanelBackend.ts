import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react'

import {
  type MissionUiState,
} from '@/lib/agent-chat/runtime-backend-events'
import {
  getMissionUiStateByJobId,
  subscribeMissionUiStateByJobId,
} from '@/lib/agent-chat/runtime-backend-events/mission-store'
import {
  missionContextpackGetLatestFeature,
  missionGetStatusFeature,
  missionKnowledgeGetLatestFeature,
  missionLayer1GetFeature,
  missionMacroGetStateFeature,
  missionReviewGetLatestFeature,
  missionReviewGetPendingDecisionFeature,
  missionReviewListFeature,
} from '@/features/agent-chat'
import { missionStatusToJobSnapshot } from '@/lib/tauri-commands/mission'
import type { JobSnapshot } from '@/types/agent-job'

import { loadKnowledgeTimelineFromArtifacts, type KnowledgeTimelineEntry } from './knowledge/timeline'
import type {
  ContextPackPayload,
  Layer1SnapshotPayload,
  KnowledgeLatestPayload,
  MacroStatePayload,
  MissionStatusPayload,
  ReviewDecisionPayload,
  ReviewHistoryPayload,
  ReviewReportPayload,
} from './types'

export type BackendState = {
  statusDetail: MissionStatusPayload | null
  jobSnapshot: JobSnapshot | null
  layer1: Layer1SnapshotPayload | null
  contextPack: ContextPackPayload
  reviewReport: ReviewReportPayload
  reviewHistory: ReviewHistoryPayload
  reviewDecision: ReviewDecisionPayload
  knowledgeLatest: KnowledgeLatestPayload | null
  knowledgeTimeline: KnowledgeTimelineEntry[] | null
  macroState: MacroStatePayload | null
  layer1Error: string | null
  contextPackError: string | null
  reviewError: string | null
  knowledgeError: string | null
  knowledgeTimelineError: string | null
  macroError: string | null
  initialLoading: boolean
}

export type MissionPanelBackend = {
  missionUi: MissionUiState | null
  backend: BackendState
  setBackend: Dispatch<SetStateAction<BackendState>>
  refreshStatus: () => Promise<void>
}

const INITIAL_BACKEND_STATE: BackendState = {
  statusDetail: null,
  jobSnapshot: null,
  layer1: null,
  contextPack: null,
  reviewReport: null,
  reviewHistory: null,
  reviewDecision: null,
  knowledgeLatest: null,
  knowledgeTimeline: null,
  macroState: null,
  layer1Error: null,
  contextPackError: null,
  reviewError: null,
  knowledgeError: null,
  knowledgeTimelineError: null,
  macroError: null,
  initialLoading: true,
}

function normalizeJobId(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}

function sortReviewHistory(value: unknown) {
  const list = Array.isArray(value) ? value : []
  return [...list].sort((a, b) => {
    const left = typeof a?.generated_at === 'number' ? a.generated_at : 0
    const right = typeof b?.generated_at === 'number' ? b.generated_at : 0
    return right - left
  })
}

export const useMissionPanelBackend = Object.assign(
  (input: { projectPath: string; missionId: string }): MissionPanelBackend => {
    const { projectPath, missionId } = input

    const lastLayer1UpdatedAtRef = useRef(0)
    const lastContextPackBuiltAtRef = useRef(0)
    const lastReviewUpdatedAtRef = useRef(0)
    const lastKnowledgeUpdatedAtRef = useRef(0)
    const lastMacroStateUpdatedAtRef = useRef(0)
    const lastMissionStateRef = useRef('')
    const pendingAutoRefreshRef = useRef<ReturnType<typeof setTimeout> | null>(null)
    const [jobStoreId, setJobStoreId] = useState(() => normalizeJobId(missionId))

    const [missionUi, setMissionUi] = useState<MissionUiState | null>(
      () => getMissionUiStateByJobId(missionId),
    )
    const [backend, setBackend] = useState<BackendState>(INITIAL_BACKEND_STATE)

    useEffect(() => {
      setJobStoreId(normalizeJobId(missionId))
    }, [missionId])

    useEffect(() => {
      const primaryJobId = normalizeJobId(jobStoreId) || normalizeJobId(missionId)
      const fallbackMissionId = normalizeJobId(missionId)
      const fallbackJobId = fallbackMissionId && fallbackMissionId !== primaryJobId
        ? fallbackMissionId
        : ''

      const resolveMissionUiState = () => {
        const primaryState = primaryJobId ? getMissionUiStateByJobId(primaryJobId) : null
        if (primaryState) {
          return primaryState
        }

        return fallbackJobId ? getMissionUiStateByJobId(fallbackJobId) : null
      }

      setMissionUi(resolveMissionUiState())

      const unsubscribers: Array<() => void> = []

      if (primaryJobId) {
        unsubscribers.push(subscribeMissionUiStateByJobId(primaryJobId, () => {
          setMissionUi(resolveMissionUiState())
        }))
      }

      if (fallbackJobId) {
        unsubscribers.push(subscribeMissionUiStateByJobId(fallbackJobId, () => {
          setMissionUi(resolveMissionUiState())
        }))
      }

      return () => {
        for (const unsubscribe of unsubscribers) {
          unsubscribe()
        }
      }
    }, [jobStoreId, missionId])

    const refreshStatus = useCallback(async () => {
      const [statusRes, layer1Res, packRes, reviewRes, reviewListRes, decisionRes, knowledgeRes, knowledgeTimelineRes, macroRes] = await Promise.allSettled([
        missionGetStatusFeature(projectPath, missionId),
        missionLayer1GetFeature(projectPath, missionId),
        missionContextpackGetLatestFeature(projectPath, missionId),
        missionReviewGetLatestFeature(projectPath, missionId),
        missionReviewListFeature(projectPath, missionId),
        missionReviewGetPendingDecisionFeature(projectPath, missionId),
        missionKnowledgeGetLatestFeature(projectPath, missionId),
        loadKnowledgeTimelineFromArtifacts({ projectPath, missionId }),
        missionMacroGetStateFeature(projectPath, missionId),
      ])
      const statusSnapshot = statusRes.status === 'fulfilled'
        ? missionStatusToJobSnapshot(statusRes.value)
        : null
      const snapshotJobId = normalizeJobId(statusSnapshot?.job_id)

      setBackend((prev) => {
        const next: BackendState = { ...prev, initialLoading: false }

        if (statusRes.status === 'fulfilled') {
          next.statusDetail = statusRes.value
          next.jobSnapshot = statusSnapshot
        } else {
          console.warn('[MissionPanel] status fetch failed:', statusRes.reason)
        }

        if (layer1Res.status === 'fulfilled') {
          next.layer1 = layer1Res.value
          next.layer1Error = null
        } else {
          console.warn('[MissionPanel] layer1 fetch failed:', layer1Res.reason)
          next.layer1 = null
          next.layer1Error = String(layer1Res.reason)
        }

        if (packRes.status === 'fulfilled') {
          next.contextPack = packRes.value
          next.contextPackError = null
        } else {
          console.warn('[MissionPanel] contextpack fetch failed:', packRes.reason)
          next.contextPack = null
          next.contextPackError = String(packRes.reason)
        }

        if (reviewRes.status === 'fulfilled') {
          next.reviewReport = reviewRes.value
          next.reviewError = null
        } else {
          console.warn('[MissionPanel] review latest fetch failed:', reviewRes.reason)
          next.reviewReport = null
          next.reviewError = String(reviewRes.reason)
        }

        if (reviewListRes.status === 'fulfilled') {
          next.reviewHistory = sortReviewHistory(reviewListRes.value) as ReviewHistoryPayload
        } else {
          console.warn('[MissionPanel] review list fetch failed:', reviewListRes.reason)
          next.reviewHistory = null
        }

        if (decisionRes.status === 'fulfilled') {
          next.reviewDecision = decisionRes.value
        } else {
          console.warn('[MissionPanel] review decision fetch failed:', decisionRes.reason)
          next.reviewDecision = null
        }

        if (knowledgeRes.status === 'fulfilled') {
          next.knowledgeLatest = knowledgeRes.value
          next.knowledgeError = null
        } else {
          console.warn('[MissionPanel] knowledge fetch failed:', knowledgeRes.reason)
          next.knowledgeLatest = null
          next.knowledgeError = String(knowledgeRes.reason)
        }

        if (knowledgeTimelineRes.status === 'fulfilled') {
          next.knowledgeTimeline = knowledgeTimelineRes.value
          next.knowledgeTimelineError = null
        } else {
          console.warn('[MissionPanel] knowledge timeline fetch failed:', knowledgeTimelineRes.reason)
          next.knowledgeTimeline = null
          next.knowledgeTimelineError = String(knowledgeTimelineRes.reason)
        }

        if (macroRes.status === 'fulfilled') {
          next.macroState = macroRes.value
          next.macroError = null
        } else {
          console.warn('[MissionPanel] macro state fetch failed:', macroRes.reason)
          next.macroState = null
          next.macroError = String(macroRes.reason)
        }

        return next
      })

      if (snapshotJobId) {
        setJobStoreId((prev) => (prev === snapshotJobId ? prev : snapshotJobId))
      }
    }, [missionId, projectPath])

    useEffect(() => {
      void refreshStatus()
    }, [refreshStatus])

    useEffect(() => {
      const layer1Ts = missionUi?.layer1UpdatedAt ?? 0
      const packTs = missionUi?.contextPackBuiltAt ?? 0
      const reviewTs = missionUi?.reviewUpdatedAt ?? 0
      const knowledgeTs = missionUi?.knowledgeUpdatedAt ?? 0
      const macroTs = missionUi?.macroStateUpdatedAt ?? 0
      const missionState = String(missionUi?.state ?? '').trim()

      const layer1Changed = layer1Ts > 0 && layer1Ts !== lastLayer1UpdatedAtRef.current
      const packChanged = packTs > 0 && packTs !== lastContextPackBuiltAtRef.current
      const reviewChanged = reviewTs > 0 && reviewTs !== lastReviewUpdatedAtRef.current
      const knowledgeChanged = knowledgeTs > 0 && knowledgeTs !== lastKnowledgeUpdatedAtRef.current
      const macroChanged = macroTs > 0 && macroTs !== lastMacroStateUpdatedAtRef.current
      const stateChanged = Boolean(missionState) && missionState !== lastMissionStateRef.current

      if (!layer1Changed && !packChanged && !reviewChanged && !knowledgeChanged && !macroChanged && !stateChanged) return
      if (layer1Changed) lastLayer1UpdatedAtRef.current = layer1Ts
      if (packChanged) lastContextPackBuiltAtRef.current = packTs
      if (reviewChanged) lastReviewUpdatedAtRef.current = reviewTs
      if (knowledgeChanged) lastKnowledgeUpdatedAtRef.current = knowledgeTs
      if (macroChanged) lastMacroStateUpdatedAtRef.current = macroTs
      if (stateChanged) lastMissionStateRef.current = missionState
      if (pendingAutoRefreshRef.current) return

      pendingAutoRefreshRef.current = setTimeout(() => {
        pendingAutoRefreshRef.current = null
        void refreshStatus()
      }, 120)
    }, [
      missionUi?.contextPackBuiltAt,
      missionUi?.knowledgeUpdatedAt,
      missionUi?.layer1UpdatedAt,
      missionUi?.macroStateUpdatedAt,
      missionUi?.reviewUpdatedAt,
      missionUi?.state,
      refreshStatus,
    ])

    useEffect(() => () => {
      if (pendingAutoRefreshRef.current) {
        clearTimeout(pendingAutoRefreshRef.current)
        pendingAutoRefreshRef.current = null
      }
    }, [])

    return { missionUi, backend, setBackend, refreshStatus }
  },
  { displayName: 'useMissionPanelBackend' },
)
