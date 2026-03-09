import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { Filespan } from '../../design/Filespan'
import { AiToolContent, CodeBlock, ShowMore } from '@/magic-ui/components'
import { resolveStepPath, parseToolOutput } from '../tool-view-utils'

type ReadToolViewProps = {
  step: AgentUiToolStep
}

export function ReadToolView({ step }: ReadToolViewProps) {
  const ai = useAiTranslations()
  const path = resolveStepPath(step)

  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const content = useMemo(() => {
    if (typeof parsed?.content === 'string') return parsed.content
    if (typeof step.rawOutput === 'string' && !parsed) return step.rawOutput
    return ''
  }, [parsed, step.rawOutput])

  const revision = parsed?.revision as string | undefined
  const metadata = parsed?.metadata as Record<string, unknown> | undefined
  const wordCount = typeof metadata?.word_count === 'number'
    ? metadata.word_count
    : content.length

  return (
    <AiToolContent className="space-y-2.5">
      {path ? <Filespan path={path} /> : null}

      {content ? (
        <ShowMore maxLines={11}>
          <CodeBlock className="text-xs text-foreground whitespace-pre-wrap break-words leading-relaxed">
            {content}
          </CodeBlock>
        </ShowMore>
      ) : (
        <div className="text-xs text-muted-foreground">{ai.toolView.noOutput}</div>
      )}

      <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
        {revision ? (
          <span>{ai.toolView.readRevision}: {revision}</span>
        ) : null}
        <span>{wordCount.toLocaleString()} {ai.toolView.readWordCount}</span>
      </div>
    </AiToolContent>
  )
}
