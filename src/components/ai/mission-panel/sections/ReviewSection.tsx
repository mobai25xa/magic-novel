import type { RefObject } from 'react'

import { Button } from '@/magic-ui/components'

import type { ReviewReportLike } from '@/components/ai/review-report-card'
import { MissionReviewSection } from '@/components/ai/mission-review-section'

import type { ReviewDecisionPayload } from '../types'

type ReviewSectionProps = {
  reviewError: string | null
  reportLike: ReviewReportLike | null
  historyLike: ReviewReportLike[] | null
  fixInProgress: boolean
  fixAttempt?: number
  fixUpdatedAt?: number
  fixMessage?: string | null
  waitingDecision: boolean
  decisionReason?: string | null
  decisionUpdatedAt?: number
  onFix?: () => void
  onDecide?: () => void
  reviewActionError: string | null
  reviewActionLoading: boolean
  reviewDecision: ReviewDecisionPayload | null
  missionUiReviewDecision: unknown | null
  onAnswerOption: (option: string) => void
  reviewDecisionRef: RefObject<HTMLDivElement | null>
}

type ReviewDecisionOptionsBlockProps = {
  reviewDecision: ReviewDecisionPayload
  reviewActionLoading: boolean
  onAnswerOption: (option: string) => void
  reviewDecisionRef: RefObject<HTMLDivElement | null>
}

function ReviewDecisionOptionsBlock({
  reviewDecision,
  reviewActionLoading,
  onAnswerOption,
  reviewDecisionRef,
}: ReviewDecisionOptionsBlockProps) {
  return (
    <div
      ref={reviewDecisionRef}
      className="rounded-md border border-border/60 bg-warning/5 px-2.5 py-2 text-xs"
    >
      <div className="font-medium text-secondary-foreground">Decision required</div>
      <div className="mt-1 whitespace-pre-wrap text-muted-foreground">{reviewDecision.question}</div>

      {reviewDecision.context_summary?.length ? (
        <div className="mt-2 whitespace-pre-wrap text-muted-foreground">
          {reviewDecision.context_summary.join('\n')}
        </div>
      ) : null}

      {reviewDecision.options?.length ? (
        <div className="mt-2 space-y-2">
          {reviewDecision.options.map((option) => (
            <Button
              key={option}
              type="button"
              size="sm"
              variant="outline"
              className="w-full justify-start text-xs font-medium disabled:opacity-50"
              onClick={() => onAnswerOption(option)}
              disabled={reviewActionLoading}
            >
              {option.replace(/_/g, ' ')}
            </Button>
          ))}
        </div>
      ) : (
        <div className="mt-2 text-muted-foreground">No options provided.</div>
      )}
    </div>
  )
}

type ReviewDecisionFallbackBlockProps = {
  missionUiReviewDecision: unknown
  reviewDecisionRef: RefObject<HTMLDivElement | null>
}

function ReviewDecisionFallbackBlock({ missionUiReviewDecision, reviewDecisionRef }: ReviewDecisionFallbackBlockProps) {
  return (
    <div
      ref={reviewDecisionRef}
      className="rounded-md border border-border/60 bg-warning/5 px-2.5 py-2 text-xs"
    >
      <div className="font-medium text-secondary-foreground">Decision required</div>
      <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded border border-border p-2 text-[11px] text-muted-foreground">
        {JSON.stringify(missionUiReviewDecision, null, 2)}
      </pre>
    </div>
  )
}

type ReviewDecisionBlockProps = {
  reviewDecision: ReviewDecisionPayload | null
  waitingDecision: boolean
  missionUiReviewDecision: unknown | null
  reviewActionLoading: boolean
  onAnswerOption: (option: string) => void
  reviewDecisionRef: RefObject<HTMLDivElement | null>
}

function ReviewDecisionBlock({
  reviewDecision,
  waitingDecision,
  missionUiReviewDecision,
  reviewActionLoading,
  onAnswerOption,
  reviewDecisionRef,
}: ReviewDecisionBlockProps) {
  if (reviewDecision) {
    return (
      <ReviewDecisionOptionsBlock
        reviewDecision={reviewDecision}
        reviewActionLoading={reviewActionLoading}
        onAnswerOption={onAnswerOption}
        reviewDecisionRef={reviewDecisionRef}
      />
    )
  }

  if (waitingDecision && missionUiReviewDecision) {
    return (
      <ReviewDecisionFallbackBlock
        missionUiReviewDecision={missionUiReviewDecision}
        reviewDecisionRef={reviewDecisionRef}
      />
    )
  }

  return null
}

export function ReviewSection({
  reviewError,
  reportLike,
  historyLike,
  fixInProgress,
  fixAttempt,
  fixUpdatedAt,
  fixMessage,
  waitingDecision,
  decisionReason,
  decisionUpdatedAt,
  onFix,
  onDecide,
  reviewActionError,
  reviewActionLoading,
  reviewDecision,
  missionUiReviewDecision,
  onAnswerOption,
  reviewDecisionRef,
}: ReviewSectionProps) {
  return (
    <div className="space-y-2">
      {reviewError ? (
        <p className="text-xs text-muted-foreground">Review unavailable: {reviewError}</p>
      ) : null}

      <MissionReviewSection
        report={reportLike}
        history={historyLike}
        historyMaxItems={5}
        showWhenEmpty
        fixInProgress={fixInProgress}
        fixAttempt={fixAttempt}
        fixMaxAttempts={2}
        fixUpdatedAt={fixUpdatedAt}
        fixMessage={fixMessage ?? undefined}
        waitingDecision={waitingDecision}
        decisionReason={decisionReason ?? undefined}
        decisionUpdatedAt={decisionUpdatedAt}
        onFix={onFix}
        onDecide={onDecide}
      />

      {reviewActionError ? (
        <p className="text-xs text-muted-foreground">Review action failed: {reviewActionError}</p>
      ) : null}

      <ReviewDecisionBlock
        reviewDecision={reviewDecision}
        waitingDecision={waitingDecision}
        missionUiReviewDecision={missionUiReviewDecision}
        reviewActionLoading={reviewActionLoading}
        onAnswerOption={onAnswerOption}
        reviewDecisionRef={reviewDecisionRef}
      />
    </div>
  )
}
