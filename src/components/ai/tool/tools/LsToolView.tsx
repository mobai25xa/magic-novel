import { useMemo } from 'react'
import { Library, FileText, Folder, File } from 'lucide-react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { AiToolContent, ShowMore } from '@/magic-ui/components'
import { parseToolOutput } from '../tool-view-utils'

type LsToolViewProps = {
  step: AgentUiToolStep
}

type LsEntry = {
  name: string
  kind?: string
  path?: string
  children?: LsEntry[]
  chapter_count?: number
}

function resolveEntryIconName(kind?: string) {
  if (kind === 'volume') return 'volume'
  if (kind === 'chapter') return 'chapter'
  if (kind === 'folder' || kind === 'directory') return 'folder'
  return 'file'
}

function EntryIcon({ name }: { name: ReturnType<typeof resolveEntryIconName> }) {
  if (name === 'volume') return <Library className="h-3 w-3 shrink-0" />
  if (name === 'chapter') return <FileText className="h-3 w-3 shrink-0" />
  if (name === 'folder') return <Folder className="h-3 w-3 shrink-0" />
  return <File className="h-3 w-3 shrink-0" />
}

function EntryRow({ entry, depth, isLast }: { entry: LsEntry; depth: number; isLast: boolean }) {
  const iconName = resolveEntryIconName(entry.kind)
  const prefix = depth === 0 ? '' : isLast ? '└─ ' : '├─ '
  const indent = depth > 0 ? '│  '.repeat(depth - 1) : ''
  const chapterInfo = entry.chapter_count != null ? ` (${entry.chapter_count} chapters)` : ''

  return (
    <>
      <div className="flex items-center gap-1 text-xs leading-5">
        <span className="text-muted-foreground/50 font-mono select-none whitespace-pre">
          {indent}{prefix}
        </span>
        <EntryIcon name={iconName} />
        <span className="truncate">{entry.name}</span>
        {chapterInfo ? (
          <span className="text-[11px] text-muted-foreground">{chapterInfo}</span>
        ) : null}
      </div>
      {entry.children?.map((child, i) => (
        <EntryRow
          key={child.path ?? child.name}
          entry={child}
          depth={depth + 1}
          isLast={i === (entry.children!.length - 1)}
        />
      ))}
    </>
  )
}

function parseLsEntries(parsed: Record<string, unknown> | null, rawOutput?: string): LsEntry[] {
  // Try structured output
  if (parsed) {
    const entries = parsed.entries ?? parsed.items ?? parsed.children ?? parsed.results
    if (Array.isArray(entries)) {
      return entries as LsEntry[]
    }
  }

  // Fallback: parse raw text as line list
  if (rawOutput) {
    return rawOutput.split('\n')
      .filter((line) => line.trim())
      .map((line) => ({ name: line.trim() }))
  }

  return []
}

export function LsToolView({ step }: LsToolViewProps) {
  const ai = useAiTranslations()
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const entries = useMemo(() => parseLsEntries(parsed, step.rawOutput), [parsed, step.rawOutput])

  if (entries.length === 0) {
    return (
      <AiToolContent className="text-xs text-muted-foreground">
        {ai.toolView.noOutput}
      </AiToolContent>
    )
  }

  return (
    <AiToolContent>
      <ShowMore maxLines={17}>
        <div className="space-y-0">
          {entries.map((entry, i) => (
            <EntryRow
              key={entry.path ?? entry.name}
              entry={entry}
              depth={0}
              isLast={i === entries.length - 1}
            />
          ))}
        </div>
      </ShowMore>
    </AiToolContent>
  )
}
