import type { AiChatViewMode } from '@/state/settings'

import { VirtualMessageList } from '../../scroll/VirtualMessageList'

type AgentChatTurnListProps = {
  turnIds: number[]
  viewMode: AiChatViewMode
  sessionId: string
  running: boolean
  sessionRuntimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  onRetryStep: (turnId: number, callId: string) => void
  scrollRef: React.RefObject<HTMLDivElement | null>
}

export function AgentChatTurnList(input: AgentChatTurnListProps) {
  return (
    <VirtualMessageList
      turnIds={input.turnIds}
      viewMode={input.viewMode}
      sessionId={input.sessionId}
      running={input.running}
      sessionRuntimeState={input.sessionRuntimeState}
      sessionCanResume={input.sessionCanResume}
      sessionReadonlyReason={input.sessionReadonlyReason}
      onRetryStep={input.onRetryStep}
      scrollRef={input.scrollRef}
    />
  )
}
