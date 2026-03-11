import type { ReviewRecommendedAction } from './review-summary-row'
import type { ReviewReportLike } from './review-report-card'

export type ReviewOverallStatus = 'pass' | 'warn' | 'block' | 'unknown'

export function normalizeReviewStatus(value: unknown): ReviewOverallStatus {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'pass') return 'pass'
  if (normalized === 'warn' || normalized === 'warning') return 'warn'
  if (normalized === 'block' || normalized === 'failed' || normalized === 'error') return 'block'
  return 'unknown'
}

export function normalizeReviewRecommendedAction(value: unknown): ReviewRecommendedAction | undefined {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'accept' || normalized === 'revise' || normalized === 'escalate') {
    return normalized
  }
  return undefined
}

export function countReviewWarnings(report: ReviewReportLike | null | undefined): number {
  const issues = Array.isArray(report?.issues) ? report!.issues : []
  return issues.filter((issue) => {
    const sev = String(issue.severity ?? '').trim().toLowerCase()
    return sev === 'warn' || sev === 'warning'
  }).length
}
