import { useMemo, useState } from 'react'

import { cn } from '@/lib/utils'

import type { KnowledgeDeltaLike } from './knowledge-apply-status-card'
import { KnowledgeApplyStatusCard } from './knowledge-apply-status-card'
import { KnowledgeConflictList, type KnowledgeConflictLike } from './knowledge-conflict-list'
import { KnowledgeProposalBundleCard, type KnowledgeProposalBundleLike } from './knowledge-proposal-bundle-card'
import { KnowledgeSummaryRow, type KnowledgeSummaryStatus } from './knowledge-summary-row'

export type KnowledgeHistoryEntryLike = {
  bundle?: KnowledgeProposalBundleLike | null
  delta?: KnowledgeDeltaLike | null
}

export type KnowledgeHistorySectionProps = {
  entries: KnowledgeHistoryEntryLike[]
  maxItems?: number
  className?: string
}

function resolveSummary(input: KnowledgeHistoryEntryLike): {
  status: KnowledgeSummaryStatus
  items?: number
  conflicts?: number
  accepted?: number
  rejected?: number
  generatedAt?: number | string
  appliedAt?: number | string
} {
  const bundle = input.bundle ?? null
  const delta = input.delta ?? null

  const items = bundle?.proposal_items?.length
  const conflicts = Array.isArray(delta?.conflicts) ? delta!.conflicts.length : 0
  const accepted = Array.isArray(delta?.accepted_item_ids) ? delta!.accepted_item_ids.length : undefined
  const rejected = Array.isArray(delta?.rejected_item_ids) ? delta!.rejected_item_ids.length : undefined

  const generatedAt = delta?.generated_at ?? bundle?.generated_at
  const appliedAt = delta?.applied_at

  const deltaStatus = String(delta?.status ?? '').trim().toLowerCase()

  let status: KnowledgeSummaryStatus = 'unknown'
  if (deltaStatus === 'applied') {
    status = 'applied'
  } else if (deltaStatus === 'rejected') {
    status = 'rejected'
  } else if (conflicts > 0) {
    status = 'blocked'
  } else if (deltaStatus === 'accepted') {
    status = 'accepted'
  } else if (deltaStatus === 'proposed') {
    status = 'proposed'
  } else if (bundle) {
    status = 'proposed'
  }

  return {
    status,
    items,
    conflicts,
    accepted,
    rejected,
    generatedAt,
    appliedAt,
  }
}

export function KnowledgeHistorySection({ entries, maxItems = 5, className }: KnowledgeHistorySectionProps) {
  const items = useMemo(
    () => (Array.isArray(entries) ? entries : []).filter((e) => Boolean(e?.bundle || e?.delta)).slice(0, Math.max(0, maxItems)),
    [entries, maxItems],
  )
  const [open, setOpen] = useState(false)

  if (items.length === 0) {
    return null
  }

  return (
    <details
      className={cn('rounded-md border border-border/60 bg-background-50 px-2.5 py-2', className)}
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Knowledge history (${items.length})`}
      </summary>

      <div className="mt-2 space-y-2">
        {items.map((entry, index) => {
          const bundle = entry.bundle ?? null
          const delta = entry.delta ?? null
          const summary = resolveSummary(entry)

          const key = bundle?.bundle_id
            ?? delta?.knowledge_delta_id
            ?? `${String(summary.generatedAt ?? '')}-${index}`

          const conflicts: KnowledgeConflictLike[] = Array.isArray(delta?.conflicts)
            ? (delta!.conflicts as KnowledgeConflictLike[])
            : []

          return (
            <details
              key={key}
              className="rounded-md border border-border/60 bg-background px-2.5 py-2"
            >
              <summary className="cursor-pointer select-none">
                <KnowledgeSummaryRow
                  status={summary.status}
                  items={summary.items}
                  conflicts={summary.conflicts}
                  accepted={summary.accepted}
                  rejected={summary.rejected}
                  generatedAt={summary.generatedAt}
                  appliedAt={summary.appliedAt}
                  className="border-0 bg-transparent px-0 py-0"
                />
              </summary>

              <div className="mt-2 space-y-2">
                {bundle ? (
                  <KnowledgeProposalBundleCard bundle={bundle} />
                ) : (
                  <div className="text-xs text-muted-foreground">No proposal bundle.</div>
                )}

                {conflicts.length > 0 ? (
                  <KnowledgeConflictList conflicts={conflicts} />
                ) : null}

                {delta ? (
                  <KnowledgeApplyStatusCard
                    delta={delta}
                    showActions={false}
                  />
                ) : (
                  <div className="text-xs text-muted-foreground">No knowledge delta.</div>
                )}
              </div>
            </details>
          )
        })}
      </div>
    </details>
  )
}
