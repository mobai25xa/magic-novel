/**
 * A7. MacroWritePathsList (P1)
 *
 * Displays chapter write_paths / targets as a copyable path list.
 * Pure display — no invoke / state management.
 */

import { useCallback } from 'react'
import { cn } from '@/lib/utils'

export type MacroWritePathsListProps = {
  paths: Array<{ chapterRef: string; writePath: string; displayTitle?: string }>
  className?: string
}

function CopyablePathRow({
  index,
  label,
  path,
}: {
  index: number
  label: string
  path: string
}) {
  const copy = useCallback(() => {
    navigator.clipboard.writeText(path).catch(() => {})
  }, [path])

  return (
    <li className="flex items-center gap-2 py-0.5">
      <span className="shrink-0 w-5 text-center text-muted-foreground">{index + 1}</span>
      <span className="shrink-0 font-medium truncate max-w-[120px]">{label}</span>
      <button
        type="button"
        onClick={copy}
        title="Copy path"
        className="min-w-0 truncate text-muted-foreground hover:text-foreground hover:underline cursor-pointer font-mono"
      >
        {path}
      </button>
    </li>
  )
}

export function MacroWritePathsList({ paths, className }: MacroWritePathsListProps) {
  if (paths.length === 0) return null

  const copyAll = () => {
    const text = paths.map((p) => p.writePath).join('\n')
    navigator.clipboard.writeText(text).catch(() => {})
  }

  return (
    <div
      className={cn(
        'flex flex-col gap-1 rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs',
        className,
      )}
    >
      <div className="flex items-center justify-between">
        <span className="font-medium">Write Targets</span>
        <button
          type="button"
          onClick={copyAll}
          className="text-muted-foreground hover:text-foreground hover:underline cursor-pointer"
        >
          Copy all
        </button>
      </div>
      <ul className="flex flex-col">
        {paths.map((p, idx) => (
          <CopyablePathRow
            key={p.chapterRef}
            index={idx}
            label={p.displayTitle || p.chapterRef}
            path={p.writePath}
          />
        ))}
      </ul>
    </div>
  )
}
