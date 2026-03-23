import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  ConsensusField,
  ConsensusFieldId,
  CreateProjectHandoffDraft,
  InspirationConsensusState,
  MetadataVariantId,
  OpenQuestion,
  OpenQuestionStatus,
} from '@/features/inspiration/types'
import { useStandardAiConsumerState } from '@/features/standard-ai-consumer'
import { useTranslation } from '@/hooks/use-translation'
import { AGENT_EVENT_CHANNEL } from '@/lib/agent-chat/runtime-backend-events/channels'
import type { AgentEventEnvelope } from '@/lib/agent-chat/runtime-backend-events/types'
import { formatUnknownError } from '@/lib/error-utils'
import { useToast } from '@/magic-ui/components'
import {
  inspirationTurnCancelClient,
  inspirationTurnStartClient,
} from '@/platform/tauri/clients/inspiration-engine-client'
import {
  inspirationSessionCreateClient,
  inspirationSessionDeleteClient,
  inspirationSessionListClient,
  inspirationSessionLoadClient,
  inspirationSessionSaveStateClient,
  inspirationSessionUpdateMetaClient,
  type InspirationSessionMeta,
} from '@/platform/tauri/clients/inspiration-session-client'
import {
  inspirationGenerateMetadataVariantsClient,
  type InspirationMetadataVariantCandidate,
} from '@/platform/tauri/clients/inspiration-variants-client'

import {
  CONSENSUS_FIELD_IDS,
  REQUIRED_VARIANT_FIELD_IDS,
  createEmptyConsensusState,
  createEmptyCreateHandoffDraft,
  getConsensusField,
  hasConsensusValue,
  mapInspirationMessagesToChatMessages,
  updateConsensusField,
} from './inspiration-helpers'

