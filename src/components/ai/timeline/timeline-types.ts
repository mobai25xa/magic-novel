import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnPhase,
} from '@/lib/agent-chat/types'
import type {
  LoadingStage,
  TimelineBlock,
} from '@/lib/agent-chat/timeline'

export type TimelineAssistantBlock = Extract<TimelineBlock, { type: 'assistant_segment' }>

export type TimelineToolCallBlock = Extract<TimelineBlock, { type: 'tool_call' }>

export type TimelineThinkingPanelBlock = Extract<TimelineBlock, { type: 'thinking_panel' }>

export type ResolveTurnTimelineInput = {
  turn: number
  events: AgentUiTimelineEvent[]
  toolSteps: AgentUiToolStep[]
  answerText?: string
  thinkingText?: string
  running: boolean
  phase: AgentUiTurnPhase
  snapshot?: unknown
}

export type ResolvedTurnTimeline = {
  blocks: TimelineBlock[]
  stage: LoadingStage
}
