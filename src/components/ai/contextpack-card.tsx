import { useMemo, useState } from 'react'

import { Badge } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

export type ContextPackEvidenceSnippetV0 = {
  source_ref: string
  snippet: string
  reason: string
  score?: number
}

export type ContextPackCastNoteV0 = {
  character_ref: string
  summary: string
  voice_signals?: string[]
}

export type ContextPackV0 = {
  schema_version?: number
  scope_ref?: string
  token_budget?: 'small' | 'medium' | 'large' | string
  objective_summary?: string
  must_keep?: string[]
  active_constraints?: string[]
  key_facts?: string[]
  cast_notes?: ContextPackCastNoteV0[]
  evidence_snippets?: ContextPackEvidenceSnippetV0[]
  style_rules?: string[]
  review_targets?: string[]
  risk_flags?: string[]
  source_revisions?: Array<{ ref: string; revision: number }>
  generated_at?: number
}

export type ContextPackCardProps = {
  contextpack?: ContextPackV0 | null
  stale?: boolean
  topEvidenceCount?: number
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

function isBlank(text?: string) {
  return !String(text ?? '').trim()
}

function countArray<T>(value: T[] | undefined) {
  return Array.isArray(value) ? value.length : 0
}

function firstLine(text?: string) {
  const raw = String(text ?? '')
  const idx = raw.indexOf('\n')
  const line = (idx === -1 ? raw : raw.slice(0, idx)).trim()
  return line || '(empty snippet)'
}

export function ContextPackCard({
  contextpack,
  stale,
  topEvidenceCount = 4,
  className,
}: ContextPackCardProps) {
  const objective = contextpack?.objective_summary ?? ''
  const factsCount = countArray(contextpack?.key_facts)
  const constraintsCount = countArray(contextpack?.active_constraints)
  const evidenceCount = countArray(contextpack?.evidence_snippets)

  const empty = useMemo(() => {
    const hasObjective = !isBlank(objective)
    const hasFacts = factsCount > 0
    const hasConstraints = constraintsCount > 0
    const hasEvidence = evidenceCount > 0
    return !(hasObjective || hasFacts || hasConstraints || hasEvidence)
  }, [objective, factsCount, constraintsCount, evidenceCount])

  const missing = !contextpack
  const defaultOpen = missing || Boolean(stale) || empty
  const [open, setOpen] = useState(defaultOpen)

  const genTime = formatTime(contextpack?.generated_at)

  const topEvidence = (contextpack?.evidence_snippets ?? []).slice(0, topEvidenceCount)

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
              <span className="font-medium text-secondary-foreground">ContextPack</span>
              {contextpack?.token_budget ? (
                <Badge variant="soft" color="info" size="sm">{contextpack.token_budget}</Badge>
              ) : null}
              {stale ? (
                <Badge variant="soft" color="warning" size="sm">stale</Badge>
              ) : null}
              {missing ? (
                <Badge variant="soft" color="error" size="sm">missing</Badge>
              ) : null}
              {!missing && empty ? (
                <Badge variant="soft" color="warning" size="sm">empty</Badge>
              ) : null}
            </div>

            <div
              className="mt-0.5 text-muted-foreground truncate"
              title={missing ? 'Missing contextpacks/contextpack.json' : objective}
            >
              {missing
                ? 'Missing contextpack.json (not built yet)'
                : (objective || 'No objective summary')}
            </div>

            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
              <span>{`facts ${factsCount}`}</span>
              <span>{`constraints ${constraintsCount}`}</span>
              <span>{`evidence ${evidenceCount}`}</span>
            </div>
          </div>

          {genTime ? (
            <span className="shrink-0 text-[11px] text-muted-foreground" title={String(contextpack?.generated_at)}>
              {genTime}
            </span>
          ) : null}
        </div>
      </summary>

      <div className="mt-2 space-y-2">
        {contextpack?.scope_ref ? (
          <div className="font-mono text-[11px] text-muted-foreground break-all" title={contextpack.scope_ref}>
            {contextpack.scope_ref}
          </div>
        ) : null}

        {contextpack?.active_constraints && contextpack.active_constraints.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Active constraints</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.active_constraints.map((c, idx) => (
                <li key={idx} className="break-words">{c}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {contextpack?.key_facts && contextpack.key_facts.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Key facts</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.key_facts.map((f, idx) => (
                <li key={idx} className="break-words">{f}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {contextpack?.cast_notes && contextpack.cast_notes.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Cast notes</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.cast_notes.map((c, idx) => (
                <li key={idx} className="break-words">
                  <div className="flex items-baseline gap-2">
                    <span className="font-mono text-[11px] text-foreground/80" title={c.character_ref}>
                      {c.character_ref}
                    </span>
                    <span className="opacity-90">{c.summary}</span>
                  </div>
                  {c.voice_signals && c.voice_signals.length > 0 ? (
                    <div className="mt-0.5 text-[11px] opacity-70">
                      {`voice: ${c.voice_signals.join(', ')}`}
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {topEvidence.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Evidence (top)</div>
            <div className="mt-1 space-y-1">
              {topEvidence.map((e, idx) => (
                <details
                  key={`${e.source_ref}-${idx}`}
                  className="rounded-md border border-border/60 bg-background px-2 py-1"
                >
                  <summary className="cursor-pointer select-none">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="font-mono text-[11px] text-muted-foreground break-all" title={e.source_ref}>
                          {e.source_ref}
                        </div>
                        <div className="mt-0.5 text-muted-foreground truncate" title={e.reason}>
                          {e.reason}
                        </div>
                        <div className="mt-0.5 font-mono text-[11px] text-foreground/70 truncate" title={firstLine(e.snippet)}>
                          {firstLine(e.snippet)}
                        </div>
                      </div>

                      {typeof e.score === 'number' && Number.isFinite(e.score) ? (
                        <span className="shrink-0 text-[11px] text-muted-foreground" title={String(e.score)}>
                          {e.score.toFixed(2)}
                        </span>
                      ) : null}
                    </div>
                  </summary>

                  <pre className="mt-1 whitespace-pre-wrap font-mono text-[11px] text-muted-foreground">
                    {e.snippet}
                  </pre>
                </details>
              ))}
            </div>
          </div>
        ) : null}

        {contextpack?.style_rules && contextpack.style_rules.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Style rules</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.style_rules.map((r, idx) => (
                <li key={idx} className="break-words">{r}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {contextpack?.review_targets && contextpack.review_targets.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Review targets</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.review_targets.map((r, idx) => (
                <li key={idx} className="break-words">{r}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {contextpack?.risk_flags && contextpack.risk_flags.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Risk flags</div>
            <ul className="mt-1 space-y-1 text-muted-foreground">
              {contextpack.risk_flags.map((r, idx) => (
                <li key={idx} className="break-words">{r}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {contextpack?.source_revisions && contextpack.source_revisions.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Source revisions</div>
            <ul className="mt-1 space-y-1 font-mono text-[11px] text-muted-foreground">
              {contextpack.source_revisions.map((s, idx) => (
                <li key={idx} className="break-all">{`${s.ref} @ ${s.revision}`}</li>
              ))}
            </ul>
          </div>
        ) : null}
      </div>
    </details>
  )
}

export default ContextPackCard
