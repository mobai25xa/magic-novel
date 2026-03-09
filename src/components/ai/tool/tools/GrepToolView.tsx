import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { AiToolContent, Badge, ShowMore } from '@/magic-ui/components'
import { Filespan } from '../../design/Filespan'
import { parseToolOutput } from '../tool-view-utils'

type GrepToolViewProps = {
  step: AgentUiToolStep
}

type GrepResult = {
  path?: string
  score?: number
  snippet?: string
  line?: number
}

function parseGrepResults(
  parsed: Record<string, unknown> | null,
  rawOutput?: string,
): { query: string; results: GrepResult[] } {
  const query = typeof parsed?.query === 'string'
    ? parsed.query
    : typeof parsed?.pattern === 'string'
      ? parsed.pattern
      : ''

  if (parsed) {
    const results = parsed.results ?? parsed.matches ?? parsed.items
    if (Array.isArray(results)) {
      return { query, results: results as GrepResult[] }
    }
  }

  // Fallback: parse raw text
  if (rawOutput) {
    const lines = rawOutput.split('\n').filter((l) => l.trim())
    const results = lines.map((line) => ({ snippet: line } as GrepResult))
    return { query, results }
  }

  return { query, results: [] }
}

function highlightKeyword(text: string, keyword: string): React.ReactNode {
  if (!keyword) return text

  const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const regex = new RegExp(`(${escaped})`, 'gi')
  const parts = text.split(regex)

  return parts.map((part, i) =>
    regex.test(part)
      ? <mark key={i} className="bg-ai-waiting-30 text-foreground rounded-sm px-0.5">{part}</mark>
      : part,
  )
}

export function GrepToolView({ step }: GrepToolViewProps) {
  const ai = useAiTranslations()
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const { query, results } = useMemo(() => parseGrepResults(parsed, step.rawOutput), [parsed, step.rawOutput])

  if (results.length === 0) {
    return (
      <AiToolContent className="text-xs text-muted-foreground">
        {ai.toolView.noOutput}
      </AiToolContent>
    )
  }

  return (
    <AiToolContent className="space-y-2">
      <div className="text-xs text-secondary-foreground">
        🔍 {query ? `"${query}" — ` : ''}{ai.toolView.grepFound} {results.length} {ai.toolView.grepResults}
      </div>

      <ShowMore maxLines={14}>
        <div className="space-y-2">
          {results.map((result, i) => (
            <div key={i} className="space-y-0.5">
              <div className="flex items-center gap-2">
                {result.path ? (
                  <Filespan path={result.path} />
                ) : null}
                {result.score != null ? (
                  <Badge color="info" variant="soft" size="sm">
                    {ai.toolView.grepScore}: {result.score.toFixed(2)}
                  </Badge>
                ) : null}
              </div>
              {result.snippet ? (
                <div className="text-xs text-secondary-foreground pl-1 truncate">
                  {highlightKeyword(result.snippet, query)}
                </div>
              ) : null}
            </div>
          ))}
        </div>
      </ShowMore>
    </AiToolContent>
  )
}
