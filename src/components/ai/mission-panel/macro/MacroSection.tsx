import { useMemo, useState } from 'react'

import type { MacroStatePayload, MissionStatusPayload } from '../types'
import { MacroCreatePanel } from './MacroCreatePanel'
import { MacroDetailsPanel } from './MacroDetailsPanel'
import { deriveMacroSummary } from './model'

type MacroSectionProps = {
  projectPath: string
  missionId: string
  liveState: string
  statusDetail: MissionStatusPayload | null
  macroState: MacroStatePayload | null
  macroFetchError: string | null
  loading: boolean
  reviewActionLoading: boolean
  reviewAutoFixAvailable: boolean
  reviewDecisionRequired: boolean
  knowledgeDecisionRequired: boolean
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onCancel: () => void
  onRefresh: () => void
  onAutoFix: () => void
  onScrollToDecision: () => void
}

export function MacroSection({
  projectPath,
  liveState,
  statusDetail,
  macroState,
  macroFetchError,
  loading,
  reviewActionLoading,
  reviewAutoFixAvailable,
  reviewDecisionRequired,
  knowledgeDecisionRequired,
  onStart,
  onPause,
  onResume,
  onCancel,
  onRefresh,
  onAutoFix,
  onScrollToDecision,
}: MacroSectionProps) {
  const summary = useMemo(() => deriveMacroSummary({
    liveState,
    macroState,
    reviewAutoFixAvailable,
    reviewDecisionRequired,
    knowledgeDecisionRequired,
  }), [knowledgeDecisionRequired, liveState, macroState, reviewAutoFixAvailable, reviewDecisionRequired])

  const [detailsOpen, setDetailsOpen] = useState(false)

  return (
    <div className="space-y-2">
      {macroFetchError ? (
        <p className="text-xs text-muted-foreground">Macro unavailable: {macroFetchError}</p>
      ) : null}

      {summary.hasMacro ? (
        <MacroDetailsPanel
          summary={summary}
          open={detailsOpen}
          onOpenChange={setDetailsOpen}
          loading={loading}
          reviewActionLoading={reviewActionLoading}
          onStart={onStart}
          onPause={onPause}
          onResume={onResume}
          onCancel={onCancel}
          onRefresh={onRefresh}
          onAutoFix={onAutoFix}
          onScrollToDecision={onScrollToDecision}
        />
      ) : (
        <MacroCreatePanel
          projectPath={projectPath}
          statusDetail={statusDetail}
          loading={loading}
          onRefresh={onRefresh}
          onOpenDetails={() => setDetailsOpen(true)}
        />
      )}
    </div>
  )
}
