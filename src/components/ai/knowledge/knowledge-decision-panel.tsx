import { useCallback, useMemo, useState } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, Button } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'

type NormalizedAcceptPolicy = 'auto_if_pass' | 'manual' | 'orchestrator_only' | 'unknown'
type Decision = 'accept' | 'reject' | 'undecided'
type DecisionOverride = Exclude<Decision, 'undecided'>

export type KnowledgeDecisionItemLike = {
  item_id: string
  kind: string
  op: string
  target_ref?: string
  accept_policy: string
  change_reason?: string
}

export type KnowledgeDecisionPanelSubmit = {
  accepted_item_ids: string[]
  rejected_item_ids: string[]
}

export type KnowledgeDecisionPanelProps = {
  items: KnowledgeDecisionItemLike[]
  disabled?: boolean
  onSubmit?: (decision: KnowledgeDecisionPanelSubmit) => void
  className?: string
}

function normalizeAcceptPolicy(value: unknown): NormalizedAcceptPolicy {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'auto_if_pass' || normalized === 'auto') return 'auto_if_pass'
  if (normalized === 'manual') return 'manual'
  if (normalized === 'orchestrator_only' || normalized === 'orchestrator') return 'orchestrator_only'
  return 'unknown'
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

export function KnowledgeDecisionPanel({ items, disabled, onSubmit, className }: KnowledgeDecisionPanelProps) {
  const entries = useMemo(
    () => (Array.isArray(items) ? items.filter((item): item is KnowledgeDecisionItemLike => Boolean(item?.item_id)) : []),
    [items],
  )

  const [overrides, setOverrides] = useState<Record<string, DecisionOverride>>({})

  const safeItemIds = useMemo(
    () => entries.filter((item) => normalizeAcceptPolicy(item.accept_policy) === 'auto_if_pass').map((item) => item.item_id),
    [entries],
  )

  const riskyItemIds = useMemo(
    () => entries.filter((item) => normalizeAcceptPolicy(item.accept_policy) !== 'auto_if_pass').map((item) => item.item_id),
    [entries],
  )

  const { decisionById, acceptedItemIds, rejectedItemIds, undecidedCount } = useMemo(() => {
    const decisionById: Record<string, Decision> = {}
    const acceptedItemIds: string[] = []
    const rejectedItemIds: string[] = []
    let undecidedCount = 0

    for (const item of entries) {
      const policy = normalizeAcceptPolicy(item.accept_policy)
      const defaultDecision: Decision = policy === 'auto_if_pass' ? 'accept' : 'undecided'
      const decision: Decision = overrides[item.item_id] ?? defaultDecision

      decisionById[item.item_id] = decision
      if (decision === 'accept') {
        acceptedItemIds.push(item.item_id)
      } else if (decision === 'reject') {
        rejectedItemIds.push(item.item_id)
      } else {
        undecidedCount += 1
      }
    }

    return {
      decisionById,
      acceptedItemIds,
      rejectedItemIds,
      undecidedCount,
    }
  }, [entries, overrides])

  const canSubmit = Boolean(onSubmit) && !disabled && entries.length > 0 && undecidedCount === 0

  const handleSetDecision = useCallback((item: KnowledgeDecisionItemLike, next: DecisionOverride) => {
    const itemId = item.item_id
    const policy = normalizeAcceptPolicy(item.accept_policy)
    const defaultDecision: Decision = policy === 'auto_if_pass' ? 'accept' : 'undecided'

    setOverrides((prev) => {
      const current = prev[itemId]
      if (next === defaultDecision) {
        if (!current) {
          return prev
        }
        const { [itemId]: _removed, ...rest } = prev
        return rest
      }

      if (current === next) {
        return prev
      }

      return {
        ...prev,
        [itemId]: next,
      }
    })
  }, [])

  const handleAcceptSafe = useCallback(() => {
    setOverrides((prev) => {
      let changed = false
      const next = { ...prev }
      for (const id of safeItemIds) {
        if (next[id]) {
          delete next[id]
          changed = true
        }
      }
      return changed ? next : prev
    })
  }, [safeItemIds])

  const handleRejectRisky = useCallback(() => {
    setOverrides((prev) => {
      let changed = false
      const next = { ...prev }
      for (const id of riskyItemIds) {
        if (next[id] !== 'reject') {
          next[id] = 'reject'
          changed = true
        }
      }
      return changed ? next : prev
    })
  }, [riskyItemIds])

  const handleClear = useCallback(() => {
    setOverrides({})
  }, [])

  const handleSubmit = useCallback(() => {
    if (!onSubmit || disabled) {
      return
    }

    onSubmit({
      accepted_item_ids: acceptedItemIds,
      rejected_item_ids: rejectedItemIds,
    })
  }, [acceptedItemIds, disabled, onSubmit, rejectedItemIds])

  if (entries.length === 0) {
    return null
  }

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs space-y-2',
        className,
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">Decision</span>
            <span className="text-[11px] text-muted-foreground">
              {`items ${entries.length}`}
              {undecidedCount > 0 ? ` · undecided ${undecidedCount}` : ''}
              {acceptedItemIds.length > 0 ? ` · accept ${acceptedItemIds.length}` : ''}
              {rejectedItemIds.length > 0 ? ` · reject ${rejectedItemIds.length}` : ''}
            </span>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="text-xs font-medium disabled:opacity-50"
            onClick={handleAcceptSafe}
            disabled={disabled || safeItemIds.length === 0}
          >
            Accept safe
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="text-xs font-medium disabled:opacity-50"
            onClick={handleRejectRisky}
            disabled={disabled || riskyItemIds.length === 0}
          >
            Reject risky
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            className="text-xs font-medium disabled:opacity-50"
            onClick={handleClear}
            disabled={disabled}
          >
            Reset
          </Button>
        </div>
      </div>

      <div className="space-y-2">
        {entries.map((item) => {
          const policy = normalizeAcceptPolicy(item.accept_policy)
          const op = normalizeOp(item.op)
          const decision = decisionById[item.item_id] ?? 'undecided'

          return (
            <div
              key={item.item_id}
              className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline" size="sm" className="font-mono">
                      {item.kind}
                    </Badge>
                    <Badge color={resolveOpColor(op)} variant="soft" size="sm">
                      {op}
                    </Badge>
                    <Badge
                      color={resolveAcceptPolicyColor(policy)}
                      variant="outline"
                      size="sm"
                      title="accept policy"
                    >
                      {resolveAcceptPolicyLabel(policy)}
                    </Badge>
                  </div>

                  {item.change_reason ? (
                    <div className="mt-1 text-muted-foreground break-words leading-relaxed">
                      {item.change_reason}
                    </div>
                  ) : null}

                  <div className="mt-2 flex flex-wrap gap-1.5">
                    <CopyPill value={item.item_id} title="Copy item_id" />
                    {item.target_ref ? <CopyPill value={item.target_ref} title="Copy target_ref" /> : null}
                  </div>
                </div>

                <div className="flex shrink-0 items-center gap-1">
                  <Button
                    type="button"
                    size="sm"
                    variant={decision === 'accept' ? 'default' : 'outline'}
                    className="text-xs font-medium disabled:opacity-50"
                    onClick={() => handleSetDecision(item, 'accept')}
                    disabled={disabled}
                  >
                    Accept
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant={decision === 'reject' ? 'destructive' : 'outline'}
                    className="text-xs font-medium disabled:opacity-50"
                    onClick={() => handleSetDecision(item, 'reject')}
                    disabled={disabled}
                  >
                    Reject
                  </Button>
                </div>
              </div>
            </div>
          )
        })}
      </div>

      <div className="flex items-center justify-end gap-2">
        <Button
          type="button"
          size="sm"
          className="text-xs font-medium disabled:opacity-50"
          onClick={handleSubmit}
          disabled={!canSubmit}
          title={undecidedCount > 0 ? 'Resolve all items before submitting' : undefined}
        >
          Submit decision
        </Button>
      </div>
    </div>
  )
}
