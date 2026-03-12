import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'
import { AiToolContent } from '@/magic-ui/components'

import { parseToolOutput } from '../tool-view-utils'
import { GenericToolView } from './GenericToolView'
import { ReviewReportCard, type ReviewReportLike } from '../../review-report-card'

type ReviewToolViewProps = {
  step: AgentUiToolStep
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function coerceReviewReport(value: unknown): ReviewReportLike | null {
  const direct = asRecord(value)
  if (!direct) {
    return null
  }

  if (typeof direct.overall_status === 'string') {
    return direct as unknown as ReviewReportLike
  }

  const data = asRecord(direct.data)
  if (data && typeof data.overall_status === 'string') {
    return data as unknown as ReviewReportLike
  }

  const result = asRecord(direct.result)
  const preview = asRecord(result?.preview)
  if (preview && typeof preview.overall_status === 'string') {
    return preview as unknown as ReviewReportLike
  }

  return null
}

export function ReviewToolView({ step }: ReviewToolViewProps) {
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const report = useMemo(
    () => coerceReviewReport(parsed ?? step.outputPreview),
    [parsed, step.outputPreview],
  )

  if (!report) {
    return <GenericToolView step={step} />
  }

  return (
    <AiToolContent className="space-y-2">
      <ReviewReportCard report={report} />
    </AiToolContent>
  )
}
