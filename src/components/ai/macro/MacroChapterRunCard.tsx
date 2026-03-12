/**
 * A3. MacroChapterRunCard
 *
 * Chapter detail card: write_path, review_id, knowledge_delta_id, handoff summary.
 * Collapsed by default — avoids "log wall".
 * Pure display — no invoke / state management.
 */

import { useCallback } from 'react'
import { cn } from '@/lib/utils'
import type { ChapterRunState } from './types'

export type MacroChapterRunCardProps = {
  chapter: ChapterRunState
  className?: string
}

function CopyableField({ label, value }: { label: string; value: string }) {
  const copy = useCallback(() => {
    navigator.clipboard.writeText(value).catch(() => {})
  }, [value])

  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-muted-foreground">{label}</span>
      <button
        type="button"
        onClick={copy}
        title="Copy"
        className="min-w-0 truncate text-right hover:underline cursor-pointer"
      >
        {value}
      </button>
    </div>
  )
}

export function MacroChapterRunCard({ chapter, className }: MacroChapterRunCardProps) {
  const hasDetails =
    chapter.write_path ||
    chapter.latest_review_id ||
    chapter.latest_knowledge_delta_id ||
    chapter.last_handoff_summary

  if (!hasDetails) {
    return (
      <div className={cn('text-xs text-muted-foreground px-2 py-1', className)}>
        No details yet.
      </div>
    )
  }

  return (
    <div
      className={cn(
        'flex flex-col gap-1 rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs',
        className,
      )}
    >
      {chapter.write_path ? (
        <CopyableField label="write_path" value={chapter.write_path} />
      ) : null}

      {chapter.latest_review_id ? (
        <CopyableField label="review_id" value={chapter.latest_review_id} />
      ) : null}

      {chapter.latest_knowledge_delta_id ? (
        <CopyableField label="knowledge_delta" value={chapter.latest_knowledge_delta_id} />
      ) : null}

      {chapter.last_handoff_summary ? (
        <div className="mt-1 border-t border-border/40 pt-1">
          <span className="text-muted-foreground">handoff</span>
          <p className="mt-0.5 whitespace-pre-wrap break-words">{chapter.last_handoff_summary}</p>
        </div>
      ) : null}
    </div>
  )
}
