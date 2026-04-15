import { Button } from '@/magic-ui/components'
import type { KnowledgeConflict, KnowledgeDelta, KnowledgeDeltaChange, KnowledgeProposalBundle } from '@/types/knowledge'

import type { KnowledgeLatestPayload } from '../types'
import type { KnowledgeTimelineEntry } from './timeline'
import { KnowledgeProposalsList, type KnowledgeProposalItem } from './KnowledgeProposalsList'

type KnowledgeBundle = KnowledgeProposalBundle | NonNullable<KnowledgeLatestPayload>['bundle']
type KnowledgeDeltaPayload = KnowledgeDelta | NonNullable<KnowledgeLatestPayload>['delta']

type KnowledgeSectionViewProps = {
  knowledgeError: string | null
  open: boolean
  onOpenChange: (open: boolean) => void
  statusLabel: string
  proposalCount: number
  conflictCount: number
  acceptedCount: number
  rejectedCount: number
  bundle: KnowledgeBundle | null
  delta: KnowledgeDeltaPayload | null
  knowledgeTimeline: KnowledgeTimelineEntry[] | null
  knowledgeTimelineError: string | null
  canDecide: boolean
  canApply: boolean
  canRollback: boolean
  actionLoading: boolean
  actionError: string | null
  onDecide: () => void
  onApply: () => void
  onRollback: () => void
  onAcceptSafe: () => void
  onAcceptAll: () => void
  onRejectAll: () => void
  proposalItems: KnowledgeProposalItem[]
  acceptedByItemId: Record<string, boolean>
  onToggleProposal: (item: KnowledgeProposalItem) => void
}
function KnowledgeIds({ bundle, delta }: { bundle: KnowledgeBundle | null; delta: KnowledgeDeltaPayload | null }) {
  return (
    <>
      {bundle?.bundle_id ? (
        <div className="font-mono text-[11px] text-muted-foreground break-all">
          {`bundle: ${bundle.bundle_id} · scope: ${bundle.scope_ref}`}
        </div>
      ) : null}

      {delta?.knowledge_delta_id ? (
        <div className="font-mono text-[11px] text-muted-foreground break-all">
          {`delta: ${delta.knowledge_delta_id}`}
        </div>
      ) : null}
    </>
  )
}

function KnowledgeStatsRow(input: {
  acceptedCount: number
  rejectedCount: number
  bundle: KnowledgeBundle | null
  delta: KnowledgeDeltaPayload | null
}) {
  const appliedAt = typeof input.delta?.applied_at === 'number' ? input.delta.applied_at : null
  const generatedAt = typeof input.bundle?.generated_at === 'number' ? input.bundle.generated_at : null
  const rollbackToken = input.delta?.rollback?.token ? String(input.delta.rollback.token) : null

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span className="text-muted-foreground">Accepted</span>
      <span className="font-medium text-foreground">{input.acceptedCount}</span>
      <span className="text-muted-foreground">Rejected</span>
      <span className="font-medium text-foreground">{input.rejectedCount}</span>
      {appliedAt ? (
        <>
          <span className="text-muted-foreground">Applied</span>
          <span className="font-medium text-foreground">{new Date(appliedAt).toLocaleTimeString()}</span>
        </>
      ) : generatedAt ? (
        <>
          <span className="text-muted-foreground">Generated</span>
          <span className="font-medium text-foreground">{new Date(generatedAt).toLocaleTimeString()}</span>
        </>
      ) : null}
      {rollbackToken ? (
        <>
          <span className="text-muted-foreground">Rollback</span>
          <span className="font-mono text-[11px] text-muted-foreground">token</span>
        </>
      ) : null}
    </div>
  )
}

