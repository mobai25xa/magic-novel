type KnowledgeProposalItem = {
  item_id: string
  kind: string
  op: string
  target_ref?: string | null
  accept_policy?: string | null
  change_reason?: string | null
  evidence_refs?: string[] | null
  source_refs?: string[] | null
  fields?: unknown
}

type KnowledgeProposalsListProps = {
  items: KnowledgeProposalItem[]
  acceptedByItemId: Record<string, boolean>
  disabled?: boolean
  onToggle: (item: KnowledgeProposalItem) => void
}

type KnowledgeProposalItemCardProps = {
  item: KnowledgeProposalItem
  checked: boolean
  disabled?: boolean
  onToggle: (item: KnowledgeProposalItem) => void
}

function KnowledgeProposalItemCard({ item, checked, disabled, onToggle }: KnowledgeProposalItemCardProps) {
  const policy = String(item.accept_policy ?? '').trim()
  const canToggleToAccept = policy !== 'orchestrator_only'
  const checkboxDisabled = disabled || (!canToggleToAccept && !checked)
  const checkboxTitle = !canToggleToAccept
    ? 'orchestrator_only items cannot be accepted by user'
    : undefined

  return (
    <div
      className="rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs"
    >
      <div className="flex items-start gap-2">
        <input
          type="checkbox"
          className="mt-0.5"
          checked={checked}
          onChange={() => onToggle(item)}
          disabled={checkboxDisabled}
          title={checkboxTitle}
        />

        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <span className="font-mono text-[11px] text-muted-foreground">{item.kind}</span>
            <span className="opacity-80">{item.op}</span>
            {item.target_ref ? (
              <span
                className="font-mono text-[11px] text-muted-foreground truncate"
                title={item.target_ref}
              >
                {item.target_ref}
              </span>
            ) : null}
            <span className="ml-auto font-mono text-[11px] text-muted-foreground">
              {String(item.accept_policy ?? '')}
            </span>
          </div>

          {item.change_reason ? (
            <div className="mt-1 text-muted-foreground break-words">{item.change_reason}</div>
          ) : null}

          {item.evidence_refs?.length ? (
            <details className="mt-1">
              <summary className="cursor-pointer select-none text-[11px] text-muted-foreground">
                Evidence
              </summary>
              <pre className="mt-1 max-h-28 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
                {item.evidence_refs.join('\n')}
              </pre>
            </details>
          ) : null}

          {item.source_refs?.length ? (
            <details className="mt-1">
              <summary className="cursor-pointer select-none text-[11px] text-muted-foreground">
                Sources
              </summary>
              <pre className="mt-1 max-h-28 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
                {item.source_refs.join('\n')}
              </pre>
            </details>
          ) : null}

          <details className="mt-1">
            <summary className="cursor-pointer select-none text-[11px] text-muted-foreground">
              Fields
            </summary>
            <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
              {JSON.stringify(item.fields ?? {}, null, 2)}
            </pre>
          </details>
        </div>
      </div>
    </div>
  )
}

export function KnowledgeProposalsList({ items, acceptedByItemId, disabled, onToggle }: KnowledgeProposalsListProps) {
  if (items.length === 0) {
    return <div className="text-muted-foreground">No proposals recorded yet.</div>
  }

  return (
    <div className="space-y-2">
      {items.map((item) => (
        <KnowledgeProposalItemCard
          key={item.item_id}
          item={item}
          checked={Boolean(acceptedByItemId[item.item_id])}
          disabled={disabled}
          onToggle={onToggle}
        />
      ))}
    </div>
  )
}
