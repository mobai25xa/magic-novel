import { useMemo, useState } from 'react'

import { reportUiCounter } from '../panel/agent-chat-panel-metrics'
import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { AiCodePreview } from '@/magic-ui/components'
import { buildToolDiffSummaryLabel } from '../ai-copy-values'
import { useAiTranslations } from '../ai-hooks'

type ToolStepDiffPreviewProps = {
  step: AgentUiToolStep
  sessionId: string
  turnId?: number
}

function extractDiffSummary(rawOutput?: string) {
  if (!rawOutput) return []

  const lines = rawOutput.split('\n')
  const diffLines = lines.filter((line) => line.startsWith('+') || line.startsWith('-'))
  if (!diffLines.length) {
    return []
  }

  return diffLines.map((line) => ({
    raw: line,
    type: line.startsWith('+') ? 'added' : 'removed',
    text: line.slice(1),
  }))
}

export function ToolStepDiffPreview(input: ToolStepDiffPreviewProps) {
  const ai = useAiTranslations()
  const [expanded, setExpanded] = useState(false)
  const diffLines = useMemo(() => extractDiffSummary(input.step.rawOutput), [input.step.rawOutput])

  if (input.step.toolName !== 'edit' || !diffLines.length) {
    return null
  }

  const plus = diffLines.filter((line) => line.type === 'added').length
  const minus = diffLines.filter((line) => line.type === 'removed').length
  const visibleLines = expanded ? diffLines : diffLines.slice(0, 12)

  return (
    <div className="ai-tool-diff">
      <div className="ai-tool-diff-summary">{buildToolDiffSummaryLabel(ai, plus, minus)}</div>
      <AiCodePreview className="ai-tool-diff-preview max-h-40 overflow-auto" aria-label={buildToolDiffSummaryLabel(ai, plus, minus)}>
        {visibleLines.map((line, index) => (
          <span key={`${line.raw}-${index}`} className={`ai-tool-diff-line ${line.type === 'added' ? 'is-added' : 'is-removed'}`}>
            <span className="ai-tool-diff-gutter" aria-hidden="true">{line.type === 'added' ? '+' : '-'}</span>
            <span>{line.text}</span>
          </span>
        ))}
      </AiCodePreview>
      {diffLines.length > 12 ? (
        <button
          type="button"
          className="ai-tool-diff-toggle"
          onClick={() => {
            const next = !expanded
            setExpanded(next)
            if (next) {
              reportUiCounter({
                sessionId: input.sessionId,
                turnId: input.turnId,
                metric: 'inline_diff_open_rate',
                tags: {
                  callId: input.step.callId,
                },
              })
            }
          }}
        >
          {expanded ? ai.action.collapseDiff : ai.action.expandDiff}
        </button>
      ) : null}
    </div>
  )
}
