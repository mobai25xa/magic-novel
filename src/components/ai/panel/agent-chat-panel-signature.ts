import { useMemo } from 'react'

import { useAgentChatStore } from '@/state/agent'

export function useLatestTurnSignature() {
  const lastTurnId = useAgentChatStore((state) => state.turnOrder[state.turnOrder.length - 1])
  const turnState = useAgentChatStore((state) => (
    typeof lastTurnId === 'number' ? state.turnById[lastTurnId] : undefined
  ))
  const answerLength = useAgentChatStore((state) => (
    typeof lastTurnId === 'number' ? (state.answerByTurnId[lastTurnId] || '').length : 0
  ))
  const thinkingLength = useAgentChatStore((state) => (
    typeof lastTurnId === 'number' ? (state.thinkingByTurnId[lastTurnId] || '').length : 0
  ))

  return useMemo(() => {
    if (typeof lastTurnId !== 'number' || !turnState) {
      return ''
    }
    return `${turnState.turn}:${turnState.updatedAt}:${answerLength}:${thinkingLength}`
  }, [answerLength, lastTurnId, thinkingLength, turnState])
}
