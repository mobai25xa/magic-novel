import { Button } from '@/magic-ui/components'

import {
  MacroChapterQueueList,
  MacroStageTimeline,
  MacroWorkflowControls,
  MacroWorkflowSummaryRow,
  MacroWritePathsList,
} from '@/components/ai/macro'

import type { deriveMacroSummary } from './model'

type MacroSummary = ReturnType<typeof deriveMacroSummary>

type MacroDetailsPanelProps = {
  summary: MacroSummary
  open: boolean
  onOpenChange: (open: boolean) => void
  loading: boolean
  reviewActionLoading: boolean
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onCancel: () => void
  onRefresh: () => void
  onAutoFix: () => void
  onScrollToDecision: () => void
}

type MacroBlockedCardProps = {
  isBlocked: boolean
  currentStage: string | null
  blockReason: string | null
  canAutoFix: boolean
  needsDecision: boolean
  canResume: boolean
  loading: boolean
  reviewActionLoading: boolean
  onAutoFix: () => void
  onScrollToDecision: () => void
  onResume: () => void
}

function MacroBlockedCard({
  isBlocked,
  currentStage,
  blockReason,
  canAutoFix,
  needsDecision,
  canResume,
  loading,
  reviewActionLoading,
  onAutoFix,
  onScrollToDecision,
  onResume,
}: MacroBlockedCardProps) {
  if (!isBlocked) {
    return null
  }

  return (
    <div className="rounded-md border border-destructive/40 bg-destructive/5 px-2.5 py-2 space-y-2">
      <div className="text-xs font-medium text-destructive">
        {currentStage === 'failed' ? 'Macro failed' : 'Macro blocked'}
      </div>
      {blockReason ? <div className="text-xs text-muted-foreground break-words">{blockReason}</div> : null}
      <div className="flex flex-wrap gap-2">
        {canAutoFix ? (
          <Button size="sm" variant="outline" className="text-xs" onClick={onAutoFix} disabled={loading || reviewActionLoading}>
            Fix
          </Button>
        ) : null}
        {needsDecision ? (
          <Button size="sm" variant="outline" className="text-xs" onClick={onScrollToDecision} disabled={loading}>
            Decide
          </Button>
        ) : null}
        {canResume ? (
          <Button size="sm" className="text-xs" onClick={onResume} disabled={loading}>
            Resume
          </Button>
        ) : null}
      </div>
    </div>
  )
}

function MacroControlRow(input: {
  summary: MacroSummary
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onCancel: () => void
  onRefresh: () => void
  loading: boolean
}) {
  const phase = input.summary.phase
  const config = input.summary.macroConfig

  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <MacroWorkflowControls
        phase={phase}
        onStart={phase === 'awaiting_input' ? input.onStart : undefined}
        onPause={phase === 'running' ? input.onPause : undefined}
        onResume={phase === 'paused' ? input.onResume : undefined}
        onCancel={phase !== 'completed' ? input.onCancel : undefined}
        onRefresh={input.onRefresh}
        disabled={input.loading}
      />

      {config ? (
        <div className="font-mono text-[11px] text-muted-foreground break-all">
          {`macro: ${config.macro_id} · ${config.workflow_kind} · ${config.token_budget}`}
          {config.strict_review ? ' · strict_review' : ''}
          {config.auto_fix_on_block ? ' · auto_fix' : ''}
        </div>
      ) : null}
    </div>
  )
}

export function MacroDetailsPanel(props: MacroDetailsPanelProps) {
  const summary = props.summary
  if (!summary.hasMacro) {
    return null
  }

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={props.open}
      onToggle={(event) => props.onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Macro (${summary.currentStage ?? 'idle'})`}
        {summary.chapters.length > 0 ? ` · ${summary.completedCount}/${summary.chapters.length}` : ''}
        {summary.failedCount > 0 ? ` · failed ${summary.failedCount}` : ''}
      </summary>

      <div className="mt-2 space-y-2 text-xs">
        {summary.macroProgress ? (
          <MacroWorkflowSummaryRow
            objective={summary.macroConfig?.objective ?? summary.macroProgress.objective}
            currentIndex={summary.currentIndex}
            total={summary.chapters.length}
            currentStage={summary.currentStage ?? 'planning'}
            blocked={summary.isBlocked}
            lastTransitionAt={summary.macroProgress.last_transition_at}
          />
        ) : null}

        <MacroControlRow
          summary={summary}
          onStart={props.onStart}
          onPause={props.onPause}
          onResume={props.onResume}
          onCancel={props.onCancel}
          onRefresh={props.onRefresh}
          loading={props.loading}
        />

        {summary.writeTargets.length > 0 ? <MacroWritePathsList paths={summary.writeTargets} /> : null}

        {summary.currentChapter ? (
          <MacroStageTimeline currentStage={summary.currentChapter.stage} chapterStatus={summary.currentChapter.status} />
        ) : null}

        <div className="max-h-64 overflow-auto">
          <MacroChapterQueueList
            chapters={summary.chapters}
            currentIndex={summary.currentIndex >= 0 ? summary.currentIndex : undefined}
          />
        </div>

        <MacroBlockedCard
          isBlocked={summary.isBlocked}
          currentStage={summary.currentStage}
          blockReason={summary.blockReason}
          canAutoFix={summary.canAutoFix}
          needsDecision={summary.needsDecision}
          canResume={summary.canResume}
          loading={props.loading}
          reviewActionLoading={props.reviewActionLoading}
          onAutoFix={props.onAutoFix}
          onScrollToDecision={props.onScrollToDecision}
          onResume={props.onResume}
        />
      </div>
    </details>
  )
}
