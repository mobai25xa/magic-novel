import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { AiToolContent, Badge, CodeBlock, ShowMore } from '@/magic-ui/components'
import { Filespan } from '../../design/Filespan'
import { resolveStepPath, parseToolOutput, isDryRun } from '../tool-view-utils'

type EditToolViewProps = {
  step: AgentUiToolStep
}

function extractDiffLines(rawOutput?: string): string[] {
  if (!rawOutput) return []
  const lines = rawOutput.split('\n')
  return lines.filter((line) => line.startsWith('+') || line.startsWith('-'))
}

function DiffLine({ line }: { line: string }) {
  const isAdd = line.startsWith('+')
  return (
    <div
      className={
        isAdd
          ? 'bg-ai-success-10 text-ai-status-success'
          : 'bg-danger-10 text-destructive'
      }
    >
      <span className="select-none inline-block w-4 text-center opacity-60">
        {isAdd ? '+' : '-'}
      </span>
      {line.slice(1)}
    </div>
  )
}

export function EditToolView({ step }: EditToolViewProps) {
  const ai = useAiTranslations()
  const path = resolveStepPath(step)
  const dryRun = isDryRun(step)
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const diffLines = useMemo(() => extractDiffLines(step.rawOutput), [step.rawOutput])

  const plus = diffLines.filter((l) => l.startsWith('+')).length
  const minus = diffLines.filter((l) => l.startsWith('-')).length

  const revisionBefore = step.revisionBefore
  const revisionAfter = step.revisionAfter

  return (
    <AiToolContent className="space-y-2">
      <div className="flex items-center gap-2 flex-wrap">
        {path ? <Filespan path={path} /> : null}
        <Badge
          color={dryRun ? 'info' : 'success'}
          variant="soft"
          size="sm"
        >
          {dryRun ? ai.toolView.editPreview : ai.toolView.editCommitted}
        </Badge>
      </div>

      {dryRun && parsed?.diagnostics_passed !== false ? (
        <div className="text-xs text-ai-status-success">✅ {ai.toolView.editDiagnostics}</div>
      ) : null}

      {(plus > 0 || minus > 0) ? (
        <div className="flex items-center gap-2">
          {plus > 0 ? (
            <Badge color="success" variant="soft" size="sm">+{plus}{ai.toolView.readWordCount}</Badge>
          ) : null}
          {minus > 0 ? (
            <Badge color="error" variant="soft" size="sm">-{minus}{ai.toolView.readWordCount}</Badge>
          ) : null}
        </div>
      ) : null}

      {diffLines.length > 0 ? (
        <ShowMore maxLines={11}>
          <CodeBlock className="rounded border bg-background p-2 text-[11px] font-mono overflow-auto whitespace-pre-wrap break-words">
            {diffLines.map((line, i) => (
              <DiffLine key={i} line={line} />
            ))}
          </CodeBlock>
        </ShowMore>
      ) : null}

      {!dryRun && revisionBefore != null && revisionAfter != null ? (
        <div className="text-[11px] text-muted-foreground">
          {ai.toolView.readRevision}: {revisionBefore} → {revisionAfter}
        </div>
      ) : null}
    </AiToolContent>
  )
}
