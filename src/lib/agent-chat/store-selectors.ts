import type { AgentChatStateSlice } from './store-helpers'
import type { AgentUiTurnView } from './types'

function sortToolSteps<T extends { startedAt: number; callId: string }>(steps: T[]) {
  return [...steps].sort((a, b) => {
    if (a.startedAt !== b.startedAt) {
      return a.startedAt - b.startedAt
    }
    return a.callId.localeCompare(b.callId)
  })
}

function getTurnUserText(state: AgentChatStateSlice, turn: number) {
  const userMessage = state.messages
    .filter((message) => message.role === 'user' && message.turn === turn)
    .at(-1)

  return userMessage?.content || ''
}

export function selectTurnViews(state: AgentChatStateSlice): AgentUiTurnView[] {
  return state.turnOrder
    .filter((turn) => Boolean(state.turnById[turn]))
    .map((turn) => ({
      state: state.turnById[turn],
      userText: getTurnUserText(state, turn),
      answerText: state.answerByTurnId[turn] || '',
      thinkingText: state.thinkingByTurnId[turn] || '',
      toolSteps: sortToolSteps(state.stepsByTurnId[turn] || []),
      events: state.eventsByTurnId[turn] || [],
    }))
}
