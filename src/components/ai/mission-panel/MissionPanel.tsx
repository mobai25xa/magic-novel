import { memo, useCallback, useMemo, useRef, useState } from 'react'
import { AiPanelCardShell } from '@/magic-ui/components'
import type { ReviewReportLike } from '@/components/ai/review-report-card'
import { KnowledgeSection } from './knowledge/KnowledgeSection'
import { MacroSection } from './macro/MacroSection'
import { Layer1ContextPackSection } from './sections/Layer1ContextPackSection'
import { FeaturesSection } from './sections/FeaturesSection'
import { JobStatusSection } from './sections/JobStatusSection'
import { RecoverySection } from './sections/RecoverySection'
import { TaskResultsSection } from './sections/TaskResultsSection'
import { MissionActionButtons } from './sections/MissionActionButtons'
import { MissionHeader } from './sections/MissionHeader'
import { MissionSummaryCard } from './sections/MissionSummaryCard'
import { ProgressSection } from './sections/ProgressSection'
import { ReviewSection } from './sections/ReviewSection'
import { WorkersSection } from './sections/WorkersSection'
import {
  computeIssueCountByWorkerId,
  maxUpdatedAt,
  resolveJobStatusView,
  resolveTaskResultsFromJobSnapshotFirst,
  resolveWorkersDefaultOpen,
} from './derived'
import type { MissionPanelProps } from './types'
import { useMissionPanelBackend } from './useMissionPanelBackend'
import { useMissionPanelActions } from './useMissionPanelActions'

