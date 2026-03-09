import { useMemo } from 'react'
import { format } from 'date-fns'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { Filespan } from '../../design/Filespan'
import { AiToolContent } from '@/magic-ui/components'
import { parseToolOutput } from '../tool-view-utils'

type CreateToolViewProps = {
  step: AgentUiToolStep
}

export function CreateToolView({ step }: CreateToolViewProps) {
  const ai = useAiTranslations()
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])

  const path = typeof parsed?.path === 'string' ? parsed.path : null
  const createdKind = typeof parsed?.created_kind === 'string' ? parsed.created_kind : null
  const title = typeof parsed?.title === 'string'
    ? parsed.title
    : (typeof parsed?.name === 'string' ? parsed.name : step.argsSummary || step.toolName)
  const createdAt = typeof parsed?.created_at === 'number'
    ? format(new Date(parsed.created_at), 'HH:mm')
    : typeof step.finishedAt === 'number'
      ? format(new Date(step.finishedAt), 'HH:mm')
      : null

  return (
    <AiToolContent className="space-y-1.5">
      <div className="text-xs font-medium">
        📖 {ai.toolView.createCreated}: {title}
      </div>

      {path ? (
        <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
          <span>{ai.toolView.createPath}:</span>
          <Filespan path={path} />
        </div>
      ) : null}

      {createdKind ? (
        <div className="text-[11px] text-muted-foreground">
          {ai.toolView.createKind}: {createdKind}
        </div>
      ) : null}

      {createdAt ? (
        <div className="text-[11px] text-muted-foreground">
          {ai.toolView.createTime}: {createdAt}
        </div>
      ) : null}
    </AiToolContent>
  )
}
