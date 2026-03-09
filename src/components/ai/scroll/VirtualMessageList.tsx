import { useCallback, useState, type RefObject } from 'react'

import type { AiChatViewMode } from '@/state/settings'

import { AgentChatTurnItem } from '../panel/agent-chat-turn-item'
import { useAiTranslations } from '../ai-hooks'
import { IntersectionDisplay } from './IntersectionDisplay'

type VirtualMessageListProps = {
  turnIds: number[]
  viewMode: AiChatViewMode
  sessionId: string
  running: boolean
  sessionRuntimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  onRetryStep: (turnId: number, callId: string) => void
  scrollRef: RefObject<HTMLDivElement | null>
}

const DEFAULT_ESTIMATED_HEIGHT = 200
const COMPACT_ESTIMATED_HEIGHT = 80

function getEstimatedHeight(
  turnKey: string,
  heightByTurnKey: Record<string, number>,
  viewMode: AiChatViewMode,
): number {
  const cached = heightByTurnKey[turnKey]
  if (cached) return cached
  return viewMode === 'compact' ? COMPACT_ESTIMATED_HEIGHT : DEFAULT_ESTIMATED_HEIGHT
}

export function VirtualMessageList(input: VirtualMessageListProps) {
  const ai = useAiTranslations()
  const [heightByTurnKey, setHeightByTurnKey] = useState<Record<string, number>>({})

  const handleHeightChange = useCallback((id: string, height: number) => {
    setHeightByTurnKey((prev) => {
      if (prev[id] === height) {
        return prev
      }
      return {
        ...prev,
        [id]: height,
      }
    })
  }, [])

  if (input.turnIds.length === 0) {
    return (
      <div className="text-xs text-muted-foreground">
        {ai.panel.emptyHint}
      </div>
    )
  }

  return input.turnIds.map((turnId) => {
    const turnKey = `turn-${turnId}`

    return (
      <IntersectionDisplay
        key={turnId}
        id={turnKey}
        estimatedHeight={getEstimatedHeight(turnKey, heightByTurnKey, input.viewMode)}
        onHeightChange={handleHeightChange}
        root={input.scrollRef}
      >
        <AgentChatTurnItem
          turnId={turnId}
          viewMode={input.viewMode}
          sessionId={input.sessionId}
          running={input.running}
          sessionRuntimeState={input.sessionRuntimeState}
          sessionCanResume={input.sessionCanResume}
          sessionReadonlyReason={input.sessionReadonlyReason}
          onRetryStep={input.onRetryStep}
        />
      </IntersectionDisplay>
    )
  })
}
