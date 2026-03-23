import type { AgentAskUserQuestion } from '@/agent/types'

import {
  mapStructuredAskUserQuestions,
  parseAskUserQuestionnaire,
} from '../../askuser'

import type { AgentRuntimeEventContext } from '../agent-event-context'

export function handleAskUserEvent(ctx: AgentRuntimeEventContext) {
  switch (ctx.envelope.type) {
    case 'ASKUSER_REQUESTED':
      handleAskUserRequested(ctx)
      break
    case 'ASKUSER_ANSWERED':
      handleAskUserAnswered(ctx)
      break
    default:
      break
  }
}

function shouldIgnoreAskUserRequest(input: {
  store: AgentRuntimeEventContext['store']
  turn: number
  callId: string
}) {
  const pending = input.store.pendingAskUser
  if (pending?.turn === input.turn && pending.callId === input.callId) {
    return true
  }

  const currentStep = input.store.stepsByTurnId[input.turn]?.find((step) => step.callId === input.callId)
  return currentStep?.progress === 'answered' || currentStep?.status === 'success'
}

function handleAskUserRequested(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx

  const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
  if (!callId) return
  if (shouldIgnoreAskUserRequest({ store, turn, callId })) return

  // Parse canonical structured questions first; questionnaire is display-only fallback.
  let questions: AgentAskUserQuestion[] | null = null
  let questionnaire = ''

  if (Array.isArray(payload.questions) && payload.questions.length > 0) {
    questions = mapStructuredAskUserQuestions(payload.questions)
    questionnaire = questions
      ? questions.map((q, i) => `${i + 1}. ${q.question}`).join('\n')
      : ''
  }

  if (!questions && typeof payload.questionnaire === 'string' && payload.questionnaire) {
    const parsed = parseAskUserQuestionnaire(payload.questionnaire)
    if (parsed.ok) {
      questions = parsed.questions
      questionnaire = parsed.questionnaire
    }
  }

  // If parsing failed, degrade gracefully to a single fallback question and keep interaction in chat.
  if (!questions || questions.length === 0) {
    console.warn('[agent-event] ASKUSER_REQUESTED: failed to parse questions, using fallback question')
    questions = [{
      index: 0,
      question: '无法解析问题格式。你希望我如何继续？',
      topic: 'askuser_fallback',
      options: ['继续执行', '取消本次操作'],
    }]
    questionnaire = '1. 无法解析问题格式。你希望我如何继续？'
  }

  store.setStateStatus('waiting_askuser')
  // Only unlock resume after TURN_COMPLETED confirms the loop is fully suspended.
  store.markWaitingForConfirmation(turn, {
    callId,
    llmCallId: payload.llm_call_id as string | undefined,
    toolName: String(payload.tool_name ?? 'askuser'),
    waitState: 'waiting_askuser',
    ts,
  })
  store.openAskUserRequest({
    callId,
    turn,
    questionnaire,
    questions,
    requestedAt: ts,
  })

  store.pushTurnEvent(turn, {
    type: 'ASKUSER_REQUESTED',
    ts,
    callId,
    summary: 'askuser · waiting user',
  })
}

function handleAskUserAnswered(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx

  store.clearPendingAskUser()
  store.setStateStatus('thinking')
  store.setSessionRuntimeCapability({
    runtimeState: 'running',
    canContinue: false,
    canResume: false,
    readonlyReason: undefined,
  })
  store.setTurnPhase(turn, 'synthesizing')
  store.pushTurnEvent(turn, {
    type: 'ASKUSER_ANSWERED',
    ts,
    callId: String(payload.call_id ?? payload.llm_call_id ?? ''),
    summary: 'askuser · answered',
  })
}

