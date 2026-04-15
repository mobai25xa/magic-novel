import { useCallback, useState } from 'react'

import {
  getMissionRecoveryEntries,
  loadAgentProviderSettings,
  missionCancelFeature,
  missionContextpackBuildFeature,
  missionContextpackGetLatestFeature,
  missionInterruptFeature,
  missionLayer1UpsertFeature,
  missionPauseFeature,
  missionRecoverFeature,
  missionReviewAnswerFeature,
  missionResumeFeature,
  missionStartFeature,
} from '@/features/agent-chat'

import { currentActiveChapterPath } from '@/lib/agent-chat/session/session-persistence-store'

import type { ActiveCastDraft } from '@/components/ai/active-cast-editor'
import type { ChapterCardDraft } from '@/components/ai/chapter-card-editor'
import type { RecentFactsDraft } from '@/components/ai/recent-facts-editor'

import {
  buildActiveCastDoc,
  buildChapterCardDoc,
  buildRecentFactsDoc,
  normalizeChapterPath,
  normalizeScopeRefFromChapterPath,
  resolveTokenBudget,
} from './mission-panel-action-helpers'
import type { BackendState } from './useMissionPanelBackend'

function inferScopeSuggestion(backend: BackendState): { scopeRef?: string; scopeLocator?: string } {
  const macroState = backend.macroState?.state
  const macroConfig = backend.macroState?.config
  const macroId = macroState?.macro_id || macroConfig?.macro_id
  const macroIdx = typeof macroState?.current_index === 'number' ? macroState.current_index : -1
  const macroChapter = macroIdx >= 0 ? macroState?.chapters?.[macroIdx] : undefined
  if (macroId && macroChapter?.chapter_ref) {
    return {
      scopeRef: `macro:${macroId}:${macroChapter.chapter_ref}`,
      scopeLocator: macroChapter.write_path ? normalizeChapterPath(macroChapter.write_path) : undefined,
    }
  }

  const activeChapter = currentActiveChapterPath()
  if (activeChapter && activeChapter.trim()) {
    const locator = normalizeChapterPath(activeChapter)
    return {
      scopeRef: normalizeScopeRefFromChapterPath(locator),
      scopeLocator: locator,
    }
  }

  return {}
}

function normalizeMissionStatus(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}

function recoveryMessageLooksRecoverable(message: string) {
  const normalized = message.toLowerCase()
  return normalized.includes('recover')
    || normalized.includes('interrupted')
    || normalized.includes('fake running')
    || normalized.includes('worker stop failed')
}

function resolveRecoveryActionState(backend: BackendState) {
  const liveState = normalizeMissionStatus(
    backend.jobSnapshot?.status
    ?? backend.statusDetail?.state?.state
    ?? '',
  )
  const runningTaskCount = backend.jobSnapshot?.running_tasks.length ?? 0
  const hasInProgressFeature = (backend.statusDetail?.features?.features ?? [])
    .some((feature) => feature.status === 'in_progress')
  const latestRecoveryMessage = getMissionRecoveryEntries(backend.statusDetail)[0]?.message?.trim() ?? ''
  const canRecover = (
    liveState === 'running'
    || liveState === 'initializing'
    || liveState === 'orchestrator_turn'
  )
    && runningTaskCount === 0
    && !hasInProgressFeature
    && recoveryMessageLooksRecoverable(latestRecoveryMessage)

  return {
    canRecover,
    recoverLabel: canRecover ? 'Recover' : null,
  }
}

function resolveRequiredScopeRef(input: {
  existingScopeRef?: string
  suggestedScopeRef?: string
  promptField: string
}) {
  const existing = String(input.existingScopeRef ?? '').trim()
  if (existing) {
    return existing
  }

  const suggested = String(input.suggestedScopeRef ?? '').trim()
  const prompted = String(
    window.prompt(`Enter ${input.promptField} (required)`, suggested) ?? '',
  ).trim()

  if (!prompted) {
    throw new Error(`${input.promptField} cannot be empty`)
  }

  return prompted
}

export type MissionPanelActions = {
  loading: boolean
  error: string | null
  buildingContextPack: boolean
  reviewActionLoading: boolean
  reviewActionError: string | null
  onSaveChapterCard: (draft: ChapterCardDraft) => Promise<void>
  onSaveRecentFacts: (draft: RecentFactsDraft) => Promise<void>
  onSaveActiveCast: (draft: ActiveCastDraft) => Promise<void>
  onCreateDefaultChapterCard: () => void
  onInferScopeFromCurrentChapter: () => void
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onCancel: () => void
  onAbandon: () => void
  canRecover: boolean
  recoverLabel: string | null
  onRecover: () => void
  onBuildContextPack: () => void
  onFetchLatestContextPack: () => void
  onAnswerOption: (option: string) => void
  scrollToDecision: () => void
}

