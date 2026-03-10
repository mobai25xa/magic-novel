import { useMemo, useState } from 'react'

import { Badge, Button } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

import { ChapterCardEditor, type ChapterCardDraft } from './chapter-card-editor'
import { AiStatusBadge } from './status-badge'

export type ChapterCardV0 = {
  schema_version?: number
  scope_ref?: string
  scope_locator?: string
  objective?: string
  workflow_kind?: 'micro' | 'chapter' | 'arc' | 'book' | string
  hard_constraints?: string[]
  success_criteria?: string[]
  status?: 'draft' | 'active' | 'blocked' | 'completed' | string
  updated_at?: number
}

export type RecentFactsV0 = {
  schema_version?: number
  scope_ref?: string
  facts?: Array<{
    summary: string
    source_ref?: string
    confidence?: 'accepted' | 'proposed' | string
  }>
  updated_at?: number
}

export type ActiveCastV0 = {
  schema_version?: number
  scope_ref?: string
  cast?: Array<{
    character_ref: string
    current_state_summary: string
    must_keep_voice_signals?: string[]
  }>
  updated_at?: number
}

export type Layer1ArtifactsCardProps = {
  chapter_card?: ChapterCardV0 | null
  recent_facts?: RecentFactsV0 | null
  active_cast?: ActiveCastV0 | null
  stale?: boolean
  onSaveChapterCard?: (draft: ChapterCardDraft) => void | Promise<void>
  onCreateDefaultChapterCard?: () => void
  onInferScopeFromCurrentChapter?: () => void
  onBuildContextPack?: () => void
  className?: string
}

function formatTime(ts?: number) {
  if (!ts || !Number.isFinite(ts)) {
    return null
  }
  try {
    return new Date(ts).toLocaleTimeString()
  } catch {
    return null
  }
}

function maxTs(values: Array<number | undefined>) {
  let best = 0
  for (const v of values) {
    if (!v || !Number.isFinite(v)) continue
    if (v > best) best = v
  }
  return best || undefined
}

function isBlank(text?: string) {
  return !String(text ?? '').trim()
}

function countArray<T>(value: T[] | undefined) {
  return Array.isArray(value) ? value.length : 0
}

