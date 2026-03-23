/**
 * A2. MacroChapterQueueList
 *
 * Per-chapter list with status badges. Click to expand detail (A3/A4).
 * Pure display — no invoke / state management.
 */

import { useState, useCallback } from 'react'
import { cn } from '@/lib/utils'
import { AiStatusBadge } from '../status-badge'
import type { ChapterRunState } from './types'
import { MacroChapterRunCard } from './MacroChapterRunCard'
import { MacroBlockedBanner } from './MacroBlockedBanner'

export type MacroChapterQueueListProps = {
  chapters: ChapterRunState[]
  currentIndex?: number
  onRetry?: (chapterRef: string) => void
  onSkip?: (chapterRef: string) => void
  onFix?: (chapterRef: string) => void
  className?: string
}

export function MacroChapterQueueList({
  chapters,
  currentIndex,
  onRetry,
  onSkip,
  onFix,
  className,
}: MacroChapterQueueListProps) {
  const [expandedRef, setExpandedRef] = useState<string | null>(null)

  const toggle = useCallback((ref: string) => {
    setExpandedRef((prev) => (prev === ref ? null : ref))
  }, [])

  if (chapters.length === 0) {
    return (
      <div className={cn('text-xs text-muted-foreground px-2 py-1', className)}>
        No chapters queued.
      </div>
    )
  }

  return (
    <div className={cn('flex flex-col gap-1', className)}>
      {chapters.map((ch, idx) => {
        const isExpanded = expandedRef === ch.chapter_ref
        const isCurrent = idx === currentIndex
        const label = ch.display_title || ch.chapter_ref

        return (
          <div key={ch.chapter_ref}>
            <button
              type="button"
              onClick={() => toggle(ch.chapter_ref)}
              className={cn(
                'flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-xs text-left transition-colors',
                'hover:bg-muted/40',
                isCurrent && 'bg-muted/30',
              )}
              aria-expanded={isExpanded}
            >
              <span className="shrink-0 w-5 text-center text-muted-foreground">{idx + 1}</span>
              <span className="min-w-0 truncate flex-1 font-medium">{label}</span>
              <AiStatusBadge status={ch.status} />
              {ch.stage ? (
                <span className="shrink-0 text-muted-foreground">{ch.stage}</span>
              ) : null}
            </button>

            {isExpanded ? (
              <div className="ml-5 mt-1 mb-1 flex flex-col gap-1">
                <MacroChapterRunCard chapter={ch} />
                {ch.status === 'blocked' ? (
                  <MacroBlockedBanner
                    lastError={ch.last_result_summary}
                    onRetry={onRetry ? () => onRetry(ch.chapter_ref) : undefined}
                    onSkip={onSkip ? () => onSkip(ch.chapter_ref) : undefined}
                    onFix={onFix ? () => onFix(ch.chapter_ref) : undefined}
                  />
                ) : null}
              </div>
            ) : null}
          </div>
        )
      })}
    </div>
  )
}
