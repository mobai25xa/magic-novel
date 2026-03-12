import type { LoadingStage } from '@/lib/agent-chat/timeline'
import type { AgentUiTurnPhase } from '@/lib/agent-chat/types'

import { PhaseTimeline } from './message/phase-timeline'

export type TurnExecutionTimelineProps = {
  phase: AgentUiTurnPhase
  stage: LoadingStage
  running: boolean
}

export function TurnExecutionTimeline({ phase, stage, running }: TurnExecutionTimelineProps) {
  return (
    <PhaseTimeline
      phase={phase}
      stage={stage}
      running={running}
    />
  )
}