export const useMissionPanelActions = Object.assign(
  (input: {
    projectPath: string
    missionId: string
    backend: BackendState
    setBackend: React.Dispatch<React.SetStateAction<BackendState>>
    refreshStatus: () => Promise<void>
    reviewDecisionRef: React.RefObject<HTMLDivElement | null>
  }): MissionPanelActions => {
    const { projectPath, missionId, backend, setBackend, refreshStatus, reviewDecisionRef } = input

    const [loading, setLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [buildingContextPack, setBuildingContextPack] = useState(false)
    const [reviewActionLoading, setReviewActionLoading] = useState(false)
    const [reviewActionError, setReviewActionError] = useState<string | null>(null)
    const recoveryActionState = resolveRecoveryActionState(backend)

    const runMissionAction = useCallback(async (fn: () => Promise<void>) => {
      setError(null)
      setLoading(true)
      try {
        await fn()
        await refreshStatus()
      } catch (e) {
        setError(String(e))
      } finally {
        setLoading(false)
      }
    }, [refreshStatus])

    const onStart = useCallback(() => {
      void runMissionAction(async () => {
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
      })
    }, [missionId, projectPath, runMissionAction])

    const onSaveChapterCard = useCallback(async (draft: ChapterCardDraft) => {
      const existing = backend.layer1?.chapter_card ?? null
      const suggestion = inferScopeSuggestion(backend)
      const scopeRef = resolveRequiredScopeRef({
        existingScopeRef: existing?.scope_ref,
        suggestedScopeRef: suggestion.scopeRef,
        promptField: 'chapter_card.scope_ref',
      })
      const scopeLocator = String(existing?.scope_locator ?? '').trim()
        || String(suggestion.scopeLocator ?? '').trim()

      await missionLayer1UpsertFeature({
        project_path: projectPath,
        mission_id: missionId,
        kind: 'chapter_card',
        doc: buildChapterCardDoc({
          existing,
          scopeRef,
          scopeLocator: scopeLocator || undefined,
          draft,
        }),
      })
    }, [backend, missionId, projectPath])

    const onSaveRecentFacts = useCallback(async (draft: RecentFactsDraft) => {
      const suggestion = inferScopeSuggestion(backend)
      const existing = backend.layer1?.recent_facts ?? null
      const scopeRef = resolveRequiredScopeRef({
        existingScopeRef: backend.layer1?.chapter_card?.scope_ref || existing?.scope_ref,
        suggestedScopeRef: suggestion.scopeRef,
        promptField: 'recent_facts.scope_ref',
      })

      await missionLayer1UpsertFeature({
        project_path: projectPath,
        mission_id: missionId,
        kind: 'recent_facts',
        doc: buildRecentFactsDoc({ existing, scopeRef, draft }),
      })
    }, [backend, missionId, projectPath])

    const onSaveActiveCast = useCallback(async (draft: ActiveCastDraft) => {
      const suggestion = inferScopeSuggestion(backend)
      const existing = backend.layer1?.active_cast ?? null
      const scopeRef = resolveRequiredScopeRef({
        existingScopeRef: backend.layer1?.chapter_card?.scope_ref || existing?.scope_ref,
        suggestedScopeRef: suggestion.scopeRef,
        promptField: 'active_cast.scope_ref',
      })

      await missionLayer1UpsertFeature({
        project_path: projectPath,
        mission_id: missionId,
        kind: 'active_cast',
        doc: buildActiveCastDoc({ existing, scopeRef, draft }),
      })
    }, [backend, missionId, projectPath])

    const onCreateDefaultChapterCard = useCallback(() => {
      void (async () => {
        setError(null)
        try {
          if (backend.layer1?.chapter_card) {
            return
          }

          const suggestion = inferScopeSuggestion(backend)
          const scopeRef = String(window.prompt(
            'Enter chapter_card.scope_ref (required)',
            suggestion.scopeRef ?? '',
          ) ?? '').trim()

          if (!scopeRef) {
            return
          }

          const scopeLocator = String(window.prompt(
            'Enter chapter_card.scope_locator (optional)',
            suggestion.scopeLocator ?? '',
          ) ?? '').trim()

          await missionLayer1UpsertFeature({
            project_path: projectPath,
            mission_id: missionId,
            kind: 'chapter_card',
            doc: buildChapterCardDoc({
              existing: null,
              scopeRef,
              scopeLocator: scopeLocator || undefined,
              draft: {
                objective: '',
                hard_constraints: [],
                success_criteria: [],
              },
            }),
          })
        } catch (e) {
          setError(String(e))
        }
      })()
    }, [backend, missionId, projectPath])

    const onInferScopeFromCurrentChapter = useCallback(() => {
      void (async () => {
        setError(null)
        try {
          const existing = backend.layer1?.chapter_card ?? null
          const existingScopeRef = String(existing?.scope_ref ?? '').trim()
          if (existingScopeRef) {
            return
          }

          const suggestion = inferScopeSuggestion(backend)
          const scopeRef = String(suggestion.scopeRef ?? '').trim()
          if (!scopeRef) {
            setError('Unable to infer scope_ref: open a chapter or provide one manually.')
            return
          }

          const scopeLocator = String(suggestion.scopeLocator ?? '').trim()

          await missionLayer1UpsertFeature({
            project_path: projectPath,
            mission_id: missionId,
            kind: 'chapter_card',
            doc: buildChapterCardDoc({
              existing,
              scopeRef,
              scopeLocator: scopeLocator || undefined,
              draft: {
                objective: existing?.objective ?? '',
                hard_constraints: existing?.hard_constraints ?? [],
                success_criteria: existing?.success_criteria ?? [],
              },
            }),
          })
        } catch (e) {
          setError(String(e))
        }
      })()
    }, [backend, missionId, projectPath])

    const onPause = useCallback(() => {
      void runMissionAction(() => missionPauseFeature(projectPath, missionId))
    }, [missionId, projectPath, runMissionAction])

    const onResume = useCallback(() => {
      void runMissionAction(() => missionResumeFeature(projectPath, missionId))
    }, [missionId, projectPath, runMissionAction])

    const onCancel = useCallback(() => {
      if (!window.confirm('Interrupt this mission now? Running work will stop and pending work can be resumed later.')) return
      void runMissionAction(() => missionInterruptFeature(projectPath, missionId))
    }, [missionId, projectPath, runMissionAction])

    const onRecover = useCallback(() => {
      if (!window.confirm('Recover this mission from fake running? Stale running assignments will be rolled back to pending.')) return
      void runMissionAction(() => missionRecoverFeature(projectPath, missionId))
    }, [missionId, projectPath, runMissionAction])

    const onAbandon = useCallback(() => {
      if (!window.confirm('Abandon this mission? This cannot be resumed.')) return
      void runMissionAction(() => missionCancelFeature(projectPath, missionId))
    }, [missionId, projectPath, runMissionAction])

    const onBuildContextPack = useCallback(() => {
      void (async () => {
        setBuildingContextPack(true)
        setBackend((prev) => ({ ...prev, contextPackError: null }))
        try {
          const chapterCard = backend.layer1?.chapter_card ?? null
          const budget = resolveTokenBudget({
            workflowKind: chapterCard?.workflow_kind,
            macroBudget: backend.macroState?.config?.token_budget,
          })

          const activeChapterPath = String(chapterCard?.scope_locator ?? '').trim()
            || normalizeChapterPath(currentActiveChapterPath() ?? '')
          const scopeRef = String(chapterCard?.scope_ref ?? '').trim()
            || (activeChapterPath ? normalizeScopeRefFromChapterPath(activeChapterPath) : '')
            || undefined

          const built = await missionContextpackBuildFeature({
            project_path: projectPath,
            mission_id: missionId,
            token_budget: budget,
            scope_ref: scopeRef,
          })

          try {
            const latest = await missionContextpackGetLatestFeature(projectPath, missionId)
            setBackend((prev) => ({ ...prev, contextPack: latest ?? built, contextPackError: null }))
          } catch {
            setBackend((prev) => ({ ...prev, contextPack: built, contextPackError: null }))
          }
        } catch (e) {
          setBackend((prev) => ({ ...prev, contextPack: null, contextPackError: String(e) }))
        } finally {
          setBuildingContextPack(false)
        }
      })()
    }, [backend, missionId, projectPath, setBackend])

    const onFetchLatestContextPack = useCallback(() => {
      void (async () => {
        try {
          const latest = await missionContextpackGetLatestFeature(projectPath, missionId)
          setBackend((prev) => ({ ...prev, contextPack: latest, contextPackError: null }))
        } catch (e) {
          setBackend((prev) => ({ ...prev, contextPackError: String(e) }))
        }
      })()
    }, [missionId, projectPath, setBackend])

    const onAnswerOption = useCallback((selectedOption: string) => {
      void (async () => {
        const reviewId = backend.reviewDecision?.review_id || backend.reviewReport?.review_id
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
      })()
    }, [backend.reviewDecision?.review_id, backend.reviewReport?.review_id, missionId, projectPath, refreshStatus])

    const scrollToDecision = useCallback(() => {
      reviewDecisionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
    }, [reviewDecisionRef])

    return {
      loading,
      error,
      buildingContextPack,
      reviewActionLoading,
      reviewActionError,
      onSaveChapterCard,
      onSaveRecentFacts,
      onSaveActiveCast,
      onCreateDefaultChapterCard,
      onInferScopeFromCurrentChapter,
      onStart,
      onPause,
      onResume,
      onCancel,
      onAbandon,
      canRecover: recoveryActionState.canRecover,
      recoverLabel: recoveryActionState.recoverLabel,
      onRecover,
      onBuildContextPack,
      onFetchLatestContextPack,
      onAnswerOption,
      scrollToDecision,
    }
  },
  { displayName: 'useMissionPanelActions' },
)