function KnowledgeTimelineDetails(input: {
  knowledgeTimeline: KnowledgeTimelineEntry[] | null
  knowledgeTimelineError: string | null
}) {
  return (
    <details className="rounded-md border border-border/60 bg-background px-2.5 py-2">
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Timeline (${input.knowledgeTimeline?.length ?? 0})`}
      </summary>

      {input.knowledgeTimelineError ? (
        <p className="mt-2 text-xs text-muted-foreground">Timeline unavailable: {input.knowledgeTimelineError}</p>
      ) : input.knowledgeTimeline && input.knowledgeTimeline.length > 0 ? (
        <div className="mt-2 max-h-48 overflow-auto space-y-1 font-mono text-[11px] text-muted-foreground">
          {input.knowledgeTimeline.map((entry) => (
            <div key={entry.key} className="break-words">
              <span className="opacity-50">{new Date(entry.ts).toLocaleTimeString()} </span>
              <span className="text-foreground">{entry.label}</span>
              {entry.detail ? <span className="opacity-70">{` · ${entry.detail}`}</span> : null}
            </div>
          ))}
        </div>
      ) : (
        <p className="mt-2 text-xs text-muted-foreground">No knowledge history recorded yet.</p>
      )}
    </details>
  )
}

function KnowledgeConflictsCard({ delta }: { delta: KnowledgeDeltaPayload | null }) {
  const conflicts: KnowledgeConflict[] = Array.isArray(delta?.conflicts) ? delta.conflicts : []
  if (conflicts.length === 0) {
    return null
  }

  return (
    <div className="rounded-md border border-border/60 bg-warning/5 px-2.5 py-2 text-xs">
      <div className="font-medium text-secondary-foreground">
        {`Blocked by conflicts (${conflicts.length})`}
      </div>
      <ul className="mt-1 space-y-1 text-muted-foreground">
        {conflicts.map((conflict, idx) => (
          <li key={idx} className="break-words">
            <span className="font-mono">{String(conflict?.type ?? '')}</span>
            {': '}
            {String(conflict?.message ?? '')}
          </li>
        ))}
      </ul>
    </div>
  )
}

function KnowledgeActionsBar(input: {
  canDecide: boolean
  canApply: boolean
  canRollback: boolean
  actionLoading: boolean
  onDecide: () => void
  onApply: () => void
  onRollback: () => void
  proposalCount: number
  onAcceptSafe: () => void
  onAcceptAll: () => void
  onRejectAll: () => void
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {input.canDecide ? (
        <Button type="button" size="sm" variant="outline" className="text-xs" onClick={input.onDecide} disabled={input.actionLoading}>
          {input.actionLoading ? 'Deciding…' : 'Decide'}
        </Button>
      ) : null}

      {input.canApply ? (
        <Button type="button" size="sm" variant="outline" className="text-xs" onClick={input.onApply} disabled={input.actionLoading}>
          {input.actionLoading ? 'Applying…' : 'Apply'}
        </Button>
      ) : null}

      {input.canRollback ? (
        <Button type="button" size="sm" variant="destructive" className="text-xs" onClick={input.onRollback} disabled={input.actionLoading}>
          {input.actionLoading ? 'Rolling back…' : 'Rollback'}
        </Button>
      ) : null}

      {input.proposalCount > 0 ? (
        <>
          <Button type="button" size="sm" variant="outline" className="text-xs" onClick={input.onAcceptSafe} disabled={input.actionLoading}>
            Accept safe
          </Button>
          <Button type="button" size="sm" variant="outline" className="text-xs" onClick={input.onAcceptAll} disabled={input.actionLoading}>
            Accept all
          </Button>
          <Button type="button" size="sm" variant="outline" className="text-xs" onClick={input.onRejectAll} disabled={input.actionLoading}>
            Reject all
          </Button>
        </>
      ) : null}
    </div>
  )
}

function KnowledgeDeltaChangesDetails({ delta }: { delta: KnowledgeDeltaPayload | null }) {
  const changes: KnowledgeDeltaChange[] = Array.isArray(delta?.changes) ? delta.changes : []
  if (changes.length === 0) {
    return null
  }

  return (
    <details className="rounded-md border border-border/60 bg-background px-2.5 py-2">
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Delta changes (${changes.length})`}
      </summary>
      <ul className="mt-2 space-y-1 text-muted-foreground">
        {changes.slice(0, 12).map((change, idx) => (
          <li key={idx} className="break-words">
            <span className="font-mono">{String(change?.kind ?? '')}</span>
            {': '}
            {String(change?.summary ?? '')}
          </li>
        ))}
      </ul>
    </details>
  )
}

function KnowledgeDetailsBody(props: KnowledgeSectionViewProps) {
  return (
    <div className="mt-2 space-y-2 text-xs">
      <KnowledgeIds bundle={props.bundle} delta={props.delta} />

      <KnowledgeStatsRow acceptedCount={props.acceptedCount} rejectedCount={props.rejectedCount} bundle={props.bundle} delta={props.delta} />

      <KnowledgeTimelineDetails knowledgeTimeline={props.knowledgeTimeline} knowledgeTimelineError={props.knowledgeTimelineError} />

      <KnowledgeConflictsCard delta={props.delta} />

      <KnowledgeActionsBar
        canDecide={props.canDecide}
        canApply={props.canApply}
        canRollback={props.canRollback}
        actionLoading={props.actionLoading}
        onDecide={props.onDecide}
        onApply={props.onApply}
        onRollback={props.onRollback}
        proposalCount={props.proposalCount}
        onAcceptSafe={props.onAcceptSafe}
        onAcceptAll={props.onAcceptAll}
        onRejectAll={props.onRejectAll}
      />

      {props.actionError ? (
        <p className="text-xs text-muted-foreground">Knowledge action failed: {props.actionError}</p>
      ) : null}

      <KnowledgeProposalsList
        items={props.proposalItems}
        acceptedByItemId={props.acceptedByItemId}
        disabled={props.actionLoading}
        onToggle={props.onToggleProposal}
      />

      <KnowledgeDeltaChangesDetails delta={props.delta} />
    </div>
  )
}

export function KnowledgeSectionView(props: KnowledgeSectionViewProps) {
  return (
    <div className="space-y-2">
      {props.knowledgeError ? (
        <p className="text-xs text-muted-foreground">Knowledge unavailable: {props.knowledgeError}</p>
      ) : null}

      <details
        className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
        open={props.open}
        onToggle={(event) => props.onOpenChange(event.currentTarget.open)}
      >
        <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
          {`Knowledge (${props.statusLabel})`}
          {props.proposalCount > 0 ? ` · proposals ${props.proposalCount}` : ''}
          {props.conflictCount > 0 ? ` · conflicts ${props.conflictCount}` : ''}
        </summary>

        <KnowledgeDetailsBody {...props} />
      </details>
    </div>
  )
}
