import type { AgentUiTurnView } from '@/lib/agent-chat/types'

import { resolveTurnTimeline } from './timeline/resolve-turn-timeline'

export type TurnContentBlock =
  | { id: string; type: 'assistant'; text: string }
  | { id: string; type: 'tools'; callIds: string[] }

export function resolveTurnContentBlocks(view: AgentUiTurnView): TurnContentBlock[] {
  const timeline = resolveTurnTimeline({
    turn: view.state.turn,
    events: view.events,
    toolSteps: view.toolSteps,
    answerText: view.answerText,
    thinkingText: view.thinkingText,
    running: false,
    phase: view.state.phase,
  })

  const blocks: TurnContentBlock[] = []

  for (const block of timeline.blocks) {
    if (block.type === 'assistant_segment') {
      blocks.push({ id: block.id, type: 'assistant', text: block.text })
      continue
    }

    if (block.type === 'tool_call') {
      blocks.push({ id: block.id, type: 'tools', callIds: [block.callId] })
    }
  }

  return blocks
}
