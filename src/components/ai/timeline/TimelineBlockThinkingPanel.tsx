import type { TimelineBlock } from '@/lib/agent-chat/timeline'

import { ThinkingBlock } from '../message/thinking-block'

type TimelineBlockThinkingPanelProps = {
  block: Extract<TimelineBlock, { type: 'thinking_panel' }>
  running: boolean
}

export function TimelineBlockThinkingPanel(input: TimelineBlockThinkingPanelProps) {
  if (!input.block.hasContent) {
    return null
  }

  return (
    <ThinkingBlock
      text="placeholder"
      streaming={input.running}
      defaultOpen={false}
    />
  )
}
