import { ArrowDown } from 'lucide-react'

import { AiScrollJumpShell } from '@/magic-ui/components'

import { buildJumpToLatestLabel } from '../../ai-copy-values'
import { useAiTranslations } from '../../ai-hooks'

type AgentChatPanelViewScrollJumpProps = {
  autoScrollLocked: boolean
  unseenCount: number
  onJump: () => void
}

export function AgentChatPanelViewScrollJump(input: AgentChatPanelViewScrollJumpProps) {
  const ai = useAiTranslations()

  if (!input.autoScrollLocked) {
    return null
  }

  return (
    <div className="ai-scroll-jump-wrap ai-animate-fly-in">
      <AiScrollJumpShell
        className="ai-scroll-jump-btn"
        onClick={input.onJump}
        aria-label={ai.panel.jumpToLatest}
      >
        <ArrowDown className="h-3.5 w-3.5" />
        {input.unseenCount > 0 ? (
          <span className="ai-scroll-jump-label is-badge ai-animate-scale-in">{buildJumpToLatestLabel(ai, input.unseenCount)}</span>
        ) : (
          <span className="ai-scroll-jump-label">{buildJumpToLatestLabel(ai, input.unseenCount)}</span>
        )}
      </AiScrollJumpShell>
    </div>
  )
}