const MissionPanelInner = memo(function MissionPanelInner(props: MissionPanelProps) {
  const { projectPath, missionId, onClose } = props
  const { missionUi, backend, setBackend, refreshStatus } = useMissionPanelBackend({ projectPath, missionId })
  const reviewDecisionRef = useRef<HTMLDivElement | null>(null)

  const {
    loading,
    error,
    buildingContextPack,
    reviewActionLoading,
    reviewActionError,
    onSaveChapterCard,
    onSaveRecentFacts,
    onSaveActiveCast,
    onCreateDefaultChapterCard,
    onInferScopeFromCurrentChapter,
    onStart,
    onPause,
    onResume,
    onCancel,
    onAbandon,
    canRecover,
    recoverLabel,
    onRecover,
    onBuildContextPack,
    onFetchLatestContextPack,
    onAnswerOption,
    scrollToDecision,
  } = useMissionPanelActions({
    projectPath,
    missionId,
    backend,
    setBackend,
    refreshStatus,
    reviewDecisionRef,
  })

  const [featuresOpen, setFeaturesOpen] = useState(false)
  const [workersOpenOverride, setWorkersOpenOverride] = useState<boolean | null>(null)
  const [resultsOpenOverride, setResultsOpenOverride] = useState<boolean | null>(null)
  const [recoveryOpenOverride, setRecoveryOpenOverride] = useState<boolean | null>(null)
  const [progressOpen, setProgressOpen] = useState(false)
  const [resultOpenByKey, setResultOpenByKey] = useState<Record<string, boolean>>({})

  const liveState = backend.jobSnapshot?.status ?? backend.statusDetail?.state.state ?? missionUi?.state ?? 'awaiting_input'
  const jobStatusSummary = useMemo(() => resolveJobStatusView({
    jobSnapshot: backend.jobSnapshot,
    statusDetail: backend.statusDetail,
    fallbackStatus: liveState,
  }), [backend.jobSnapshot, backend.statusDetail, liveState])
  const features = backend.statusDetail?.features.features ?? []
  const taskResults = resolveTaskResultsFromJobSnapshotFirst({
    jobSnapshot: backend.jobSnapshot,
    statusDetail: backend.statusDetail,
  })
  const progressLog = missionUi?.progressLog ?? []
  const workerEntries = useMemo(() => {
    const workerStatuses = missionUi?.workerStatuses ?? {}
    return Object.entries(workerStatuses).sort(([, left], [, right]) => right.updatedAt - left.updatedAt)
  }, [missionUi?.workerStatuses])

  const completedFeatureCount = backend.jobSnapshot?.completed_tasks.length
    ?? features.filter((f) => f.status === 'completed').length
  const failedFeatureCount = backend.jobSnapshot?.failed_tasks.length
    ?? features.filter((f) => f.status === 'failed').length
  const runningWorkersCount = workerEntries.filter(([, info]) => info.status === 'running').length
  const failedWorkersCount = workerEntries.filter(([, info]) => info.status === 'failed').length
  const failedResultCount = taskResults.filter((entry) => entry.status !== 'completed').length
  const issueCountByWorkerId = computeIssueCountByWorkerId(taskResults)
  const lastProgress = progressLog.length > 0 ? progressLog[progressLog.length - 1] : null

  const isRunning = liveState === 'running' || liveState === 'initializing'
  const isCompleted = liveState === 'completed'
  const canStart = liveState === 'awaiting_input'
    || liveState === 'orchestrator_turn'
    || liveState === 'draft'
    || liveState === 'ready'
  const canResume = jobStatusSummary.canResume
  const canPause = isRunning
  const canStop = isRunning
  const canAbandon = !isCompleted

  const workersDefaultOpen = resolveWorkersDefaultOpen({ liveState, workerEntries, failedResults: failedResultCount })
  const workersOpen = workersOpenOverride ?? workersDefaultOpen
  const resultsOpen = resultsOpenOverride ?? (failedResultCount > 0)
  const recoveryOpen = recoveryOpenOverride ?? jobStatusSummary.shouldOpenRecovery

  const layer1LastUpdatedAt = maxUpdatedAt(backend.layer1)
  const contextPackGeneratedAt = backend.contextPack?.generated_at ?? 0
  const contextPackStale = backend.contextPack != null && layer1LastUpdatedAt > contextPackGeneratedAt

  const reportLike = backend.reviewReport as unknown as ReviewReportLike | null
  const historyLike = backend.reviewHistory as unknown as ReviewReportLike[] | null

  const fixInProgress = Boolean(missionUi?.fixupInProgress)
  const fixAttempt = missionUi?.fixupAttempt
  const fixMessage = missionUi?.fixupMessage
  const fixUpdatedAt = missionUi?.fixupUpdatedAt

  const reviewDecisionRequired = Boolean(missionUi?.reviewDecisionRequired)
  const knowledgeDecisionRequired = Boolean(missionUi?.knowledgeDecisionRequired)
  const waitingDecision = Boolean(backend.reviewDecision) || reviewDecisionRequired
  const decisionReason = backend.reviewDecision?.question
  const decisionUpdatedAt = backend.reviewDecision?.created_at
  const reviewAutoFixAvailable = Boolean(backend.reviewDecision?.options?.includes('auto_fix'))

  const handleResultEntryOpenChange = useCallback((key: string, open: boolean) => setResultOpenByKey((prev) => ({ ...prev, [key]: open })), [])

  return (
    <AiPanelCardShell className="p-3 bg-background space-y-2">
      <MissionHeader liveState={liveState} missionId={missionId} onRefresh={() => void refreshStatus()} onClose={onClose} />

      <JobStatusSection summary={jobStatusSummary} />

      <MissionSummaryCard
        completedFeatureCount={completedFeatureCount}
        featuresCount={features.length}
        failedFeatureCount={failedFeatureCount}
        workerCount={workerEntries.length}
        runningWorkersCount={runningWorkersCount}
        failedWorkersCount={failedWorkersCount}
        resultCount={taskResults.length}
        failedResultCount={failedResultCount}
        lastProgressMessage={lastProgress?.message}
      />

      <MissionActionButtons
        canStart={canStart}
        canResume={canResume}
        canRecover={canRecover}
        canPause={canPause}
        canStop={canStop}
        canAbandon={canAbandon}
        resumeLabel={jobStatusSummary.resumeActionLabel}
        recoverLabel={recoverLabel}
        loading={loading}
        onStart={onStart}
        onPause={onPause}
        onResume={onResume}
        onRecover={onRecover}
        onStop={onCancel}
        onAbandon={onAbandon}
      />

      <RecoverySection
        summary={jobStatusSummary}
        open={recoveryOpen}
        onOpenChange={setRecoveryOpenOverride}
      />

      {error ? <p className="text-xs text-destructive bg-danger-10 rounded px-2 py-1">{error}</p> : null}

      <Layer1ContextPackSection
        layer1Error={backend.layer1Error}
        contextPackError={backend.contextPackError}
        buildingContextPack={buildingContextPack}
        chapterCard={backend.layer1?.chapter_card ?? null}
        recentFacts={backend.layer1?.recent_facts ?? null}
        activeCast={backend.layer1?.active_cast ?? null}
        contextPack={backend.contextPack}
        contextPackStale={contextPackStale}
        onSaveChapterCard={onSaveChapterCard}
        onSaveRecentFacts={onSaveRecentFacts}
        onSaveActiveCast={onSaveActiveCast}
        onCreateDefaultChapterCard={onCreateDefaultChapterCard}
        onInferScopeFromCurrentChapter={onInferScopeFromCurrentChapter}
        onBuildContextPack={onBuildContextPack}
        onFetchLatestContextPack={onFetchLatestContextPack}
      />

      <ReviewSection
        reviewError={backend.reviewError}
        reportLike={reportLike}
        historyLike={historyLike}
        fixInProgress={fixInProgress}
        fixAttempt={fixAttempt}
        fixUpdatedAt={fixUpdatedAt}
        fixMessage={fixMessage}
        waitingDecision={waitingDecision}
        decisionReason={decisionReason}
        decisionUpdatedAt={decisionUpdatedAt}
        onFix={reviewAutoFixAvailable ? () => onAnswerOption('auto_fix') : undefined}
        onDecide={waitingDecision ? scrollToDecision : undefined}
        reviewActionError={reviewActionError}
        reviewActionLoading={reviewActionLoading}
        reviewDecision={backend.reviewDecision}
        missionUiReviewDecision={missionUi?.reviewDecision ?? null}
        onAnswerOption={onAnswerOption}
        reviewDecisionRef={reviewDecisionRef}
      />

      <KnowledgeSection
        projectPath={projectPath}
        missionId={missionId}
        knowledgeLatest={backend.knowledgeLatest}
        knowledgeError={backend.knowledgeError}
        knowledgeTimeline={backend.knowledgeTimeline}
        knowledgeTimelineError={backend.knowledgeTimelineError}
        knowledgeDecisionRequired={knowledgeDecisionRequired}
        onRefresh={() => void refreshStatus()}
      />

      <MacroSection
        projectPath={projectPath}
        missionId={missionId}
        liveState={liveState}
        statusDetail={backend.statusDetail}
        macroState={backend.macroState}
        macroFetchError={backend.macroError}
        loading={loading}
        reviewActionLoading={reviewActionLoading}
        reviewAutoFixAvailable={reviewAutoFixAvailable}
        reviewDecisionRequired={reviewDecisionRequired}
        knowledgeDecisionRequired={knowledgeDecisionRequired}
        onStart={onStart}
        onPause={onPause}
        onResume={onResume}
        onCancel={onCancel}
        onRefresh={() => void refreshStatus()}
        onAutoFix={() => onAnswerOption('auto_fix')}
        onScrollToDecision={scrollToDecision}
      />

      <FeaturesSection
        features={features}
        completedFeatureCount={completedFeatureCount}
        open={featuresOpen}
        onOpenChange={setFeaturesOpen}
      />

      <WorkersSection
        workerEntries={workerEntries}
        issueCountByWorkerId={issueCountByWorkerId}
        open={workersOpen}
        onOpenChange={setWorkersOpenOverride}
      />

      <TaskResultsSection
        taskResults={taskResults}
        open={resultsOpen}
        onOpenChange={setResultsOpenOverride}
        openByKey={resultOpenByKey}
        onEntryOpenChange={handleResultEntryOpenChange}
      />

      <ProgressSection
        progressLog={progressLog}
        open={progressOpen}
        onOpenChange={setProgressOpen}
      />
    </AiPanelCardShell>
  )
})

export const MissionPanel = memo(function MissionPanel(props: MissionPanelProps) {
  return <MissionPanelInner key={props.missionId} {...props} />
})

export default MissionPanel
