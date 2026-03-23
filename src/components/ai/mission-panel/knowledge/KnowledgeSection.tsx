import { useState } from 'react'

import type { KnowledgeTimelineEntry } from './timeline'
import { KnowledgeSectionView } from './KnowledgeSectionView'
import type { KnowledgeLatestPayload } from '../types'
import { deriveKnowledgeSummary, useKnowledgeActions, useKnowledgeProposalSelection } from './model'

type KnowledgeSectionProps = {
  projectPath: string
  missionId: string
  knowledgeLatest: KnowledgeLatestPayload | null
  knowledgeError: string | null
  knowledgeTimeline: KnowledgeTimelineEntry[] | null
  knowledgeTimelineError: string | null
  knowledgeDecisionRequired: boolean
  onRefresh: () => void
}

export function KnowledgeSection({
  projectPath,
  missionId,
  knowledgeLatest,
  knowledgeError,
  knowledgeTimeline,
  knowledgeTimelineError,
  knowledgeDecisionRequired,
  onRefresh,
}: KnowledgeSectionProps) {
  const summary = deriveKnowledgeSummary(knowledgeLatest, knowledgeDecisionRequired)

  const [openOverride, setOpenOverride] = useState<boolean | null>(null)
  const open = openOverride ?? summary.defaultOpen

  const selection = useKnowledgeProposalSelection({
    bundleId: summary.bundle?.bundle_id ?? null,
    items: summary.proposalItems,
  })

  const actions = useKnowledgeActions({
    projectPath,
    missionId,
    knowledgeLatest,
    acceptedByItemId: selection.acceptedByItemId,
    onRefresh,
  })

  return (
    <KnowledgeSectionView
      knowledgeError={knowledgeError}
      open={open}
      onOpenChange={setOpenOverride}
      statusLabel={summary.statusLabel}
      proposalCount={summary.proposalCount}
      conflictCount={summary.conflictCount}
      acceptedCount={summary.acceptedCount}
      rejectedCount={summary.rejectedCount}
      bundle={summary.bundle}
      delta={summary.delta}
      knowledgeTimeline={knowledgeTimeline}
      knowledgeTimelineError={knowledgeTimelineError}
      canDecide={summary.canDecide}
      canApply={summary.canApply}
      canRollback={summary.canRollback}
      actionLoading={actions.actionLoading}
      actionError={actions.actionError}
      onDecide={actions.onDecide}
      onApply={actions.onApply}
      onRollback={actions.onRollback}
      onAcceptSafe={selection.onAcceptSafe}
      onAcceptAll={selection.onAcceptAll}
      onRejectAll={selection.onRejectAll}
      proposalItems={summary.proposalItems}
      acceptedByItemId={selection.acceptedByItemId}
      onToggleProposal={selection.onToggle}
    />
  )
}

