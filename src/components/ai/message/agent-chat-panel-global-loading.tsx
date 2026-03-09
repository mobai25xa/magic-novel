import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'
import { Spinner } from '@/magic-ui/components'

type LoadingStage = 'response' | 'thinking' | 'streaming'

type AgentChatPanelGlobalLoadingProps = {
  running: boolean
  stage?: LoadingStage
}

const STAGE_LABEL: Record<LoadingStage, string> = {
  response: 'response',
  thinking: 'thinking',
  streaming: 'streaming',
}

export function AgentChatPanelGlobalLoading({ running, stage = 'response' }: AgentChatPanelGlobalLoadingProps) {
  const ai = useAiTranslations()

  if (!running) {
    return null
  }

  return (
    <div className="flex items-center gap-1.5 pl-1.5 py-0.5 ai-animate-fly-in" aria-live="polite" aria-label={ai.panel.generating}>
      <Spinner size="xs" className="text-ai-status-running" />
      <span className={cn('text-[11px] text-muted-foreground select-none')}>{STAGE_LABEL[stage]}</span>
    </div>
  )
}
