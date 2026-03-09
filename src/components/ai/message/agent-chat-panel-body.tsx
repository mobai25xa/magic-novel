import { useEffect, useMemo, useRef } from 'react'

import type { AgentChatPanelViewProps } from '../panel/view/agent-chat-panel-view-types'
import {
  createScrollHandler,
  jumpToLatest,
  type ChatScrollState,
} from '../panel/agent-chat-panel-scroll'
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
  scrollState: ChatScrollState
  setScrollState: React.Dispatch<React.SetStateAction<ChatScrollState>>
}

export function AgentChatPanelBody(props: AgentChatPanelBodyProps) {
  const {
    scrollRef,
    scrollState,
    setScrollState,
    turnIds,
    viewMode,
    sessionId,
    running,
    onRetryStep,
  } = props

  const handleScroll = useMemo(() => createScrollHandler(setScrollState), [setScrollState])

  // Ensure the scroll container is pinned to bottom while streaming and auto-scroll isn't locked.
  // Some virtualized/IO-rendered items can cause the last height change to land after our signature effect.
  const scrollRafRef = useRef<number | null>(null)
  const prevTurnCountRef = useRef<number>(turnIds.length)

  useEffect(() => {
    if (!running) {
      if (scrollRafRef.current !== null) {
        cancelAnimationFrame(scrollRafRef.current)
        scrollRafRef.current = null
      }
      prevTurnCountRef.current = turnIds.length
      return
    }

    const el = scrollRef.current
    if (!el || scrollState.autoScrollLocked) {
      prevTurnCountRef.current = turnIds.length
      return
    }

    // Only force pin when:
    // - streaming is active, and
    // - we are already near the bottom OR a new turn just appeared (so we don't fight user scroll).
    const nearBottom = el.scrollHeight - (el.scrollTop + el.clientHeight) <= 120
    const newTurnAppended = turnIds.length > prevTurnCountRef.current

    if (!nearBottom && !newTurnAppended) {
      prevTurnCountRef.current = turnIds.length
      return
    }

    if (scrollRafRef.current !== null) {
      cancelAnimationFrame(scrollRafRef.current)
      scrollRafRef.current = null
    }

    scrollRafRef.current = requestAnimationFrame(() => {
      el.scrollTo({ top: el.scrollHeight, behavior: 'auto' })
      scrollRafRef.current = null
    })

    prevTurnCountRef.current = turnIds.length

    return () => {
      if (scrollRafRef.current !== null) {
        cancelAnimationFrame(scrollRafRef.current)
        scrollRafRef.current = null
      }
    }
  }, [running, scrollRef, scrollState.autoScrollLocked, turnIds.length])

  return (
    <div
      ref={scrollRef}
      className="editor-shell-ai-scroll"
      onScroll={handleScroll}
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
        autoScrollLocked={scrollState.autoScrollLocked}
        unseenCount={scrollState.unseenCount}
        onJump={() => setScrollState(jumpToLatest(scrollRef))}
      />
    </div>
  )
}