export function Layer1ArtifactsCard({
  chapter_card,
  recent_facts,
  active_cast,
  stale,
  onSaveChapterCard,
  onCreateDefaultChapterCard,
  onInferScopeFromCurrentChapter,
  onBuildContextPack,
  className,
}: Layer1ArtifactsCardProps) {
  const factsCount = countArray(recent_facts?.facts)
  const castCount = countArray(active_cast?.cast)

  const missingChapterCard = !chapter_card
  const missingRecentFacts = !recent_facts
  const missingActiveCast = !active_cast
  const missingAny = missingChapterCard || missingRecentFacts || missingActiveCast

  const objective = chapter_card?.objective ?? ''
  const constraintsCount = countArray(chapter_card?.hard_constraints)
  const successCount = countArray(chapter_card?.success_criteria)

  const empty = useMemo(() => {
    const hasObjective = !isBlank(objective)
    const hasConstraints = constraintsCount > 0
    const hasSuccess = successCount > 0
    const hasFacts = factsCount > 0
    const hasCast = castCount > 0
    return !(hasObjective || hasConstraints || hasSuccess || hasFacts || hasCast)
  }, [objective, constraintsCount, successCount, factsCount, castCount])

  const updatedAt = maxTs([
    chapter_card?.updated_at,
    recent_facts?.updated_at,
    active_cast?.updated_at,
  ])
  const time = formatTime(updatedAt)

  const defaultOpen = missingAny || Boolean(stale) || empty
  const [open, setOpen] = useState(defaultOpen)
  const [editingChapterCard, setEditingChapterCard] = useState(false)

  return (
    <details
      className={cn(
        'rounded-md border border-border/60 bg-background-50 px-2.5 py-2 text-xs',
        className,
      )}
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium text-secondary-foreground">Layer1</span>
              {chapter_card?.status ? (
                <AiStatusBadge status={chapter_card.status} size="sm" />
              ) : null}
              {stale ? (
                <Badge variant="soft" color="warning" size="sm">stale</Badge>
              ) : null}
              {missingAny ? (
                <Badge variant="soft" color="error" size="sm">missing</Badge>
              ) : null}
              {!missingAny && empty ? (
                <Badge variant="soft" color="warning" size="sm">empty</Badge>
              ) : null}
            </div>

            <div
              className="mt-0.5 text-muted-foreground truncate"
              title={missingChapterCard ? 'Missing chapter_card.json' : objective}
            >
              {missingChapterCard
                ? 'Missing chapter_card.json (objective/constraints/success criteria)'
                : (objective || 'No objective set')}
            </div>

            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
              <span>{`facts ${missingRecentFacts ? '—' : factsCount}`}</span>
              <span>{`cast ${missingActiveCast ? '—' : castCount}`}</span>
              <span>{`constraints ${constraintsCount}`}</span>
              <span>{`success ${successCount}`}</span>
            </div>
          </div>

          {time ? (
            <span className="shrink-0 text-[11px] text-muted-foreground" title={String(updatedAt)}>
              {time}
            </span>
          ) : null}
        </div>
      </summary>

      <div className="mt-2 space-y-2">
        {missingAny || empty ? (
          <div className="rounded-md border border-border/60 bg-background px-2.5 py-2">
            <div className="text-[11px] font-medium text-secondary-foreground">Next steps</div>
            <ul className="mt-1 space-y-1 text-muted-foreground list-disc ml-4">
              {empty && !missingAny ? (
                <li>Fill objective/constraints/success criteria so the task is well-scoped.</li>
              ) : null}
              {missingChapterCard ? (
                <li>Create `chapter_card.json` (objective/constraints/success criteria).</li>
              ) : null}
              {missingRecentFacts ? (
                <li>Generate `recent_facts.json` to keep short-term facts consistent.</li>
              ) : null}
              {missingActiveCast ? (
                <li>Generate `active_cast.json` so voice/state stays stable.</li>
              ) : null}
              <li>After Layer1 is ready, build/refresh ContextPack for minimal injection.</li>
            </ul>

            <div className="mt-2 flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="outline"
                className="text-xs"
                onClick={() => setEditingChapterCard(true)}
              >
                {chapter_card ? 'Edit chapter card' : 'Draft chapter card'}
              </Button>

              {onCreateDefaultChapterCard ? (
                <Button
                  size="sm"
                  variant="outline"
                  className="text-xs"
                  onClick={onCreateDefaultChapterCard}
                >
                  Create default chapter card
                </Button>
              ) : null}

              {onInferScopeFromCurrentChapter ? (
                <Button
                  size="sm"
                  variant="outline"
                  className="text-xs"
                  onClick={onInferScopeFromCurrentChapter}
                >
                  Infer scope
                </Button>
              ) : null}

              {onBuildContextPack ? (
                <Button
                  size="sm"
                  variant="outline"
                  className="text-xs"
                  onClick={onBuildContextPack}
                >
                  Build ContextPack
                </Button>
              ) : null}
            </div>
          </div>
        ) : null}

        <div>
          <div className="flex items-center justify-between gap-2">
            <div className="text-[11px] font-medium text-secondary-foreground">Chapter card</div>
            <Button
              size="sm"
              variant="outline"
              className="text-xs"
              onClick={() => setEditingChapterCard((prev) => !prev)}
            >
              {editingChapterCard
                ? 'Close editor'
                : (chapter_card ? 'Edit' : 'Draft')}
            </Button>
          </div>
          {chapter_card ? (
            <div className="mt-1 space-y-1 text-muted-foreground">
              {chapter_card.workflow_kind ? (
                <div>{`workflow: ${chapter_card.workflow_kind}`}</div>
              ) : null}
              {chapter_card.scope_ref ? (
                <div className="font-mono text-[11px] break-all" title={chapter_card.scope_ref}>
                  {chapter_card.scope_ref}
                </div>
              ) : null}
            </div>
          ) : (
            <div className="mt-1 text-muted-foreground">
              No chapter card loaded.
            </div>
          )}
        </div>

        {editingChapterCard ? (
          <ChapterCardEditor
            initial={chapter_card ?? undefined}
            onCancel={() => setEditingChapterCard(false)}
            onSaved={() => setEditingChapterCard(false)}
            onSave={onSaveChapterCard}
          />
        ) : null}

        {chapter_card?.hard_constraints && chapter_card.hard_constraints.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Constraints</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {chapter_card.hard_constraints.map((c, idx) => (
                <li key={idx} className="break-words">{c}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {chapter_card?.success_criteria && chapter_card.success_criteria.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Success criteria</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {chapter_card.success_criteria.map((c, idx) => (
                <li key={idx} className="break-words">{c}</li>
              ))}
            </ul>
          </div>
        ) : null}

        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Recent facts</div>
          {recent_facts ? (
            factsCount > 0 ? (
              <ul className="mt-1 space-y-1 text-muted-foreground">
                {recent_facts.facts!.map((f, idx) => (
                  <li key={idx} className="break-words">
                    <span className="opacity-90">{f.summary}</span>
                    {f.confidence ? (
                      <span className="ml-2 opacity-60">[{f.confidence}]</span>
                    ) : null}
                    {f.source_ref ? (
                      <span className="ml-2 font-mono text-[11px] opacity-70" title={f.source_ref}>
                        {f.source_ref}
                      </span>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : (
              <div className="mt-1 text-muted-foreground">No recent facts.</div>
            )
          ) : (
            <div className="mt-1 text-muted-foreground">Missing recent_facts.json</div>
          )}
        </div>

        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Active cast</div>
          {active_cast ? (
            castCount > 0 ? (
              <ul className="mt-1 space-y-1 text-muted-foreground">
                {active_cast.cast!.map((c, idx) => (
                  <li key={idx} className="break-words">
                    <div className="flex items-baseline gap-2">
                      <span className="font-mono text-[11px] text-foreground/80" title={c.character_ref}>
                        {c.character_ref}
                      </span>
                      <span className="opacity-90">{c.current_state_summary}</span>
                    </div>
                    {c.must_keep_voice_signals && c.must_keep_voice_signals.length > 0 ? (
                      <div className="mt-0.5 text-[11px] opacity-70">
                        {`voice: ${c.must_keep_voice_signals.join(', ')}`}
                      </div>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : (
              <div className="mt-1 text-muted-foreground">No active cast.</div>
            )
          ) : (
            <div className="mt-1 text-muted-foreground">Missing active_cast.json</div>
          )}
        </div>
      </div>
    </details>
  )
}

export default Layer1ArtifactsCard
