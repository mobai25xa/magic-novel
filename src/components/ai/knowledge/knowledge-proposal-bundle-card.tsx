import { useMemo } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, ShowMore, Tab, TabPanel, Tabs } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'
import { JsonDiffView } from './json-diff-view'

export type KnowledgeProposalItemLike = {
  item_id: string
  kind: string
  op: string
  target_ref?: string
  target_revision?: number
  fields: Record<string, unknown>
  evidence_refs: string[]
  source_refs: string[]
  change_reason: string
  accept_policy: string
}

export type KnowledgeProposalBundleLike = {
  schema_version?: number
  bundle_id: string
  scope_ref: string
  branch_id?: string
  source_session_id: string
  source_review_id?: string
  generated_at: number | string
  proposal_items: KnowledgeProposalItemLike[]
}

export type KnowledgeProposalBundleCardProps = {
  bundle: KnowledgeProposalBundleLike
  className?: string
}

type NormalizedAcceptPolicy = 'auto_if_pass' | 'manual' | 'orchestrator_only' | 'unknown'

const acceptPolicyOrder: NormalizedAcceptPolicy[] = [
  'auto_if_pass',
  'manual',
  'orchestrator_only',
  'unknown',
]

function normalizeAcceptPolicy(value: unknown): NormalizedAcceptPolicy {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'auto_if_pass' || normalized === 'auto') return 'auto_if_pass'
  if (normalized === 'manual') return 'manual'
  if (normalized === 'orchestrator_only' || normalized === 'orchestrator') return 'orchestrator_only'
  return 'unknown'
}

function resolveAcceptPolicyLabel(policy: NormalizedAcceptPolicy) {
  switch (policy) {
    case 'auto_if_pass':
      return 'auto'
    case 'manual':
      return 'manual'
    case 'orchestrator_only':
      return 'orchestrator'
    default:
      return 'unknown'
  }
}

function resolveAcceptPolicyColor(policy: NormalizedAcceptPolicy): BadgeProps['color'] {
  switch (policy) {
    case 'auto_if_pass':
      return 'success'
    case 'manual':
      return 'warning'
    case 'orchestrator_only':
      return 'info'
    default:
      return 'default'
  }
}

function normalizeOp(value: unknown): 'create' | 'update' | 'archive' | 'restore' | 'unknown' {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'create') return 'create'
  if (normalized === 'update') return 'update'
  if (normalized === 'archive') return 'archive'
  if (normalized === 'restore') return 'restore'
  return 'unknown'
}

function resolveOpColor(op: ReturnType<typeof normalizeOp>): BadgeProps['color'] {
  switch (op) {
    case 'create':
      return 'success'
    case 'update':
      return 'info'
    case 'archive':
      return 'warning'
    case 'restore':
      return 'info'
    default:
      return 'default'
  }
}

function formatGeneratedAt(value: KnowledgeProposalBundleLike['generated_at']) {
  if (value === null || value === undefined) {
    return null
  }

  try {
    const date = typeof value === 'number' ? new Date(value) : new Date(String(value))
    if (Number.isNaN(date.getTime())) {
      return null
    }
    return {
      title: String(value),
      label: date.toLocaleString(),
    }
  } catch {
    return null
  }
}

