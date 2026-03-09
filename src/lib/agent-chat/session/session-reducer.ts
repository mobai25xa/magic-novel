import type {
  AgentCompactionMeta,
  AgentLoopStopReason,
  AgentPendingAskUserRequest,
  AgentTodoState,
} from '@/agent/types'
import type { TurnTimelineSnapshot } from '../timeline'

import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnState,
  ChatToolTrace,
  ChatUiMessage,
  OpenAiMessage,
} from '../types'
import { replaySessionState } from './session-events'
import {
  buildAnswersByTurn,
  buildThinkingByTurn,
  buildTimelineEventsByTurn,
  buildToolCallIdByTurn,
  buildToolStepsByTurn,
  buildTurnStateMaps,
  collectTurns,
  extractLastCompaction,
  toOpenAiMessage,
} from './session-reducer-helpers'
import type { AgentSessionEvent, AgentSessionMeta } from './session-types'


export interface SessionReplayStorePatch {
  session_id: string
  turn: number
  replayTurn: number
  active_chapter_path?: string
  activeSkill?: string
  messages: ChatUiMessage[]
  traces: ChatToolTrace[]
  llmMessages: OpenAiMessage[]
  lastStopReason?: AgentLoopStopReason
  lastCompaction?: AgentCompactionMeta
  turnOrder: number[]
  turnById: Record<number, AgentUiTurnState>
  answerByTurnId: Record<number, string>
  thinkingByTurnId: Record<number, string>
  stepsByTurnId: Record<number, AgentUiToolStep[]>
  eventsByTurnId: Record<number, AgentUiTimelineEvent[]>
  committedTimelineByTurnId: Record<number, TurnTimelineSnapshot>
  pendingAskUser?: AgentPendingAskUserRequest
  todoState: AgentTodoState
}


export function reduceSessionEventsToStore(input: {
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
  replayedAt?: number
}): SessionReplayStorePatch {
  const replay = replaySessionState({
    sessionId: input.sessionId,
    events: input.events,
    meta: input.meta,
  })

  const turnOrder = collectTurns(replay.messages, replay.traces, replay.turnStopReasonById)
  const { turnById } = buildTurnStateMaps({
    turnOrder,
    messages: replay.messages,
    turnStopReasonById: replay.turnStopReasonById,
  })
  const eventsByTurnId = buildTimelineEventsByTurn(input.events, input.replayedAt)
  const thinkingByTurnId = buildThinkingByTurn(eventsByTurnId)
  const lastCompaction = extractLastCompaction(input.events)
  const toolCallIdByTurn = buildToolCallIdByTurn(replay.messages, replay.traces)
  const usedToolCallIdCountByTurn: Record<number, number> = {}
  const replayLlmMessages = replay.messages
    .map((message) => toOpenAiMessage(message, {
      toolCallIdByTurn,
      usedToolCallIdCountByTurn,
    }))
    .filter((message): message is OpenAiMessage => Boolean(message))

  return {
    session_id: replay.sessionId,
    turn: replay.turn,
    replayTurn: replay.turn,
    active_chapter_path: replay.activeChapterPath,
    activeSkill: replay.activeSkill,
    messages: replay.messages,
    traces: replay.traces,
    llmMessages: replayLlmMessages,
    lastStopReason: replay.lastStopReason,
    lastCompaction,
    turnOrder,
    turnById,
    answerByTurnId: buildAnswersByTurn(replay.messages),
    thinkingByTurnId,
    stepsByTurnId: buildToolStepsByTurn(replay.traces),
    eventsByTurnId,
    committedTimelineByTurnId: {},
    pendingAskUser: replay.pendingAskUser,
    todoState: replay.todoState,
  }
}
