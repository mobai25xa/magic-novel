export type ReviewType =
  | 'word_count'
  | 'continuity'
  | 'logic'
  | 'character'
  | 'style'
  | 'terminology'
  | 'foreshadow'
  | 'objective_completion'

export type ReviewOverallStatus = 'pass' | 'warn' | 'block'

export type ReviewIssueSeverity = 'info' | 'warn' | 'block'

export type ReviewIssueConfidence = 'low' | 'medium' | 'high'

export type ReviewRecommendedAction = 'accept' | 'revise' | 'escalate'

export type ReviewIssue = {
  issue_id: string
  review_type: ReviewType
  severity: ReviewIssueSeverity
  summary: string
  subject_refs: string[]
  evidence_refs: string[]
  confidence: ReviewIssueConfidence
  suggested_fix?: string
  auto_fixable: boolean
} & Record<string, unknown>

export type ReviewReport = {
  schema_version: number
  review_id: string
  scope_ref: string
  target_refs: string[]
  review_types: ReviewType[]
  overall_status: ReviewOverallStatus
  issues: ReviewIssue[]
  evidence_summary: string[]
  recommended_action: ReviewRecommendedAction
  generated_at: number
} & Record<string, unknown>

export type ReviewDecisionRequest = {
  schema_version: number
  review_id: string
  feature_id?: string | null
  scope_ref: string
  target_refs?: string[] | null
  question: string
  options: string[]
  context_summary: string[]
  created_at: number
} & Record<string, unknown>

export type ReviewDecisionAnswer = {
  schema_version: number
  review_id: string
  selected_option: string
  note?: string | null
  answered_at: number
} & Record<string, unknown>
