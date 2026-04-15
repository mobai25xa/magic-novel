import type { AgentChatPanelViewProps } from '../panel/view/agent-chat-panel-view-types'
import { AgentChatPanelViewScrollJump } from '../panel/view/agent-chat-panel-view-scroll'
import { AgentChatTurnList } from '../panel/view/agent-chat-panel-view-turns'

type AgentChatPanelBodyProps = Pick<
  AgentChatPanelViewProps,
  | 'turnIds'
  | 'viewMode'
  | 'sessionId'
  | 'running'
  | 'onRetryStep'
  | 'sessionRuntimeState'
  | 'sessionCanResume'
  | 'sessionReadonlyReason'
> & {
  scrollRef: React.RefObject<HTMLDivElement | null>
  onScroll: (event: React.UIEvent<HTMLDivElement>) => void
  autoScrollLocked: boolean
  unseenCount: number
  onJumpToLatest: () => void
}

export function AgentChatPanelBody(props: AgentChatPanelBodyProps) {
  const {
    scrollRef,
    onScroll,
    autoScrollLocked,
    unseenCount,
    onJumpToLatest,
    turnIds,
    viewMode,
    sessionId,
    running,
    onRetryStep,
  } = props

  return (
    <div
      ref={scrollRef}
      className="editor-shell-ai-scroll"
      onScroll={onScroll}
    >
      <AgentChatTurnList
        turnIds={turnIds}
        viewMode={viewMode}
        sessionId={sessionId}
        running={running}
        sessionRuntimeState={props.sessionRuntimeState}
        sessionCanResume={props.sessionCanResume}
        sessionReadonlyReason={props.sessionReadonlyReason}
        onRetryStep={onRetryStep}
        scrollRef={scrollRef}
      />

      <AgentChatPanelViewScrollJump
        autoScrollLocked={autoScrollLocked}
        unseenCount={unseenCount}
        onJump={onJumpToLatest}
      />
    </div>
  )
}