function safeStringify(value: unknown) {
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function extractBeforeAfter(fields: unknown): { before: unknown; after: unknown } | null {
  if (!fields || typeof fields !== 'object') {
    return null
  }

  const record = fields as Record<string, unknown>
  const before = record.before ?? record.before_fields ?? record.beforeFields ?? record.prev ?? record.previous
  const after = record.after ?? record.after_fields ?? record.afterFields ?? record.next ?? record.current

  if (before === undefined || after === undefined) {
    return null
  }

  return { before, after }
}

function ProposalItemCard({ item, policy }: { item: KnowledgeProposalItemLike; policy: NormalizedAcceptPolicy }) {
  const op = normalizeOp(item.op)
  const fieldsText = safeStringify(item.fields)
  const diffPayload = extractBeforeAfter(item.fields)

  return (
    <details className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs">
      <summary className="cursor-pointer select-none">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline" size="sm" className="font-mono">
                {item.kind}
              </Badge>
              <Badge color={resolveOpColor(op)} variant="soft" size="sm">
                {op}
              </Badge>
              {item.target_ref ? (
                <span className="font-mono text-[11px] text-muted-foreground truncate" title={item.target_ref}>
                  {item.target_ref}
                </span>
              ) : null}
            </div>

            {item.change_reason ? (
              <div className="mt-1 text-muted-foreground break-words leading-relaxed">
                {item.change_reason}
              </div>
            ) : null}
          </div>

          <Badge color={resolveAcceptPolicyColor(policy)} variant="outline" size="sm" title="accept policy">
            {resolveAcceptPolicyLabel(policy)}
          </Badge>
        </div>
      </summary>

      <div className="mt-2 space-y-2">
        <div className="flex flex-wrap gap-1.5">
          <CopyPill value={item.item_id} title="Copy item_id" />
          {item.target_ref ? <CopyPill value={item.target_ref} title="Copy target_ref" /> : null}
          {typeof item.target_revision === 'number' ? (
            <Badge variant="outline" size="sm" className="font-mono" title="target revision">
              {`rev ${item.target_revision}`}
            </Badge>
          ) : null}
        </div>

        {item.evidence_refs && item.evidence_refs.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Evidence</div>
            <div className="mt-1 flex flex-wrap gap-1.5">
              {item.evidence_refs.map((ref, idx) => (
                <CopyPill key={`${item.item_id}-evidence-${idx}`} value={ref} title="Copy evidence" />
              ))}
            </div>
          </div>
        ) : null}

        {item.source_refs && item.source_refs.length > 0 ? (
          <div>
            <div className="text-[11px] font-medium text-secondary-foreground">Sources</div>
            <div className="mt-1 flex flex-wrap gap-1.5">
              {item.source_refs.map((ref, idx) => (
                <CopyPill key={`${item.item_id}-source-${idx}`} value={ref} title="Copy source" />
              ))}
            </div>
          </div>
        ) : null}

        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Fields</div>

          {diffPayload ? (
            <Tabs defaultValue="diff" className="mt-1">
              <Tab value="diff" className="text-[11px]">
                Diff
              </Tab>
              <Tab value="json" className="text-[11px]">
                JSON
              </Tab>

              <TabPanel value="diff" className="mt-2">
                <JsonDiffView before={diffPayload.before} after={diffPayload.after} />
              </TabPanel>
              <TabPanel value="json" className="mt-2">
                <ShowMore maxLines={8}>
                  <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                    {fieldsText}
                  </pre>
                </ShowMore>
              </TabPanel>
            </Tabs>
          ) : (
            <ShowMore maxLines={8} className="mt-1">
              <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                {fieldsText}
              </pre>
            </ShowMore>
          )}
        </div>
      </div>
    </details>
  )
}

export function KnowledgeProposalBundleCard({ bundle, className }: KnowledgeProposalBundleCardProps) {
  const items = useMemo(
    () => (Array.isArray(bundle?.proposal_items) ? bundle.proposal_items : []),
    [bundle],
  )

  const groups = useMemo(() => {
    const next: Record<NormalizedAcceptPolicy, KnowledgeProposalItemLike[]> = {
      auto_if_pass: [],
      manual: [],
      orchestrator_only: [],
      unknown: [],
    }

    for (const item of items) {
      next[normalizeAcceptPolicy(item.accept_policy)].push(item)
    }

    return next
  }, [items])

  const generatedAt = formatGeneratedAt(bundle?.generated_at)

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs space-y-2',
        className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">Knowledge proposals</span>
            <Badge variant="outline" size="sm" className="font-mono" title="bundle_id">
              {bundle.bundle_id}
            </Badge>
            <span className="text-[11px] text-muted-foreground">
              {`items ${items.length}`}
            </span>
          </div>

          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            <CopyPill value={bundle.scope_ref} title="Copy scope_ref" />
            {bundle.branch_id ? <CopyPill value={bundle.branch_id} title="Copy branch_id" /> : null}
            {bundle.source_session_id ? <CopyPill value={bundle.source_session_id} title="Copy source_session_id" /> : null}
            {bundle.source_review_id ? <CopyPill value={bundle.source_review_id} title="Copy source_review_id" /> : null}
          </div>
        </div>

        {generatedAt ? (
          <span
            className="shrink-0 text-[11px] text-muted-foreground"
            title={generatedAt.title}
          >
            {generatedAt.label}
          </span>
        ) : null}
      </div>

      {items.length === 0 ? (
        <div className="text-muted-foreground">No proposal items.</div>
      ) : (
        <div className="space-y-3">
          {acceptPolicyOrder
            .filter((policy) => groups[policy].length > 0)
            .map((policy) => (
              <div key={policy} className="space-y-1.5">
                <div className="flex items-center gap-2">
                  <Badge color={resolveAcceptPolicyColor(policy)} variant="soft" size="sm">
                    {resolveAcceptPolicyLabel(policy)}
                  </Badge>
                  <span className="text-[11px] text-muted-foreground">{groups[policy].length}</span>
                </div>

                <div className="space-y-2">
                  {groups[policy].map((item) => (
                    <ProposalItemCard key={item.item_id} item={item} policy={policy} />
                  ))}
                </div>
              </div>
            ))}
        </div>
      )}
    </div>
  )
}
