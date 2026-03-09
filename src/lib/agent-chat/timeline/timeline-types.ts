export type LoadingStage = 'response' | 'thinking' | 'streaming'

export type TimelineBlock =
  | { id: string; type: 'assistant_segment'; seqStart: number; seqEnd: number; ts: number; text: string }
  | { id: string; type: 'tool_call'; seq: number; ts: number; callId: string }
  | { id: string; type: 'thinking_panel'; seqStart: number; seqEnd: number; ts: number; hasContent: boolean }

export type TurnTimelineSnapshot = {
  version: 2
  turn: number
  blocks: TimelineBlock[]
  stage?: LoadingStage
  createdAt: number
}
