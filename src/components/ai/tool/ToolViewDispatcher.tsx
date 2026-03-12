import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { ReadToolView } from './tools/ReadToolView'
import { EditToolView } from './tools/EditToolView'
import { CreateToolView } from './tools/CreateToolView'
import { LsToolView } from './tools/LsToolView'
import { GrepToolView } from './tools/GrepToolView'
import { ReviewToolView } from './tools/ReviewToolView'
import { GenericToolView } from './tools/GenericToolView'

type ToolViewDispatcherProps = {
  step: AgentUiToolStep
  sessionId: string
  turnId: number
}

export function ToolViewDispatcher({ step }: ToolViewDispatcherProps) {
  switch (step.toolName) {
    case 'read':
      return <ReadToolView step={step} />
    case 'edit':
      return <EditToolView step={step} />
    case 'create':
      return <CreateToolView step={step} />
    case 'delete':
      return <GenericToolView step={step} />
    case 'move':
      return <GenericToolView step={step} />
    case 'ls':
      return <LsToolView step={step} />
    case 'grep':
      return <GrepToolView step={step} />
    case 'review_check':
      return <ReviewToolView step={step} />
    case 'todowrite':
      return <GenericToolView step={step} />
    default:
      return <GenericToolView step={step} />
  }
}