function createClientRequestId() {
  return `inspiration_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
}

function summarizeTurnFailure(payload: Record<string, unknown>) {
  const errorMessage = typeof payload.error_message === 'string' ? payload.error_message.trim() : ''
  if (errorMessage) {
    return errorMessage
  }

  const errorCode = typeof payload.error_code === 'string' ? payload.error_code.trim() : ''
  if (errorCode) {
    return errorCode
  }

  return 'turn failed'
}

function asRecord(input: unknown): Record<string, unknown> {
  if (input && typeof input === 'object' && !Array.isArray(input)) {
    return input as Record<string, unknown>
  }

  return {}
}

function parseErrorCode(error: unknown): string | undefined {
  const base = asRecord(error)
  const details = asRecord(base.details)
  const code = typeof base.code === 'string' ? base.code.trim() : ''
  if (code) {
    return code
  }

  const detailsCode = typeof details.code === 'string' ? details.code.trim() : ''
  return detailsCode || undefined
}

interface UseInspirationWorkflowInput {
  enabled: boolean
}

export function useInspirationWorkflow(input: UseInspirationWorkflowInput) {
  const { translations } = useTranslation()
  const { addToast } = useToast()
  const standardAi = useStandardAiConsumerState()

  const runIdRef = useRef(0)
  const createSessionOpRef = useRef<{ id: number; promise: Promise<string | null> } | null>(null)
  const createSessionSeqRef = useRef(0)
  const persistTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const [sessionId, setSessionId] = useState<string | null>(null)
  const [sessionList, setSessionList] = useState<InspirationSessionMeta[]>([])
  const [loadingSessionList, setLoadingSessionList] = useState(false)
  const [sessionListError, setSessionListError] = useState<string | null>(null)
  const [messages, setMessages] = useState<ReturnType<typeof mapInspirationMessagesToChatMessages>>([])
  const [consensus, setConsensus] = useState<InspirationConsensusState>(() => createEmptyConsensusState())
  const [openQuestions, setOpenQuestions] = useState<OpenQuestion[]>([])
  const [finalCreateHandoffDraft, setFinalCreateHandoffDraft] = useState<CreateProjectHandoffDraft | undefined>(undefined)
  const [variants, setVariants] = useState<InspirationMetadataVariantCandidate[]>([])
  const [selectedVariantId, setSelectedVariantId] = useState<MetadataVariantId | null>(null)
  const [sharedStoryCore, setSharedStoryCore] = useState('')
  const [chatInput, setChatInput] = useState('')
  const [pendingUserMessage, setPendingUserMessage] = useState<string | null>(null)
  const [pendingAssistantText, setPendingAssistantText] = useState('')
  const [activeTurnId, setActiveTurnId] = useState<number | null>(null)
  const [loadingSession, setLoadingSession] = useState(false)
  const [runningTurn, setRunningTurn] = useState(false)
  const [generatingVariants, setGeneratingVariants] = useState(false)
  const [chatError, setChatError] = useState<string | null>(null)
  const [runtimeState, setRuntimeState] = useState('idle')

  useEffect(() => {
    return () => {
      if (persistTimerRef.current) {
        clearTimeout(persistTimerRef.current)
        persistTimerRef.current = null
      }
    }
  }, [])

  const applySnapshot = useCallback((snapshot: Awaited<ReturnType<typeof inspirationSessionLoadClient>>['snapshot']) => {
    setMessages(mapInspirationMessagesToChatMessages(snapshot.messages))
    setConsensus(snapshot.consensus)
    setOpenQuestions(snapshot.open_questions)
    setFinalCreateHandoffDraft(snapshot.final_create_handoff_draft)
    setRuntimeState(snapshot.runtime_state)
  }, [])

  const clearSessionWorkingState = useCallback(() => {
    setSessionId(null)
    setMessages([])
    setConsensus(createEmptyConsensusState())
    setOpenQuestions([])
    setFinalCreateHandoffDraft(undefined)
    setVariants([])
    setSelectedVariantId(null)
    setSharedStoryCore('')
    setChatInput('')
    setPendingUserMessage(null)
    setPendingAssistantText('')
    setActiveTurnId(null)
    setLoadingSession(false)
    setRunningTurn(false)
    setGeneratingVariants(false)
    setChatError(null)
    setRuntimeState('idle')
  }, [])

  const loadSessionList = useCallback(async (limit?: number) => {
    const runId = runIdRef.current
    setLoadingSessionList(true)
    setSessionListError(null)

    try {
      const list = await inspirationSessionListClient({ limit })
      if (runIdRef.current !== runId) {
        return
      }

      setSessionList(list)
    } catch (error) {
      if (runIdRef.current !== runId) {
        return
      }

      const message = formatUnknownError(error, 'E_INSPIRATION_SESSION_LIST_FAILED')
      setSessionListError(message)
      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
    } finally {
      if (runIdRef.current === runId) {
        setLoadingSessionList(false)
      }
    }
  }, [addToast, translations.common.error])

  const loadSnapshot = useCallback(async (nextSessionId?: string) => {
    const targetSessionId = nextSessionId ?? sessionId
    if (!targetSessionId) {
      return null
    }

    const runId = runIdRef.current
    setLoadingSession(true)
    try {
      const loaded = await inspirationSessionLoadClient({ session_id: targetSessionId })
      if (runIdRef.current !== runId) {
        return loaded.snapshot
      }

      setSessionId(loaded.session_id)
      applySnapshot(loaded.snapshot)
      return loaded.snapshot
    } catch (error) {
      if (runIdRef.current !== runId) {
        return null
      }

      const message = formatUnknownError(error, 'E_INSPIRATION_SESSION_LOAD_FAILED')
      setChatError(message)
      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
      return null
    } finally {
      if (runIdRef.current === runId) {
        setLoadingSession(false)
      }
    }
  }, [addToast, applySnapshot, sessionId, translations.common.error])

  const persistStateNow = useCallback(async (
    nextConsensus: InspirationConsensusState,
    nextOpenQuestions: OpenQuestion[],
    nextFinalDraft?: CreateProjectHandoffDraft,
  ) => {
    const targetSessionId = sessionId
    if (!targetSessionId) {
      return
    }

    const runId = runIdRef.current
    try {
      await inspirationSessionSaveStateClient({
        session_id: targetSessionId,
        consensus: nextConsensus,
        open_questions: nextOpenQuestions,
        final_create_handoff_draft: nextFinalDraft,
      })
    } catch (error) {
      if (runIdRef.current !== runId) {
        return
      }

      addToast({
        title: translations.common.error,
        description: formatUnknownError(error, 'E_INSPIRATION_SESSION_SAVE_FAILED'),
        variant: 'destructive',
      })
      void loadSnapshot(targetSessionId)
    }
  }, [addToast, loadSnapshot, sessionId, translations.common.error])

  const schedulePersistState = useCallback((
    nextConsensus: InspirationConsensusState,
    nextOpenQuestions: OpenQuestion[],
    nextFinalDraft?: CreateProjectHandoffDraft,
  ) => {
    if (persistTimerRef.current) {
      clearTimeout(persistTimerRef.current)
    }

    persistTimerRef.current = setTimeout(() => {
      void persistStateNow(nextConsensus, nextOpenQuestions, nextFinalDraft)
    }, 300)
  }, [persistStateNow])

  const createSession = useCallback(async () => {
    if (!input.enabled) {
      return null
    }

    if (sessionId) {
      return sessionId
    }

    const existingOp = createSessionOpRef.current
    if (existingOp) {
      return await existingOp.promise
    }

    const runId = runIdRef.current
    const opId = createSessionSeqRef.current + 1
    createSessionSeqRef.current = opId
    setLoadingSession(true)
    setChatError(null)

    const opPromise = (async () => {
      try {
        const created = await inspirationSessionCreateClient({
          title: translations.createPage.inspirationSessionTitle,
        })

        if (runIdRef.current !== runId || createSessionOpRef.current?.id !== opId) {
          return null
        }

        const snapshot = await loadSnapshot(created.session_id)
        if (runIdRef.current !== runId || createSessionOpRef.current?.id !== opId) {
          return null
        }

        if (!snapshot) {
          return null
        }

        void loadSessionList(20)
        return created.session_id
      } catch (error) {
        if (runIdRef.current !== runId || createSessionOpRef.current?.id !== opId) {
          return null
        }

        const message = formatUnknownError(error, 'E_INSPIRATION_SESSION_CREATE_FAILED')
        setChatError(message)
        addToast({
          title: translations.common.error,
          description: message,
          variant: 'destructive',
        })
        return null
      } finally {
        if (createSessionOpRef.current?.id === opId) {
          createSessionOpRef.current = null
        }
        if (runIdRef.current === runId) {
          setLoadingSession(false)
        }
      }
    })()

    createSessionOpRef.current = { id: opId, promise: opPromise }
    return await opPromise
  }, [
    addToast,
    input.enabled,
    loadSessionList,
    loadSnapshot,
    sessionId,
    translations.common.error,
    translations.createPage.inspirationSessionTitle,
  ])

  const openSession = useCallback(async (nextSessionId: string) => {
    const targetSessionId = nextSessionId.trim()
    if (!targetSessionId) {
      return
    }

    if (loadingSession) {
      addToast({
        title: translations.common.warning,
        description: 'Session is still loading. Please try again in a moment.',
      })
      return
    }

    if (runningTurn) {
      addToast({
        title: translations.common.warning,
        description: 'Current turn is running. Stop it before switching sessions.',
      })
      return
    }

    const previousSessionId = sessionId
    runIdRef.current += 1
    createSessionOpRef.current = null
    if (persistTimerRef.current) {
      clearTimeout(persistTimerRef.current)
      persistTimerRef.current = null
    }

    clearSessionWorkingState()
    setSessionId(targetSessionId)
    const snapshot = await loadSnapshot(targetSessionId)
    if (snapshot) {
      return
    }

    await loadSessionList(20)

    if (previousSessionId && previousSessionId !== targetSessionId) {
      runIdRef.current += 1
      setSessionId(previousSessionId)
      await loadSnapshot(previousSessionId)
      return
    }

    clearSessionWorkingState()
  }, [
    addToast,
    clearSessionWorkingState,
    loadSessionList,
    loadSnapshot,
    loadingSession,
    runningTurn,
    sessionId,
    translations.common.warning,
  ])

  const newSession = useCallback(async () => {
    if (runningTurn) {
      addToast({
        title: translations.common.warning,
        description: 'Current turn is running. Stop it before creating a new session.',
      })
      return
    }

    runIdRef.current += 1
    createSessionOpRef.current = null
    if (persistTimerRef.current) {
      clearTimeout(persistTimerRef.current)
      persistTimerRef.current = null
    }

    clearSessionWorkingState()
  }, [addToast, clearSessionWorkingState, runningTurn, translations.common.warning])

  const renameSession = useCallback(async (targetSessionId: string, title: string) => {
    const normalizedSessionId = targetSessionId.trim()
    if (!normalizedSessionId) {
      return
    }

    const runId = runIdRef.current
    try {
      await inspirationSessionUpdateMetaClient({
        session_id: normalizedSessionId,
        title: title.trim(),
      })
      if (runIdRef.current !== runId) {
        return
      }

      await loadSessionList(20)
    } catch (error) {
      if (runIdRef.current !== runId) {
        return
      }

      const message = formatUnknownError(error, 'E_INSPIRATION_SESSION_RENAME_FAILED')
      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
    }
  }, [addToast, loadSessionList, translations.common.error])

  const deleteSession = useCallback(async (targetSessionId: string) => {
    const normalizedSessionId = targetSessionId.trim()
    if (!normalizedSessionId) {
      return
    }

    if (loadingSession) {
      addToast({
        title: translations.common.warning,
        description: 'Session is still loading. Please try again in a moment.',
      })
      return
    }

    if (runningTurn && normalizedSessionId === sessionId) {
      addToast({
        title: translations.common.warning,
        description: 'Current turn is running. Stop it before deleting this session.',
      })
      return
    }

    const runId = runIdRef.current
    try {
      await inspirationSessionDeleteClient({
        session_id: normalizedSessionId,
      })
      if (runIdRef.current !== runId) {
        return
      }

      if (normalizedSessionId === sessionId) {
        runIdRef.current += 1
        createSessionOpRef.current = null
        if (persistTimerRef.current) {
          clearTimeout(persistTimerRef.current)
          persistTimerRef.current = null
        }
        clearSessionWorkingState()
      }

      await loadSessionList(20)
    } catch (error) {
      if (runIdRef.current !== runId) {
        return
      }

      const code = parseErrorCode(error)
      const message = code === 'E_INSPIRATION_SESSION_DELETE_CONFLICT_ACTIVE_TURN'
        ? 'Current turn is running. Stop it before deleting this session.'
        : formatUnknownError(error, 'E_INSPIRATION_SESSION_DELETE_FAILED')

      if (code === 'E_INSPIRATION_SESSION_NOT_FOUND') {
        if (normalizedSessionId === sessionId) {
          clearSessionWorkingState()
        }
        await loadSessionList(20)
      }

      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
    }
  }, [
    addToast,
    clearSessionWorkingState,
    loadSessionList,
    loadingSession,
    runningTurn,
    sessionId,
    translations.common.error,
    translations.common.warning,
  ])

  useEffect(() => {
    if (!input.enabled || !sessionId) {
      return
    }

    const runId = runIdRef.current
    let unlisten: UnlistenFn | null = null
    void listen<AgentEventEnvelope>(AGENT_EVENT_CHANNEL, (event) => {
      if (runIdRef.current !== runId) {
        return
      }

      const envelope = event.payload
      if (envelope.session_id !== sessionId) {
        return
      }

      switch (envelope.type) {
        case 'TURN_STARTED':
          setRuntimeState('running')
          setRunningTurn(true)
          return
        case 'ASSISTANT_TEXT_DELTA':
          if (activeTurnId !== null && envelope.turn_id !== activeTurnId) {
            return
          }
          setPendingAssistantText((current) => current + String(envelope.payload.delta ?? ''))
          return
        case 'TURN_COMPLETED':
        case 'TURN_CANCELLED':
          if (activeTurnId !== null && envelope.turn_id !== activeTurnId) {
            return
          }
          setActiveTurnId(null)
          setRunningTurn(false)
          setPendingUserMessage(null)
          setPendingAssistantText('')
          void loadSnapshot(sessionId)
          return
        case 'TURN_FAILED':
          if (activeTurnId !== null && envelope.turn_id !== activeTurnId) {
            return
          }
          setActiveTurnId(null)
          setRunningTurn(false)
          setPendingUserMessage(null)
          setPendingAssistantText('')
          setChatError(summarizeTurnFailure(envelope.payload))
          void loadSnapshot(sessionId)
          return
        default:
          return
      }
    }).then((stopListening) => {
      unlisten = stopListening
    }).catch((error) => {
      console.error('Failed to listen inspiration agent events:', error)
    })

    return () => {
      unlisten?.()
    }
  }, [activeTurnId, input.enabled, loadSnapshot, sessionId])

  const sendMessage = useCallback(async () => {
    const text = chatInput.trim()
    if (!text || runningTurn) {
      return
    }

    const runId = runIdRef.current
    const targetSessionId = sessionId ?? await createSession()
    if (runIdRef.current !== runId) {
      return
    }
    if (!targetSessionId) {
      return
    }

    setChatError(null)
    setPendingUserMessage(text)
    setPendingAssistantText('')
    setChatInput('')

    try {
      const started = await inspirationTurnStartClient({
        session_id: targetSessionId,
        client_request_id: createClientRequestId(),
        user_text: text,
        capability_mode: 'planning',
        approval_mode: 'auto',
        clarification_mode: 'interactive',
         ...standardAi.providerConfig,
      })

      if (runIdRef.current !== runId) {
        return
      }

      setActiveTurnId(started.turn_id)
      setRunningTurn(true)
      setRuntimeState('running')
    } catch (error) {
      if (runIdRef.current !== runId) {
        return
      }

      const message = formatUnknownError(error, 'E_INSPIRATION_TURN_START_FAILED')
      setChatInput(text)
      setPendingUserMessage(null)
      setPendingAssistantText('')
      setRunningTurn(false)
      setChatError(message)
      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
    }
  }, [
    addToast,
    chatInput,
    createSession,
    runningTurn,
    sessionId,
    standardAi.providerConfig,
    translations.common.error,
  ])

  const cancelTurn = useCallback(async () => {
    if (!sessionId || activeTurnId === null) {
      return
    }

    try {
      await inspirationTurnCancelClient({
        session_id: sessionId,
        turn_id: activeTurnId,
      })
    } catch (error) {
      addToast({
        title: translations.common.error,
        description: formatUnknownError(error, 'E_INSPIRATION_TURN_CANCEL_FAILED'),
        variant: 'destructive',
      })
    }
  }, [activeTurnId, addToast, sessionId, translations.common.error])

  const updateConsensus = useCallback((
    fieldId: ConsensusFieldId,
    updater: (field: ConsensusField) => ConsensusField,
    mode: 'immediate' | 'deferred' = 'immediate',
  ) => {
    setConsensus((current) => {
      const nextConsensus = updateConsensusField(current, fieldId, updater)
      if (mode === 'immediate') {
        void persistStateNow(nextConsensus, openQuestions, finalCreateHandoffDraft)
      } else {
        schedulePersistState(nextConsensus, openQuestions, finalCreateHandoffDraft)
      }
      return nextConsensus
    })
  }, [finalCreateHandoffDraft, openQuestions, persistStateNow, schedulePersistState])

  const confirmConsensusField = useCallback((fieldId: ConsensusFieldId) => {
    updateConsensus(fieldId, (field) => {
      if (!field.draft_value) {
        return field
      }

      return {
        ...field,
        confirmed_value: field.draft_value,
      }
    })
  }, [updateConsensus])

  const toggleConsensusLock = useCallback((fieldId: ConsensusFieldId) => {
    updateConsensus(fieldId, (field) => ({
      ...field,
      locked: !field.locked,
    }))
  }, [updateConsensus])

  const clearConsensusField = useCallback((fieldId: ConsensusFieldId) => {
    updateConsensus(fieldId, (field) => ({
      ...field,
      draft_value: undefined,
      confirmed_value: undefined,
      last_source_turn_id: undefined,
    }))
  }, [updateConsensus])

  const updateOpenQuestionsState = useCallback((
    updater: (current: OpenQuestion[]) => OpenQuestion[],
  ) => {
    setOpenQuestions((current) => {
      const nextQuestions = updater(current)
      void persistStateNow(consensus, nextQuestions, finalCreateHandoffDraft)
      return nextQuestions
    })
  }, [consensus, finalCreateHandoffDraft, persistStateNow])

  const updateOpenQuestionStatus = useCallback((
    questionId: string,
    status: OpenQuestionStatus,
  ) => {
    updateOpenQuestionsState((current) => current.map((question) => (
      question.question_id === questionId
        ? { ...question, status }
        : question
    )))
  }, [updateOpenQuestionsState])

  const updateFinalDraft = useCallback((patch: Partial<CreateProjectHandoffDraft>) => {
    setFinalCreateHandoffDraft((current) => {
      const baseDraft = current ?? createEmptyCreateHandoffDraft()
      const nextDraft = { ...baseDraft, ...patch }
      schedulePersistState(consensus, openQuestions, nextDraft)
      return nextDraft
    })
  }, [consensus, openQuestions, schedulePersistState])

  const selectVariant = useCallback((candidate: InspirationMetadataVariantCandidate) => {
    setSelectedVariantId(candidate.variant.variant_id)
    setFinalCreateHandoffDraft(candidate.create_handoff)
    void persistStateNow(consensus, openQuestions, candidate.create_handoff)
  }, [consensus, openQuestions, persistStateNow])

  const generateVariants = useCallback(async () => {
    const runId = runIdRef.current
    setGeneratingVariants(true)
    setChatError(null)

    try {
      const output = await inspirationGenerateMetadataVariantsClient({ consensus })
      if (runIdRef.current !== runId) {
        return null
      }

      setVariants(output.variants)
      setSharedStoryCore(output.shared_story_core)

      if (output.variants[0]) {
        const selectedCandidate = output.variants.find(
          (candidate) => candidate.variant.variant_id === selectedVariantId,
        ) ?? output.variants[0]

        setSelectedVariantId(selectedCandidate.variant.variant_id)
        setFinalCreateHandoffDraft((current) => current ?? selectedCandidate.create_handoff)
        if (!finalCreateHandoffDraft) {
          void persistStateNow(consensus, openQuestions, selectedCandidate.create_handoff)
        }
      }

      return output
    } catch (error) {
      if (runIdRef.current !== runId) {
        return null
      }

      const message = formatUnknownError(error, 'E_INSPIRATION_VARIANTS_FAILED')
      setChatError(message)
      addToast({
        title: translations.common.error,
        description: message,
        variant: 'destructive',
      })
      return null
    } finally {
      if (runIdRef.current === runId) {
        setGeneratingVariants(false)
      }
    }
  }, [
    addToast,
    consensus,
    finalCreateHandoffDraft,
    openQuestions,
    persistStateNow,
    selectedVariantId,
    translations.common.error,
  ])

  const reset = useCallback((options?: { suspendAutoCreate?: boolean; preserveSessionList?: boolean }) => {
    runIdRef.current += 1
    createSessionOpRef.current = null
    if (persistTimerRef.current) {
      clearTimeout(persistTimerRef.current)
      persistTimerRef.current = null
    }

    clearSessionWorkingState()
    if (!options?.preserveSessionList) {
      setSessionList([])
      setSessionListError(null)
      setLoadingSessionList(false)
    }
  }, [clearSessionWorkingState])

  const missingRequiredFields = useMemo(() => (
    REQUIRED_VARIANT_FIELD_IDS.filter((fieldId) => !hasConsensusValue(getConsensusField(consensus, fieldId)))
  ), [consensus])

  const fieldsWithContentCount = useMemo(() => (
    CONSENSUS_FIELD_IDS.filter((fieldId) => hasConsensusValue(getConsensusField(consensus, fieldId))).length
  ), [consensus])

  return {
    availableModels: standardAi.availableModels,
    selectedModel: standardAi.selectedModel,
    onSelectModel: standardAi.handleSelectModel,
    sessionId,
    sessionList,
    loadingSessionList,
    sessionListError,
    loadingSession,
    runningTurn,
    generatingVariants,
    runtimeState,
    chatError,
    messages,
    consensus,
    openQuestions,
    finalCreateHandoffDraft,
    variants,
    selectedVariantId,
    sharedStoryCore,
    chatInput,
    setChatInput,
    pendingUserMessage,
    pendingAssistant: runningTurn
      ? {
          label: translations.createPage.inspirationThinking,
          content: pendingAssistantText,
        }
      : null,
    missingRequiredFields,
    fieldsWithContentCount,
    sendMessage,
    cancelTurn,
    loadSnapshot,
    loadSessionList,
    openSession,
    newSession,
    renameSession,
    deleteSession,
    confirmConsensusField,
    toggleConsensusLock,
    clearConsensusField,
    updateOpenQuestionStatus,
    updateFinalDraft,
    generateVariants,
    selectVariant,
    reset,
  }
}
