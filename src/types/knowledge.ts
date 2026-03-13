export type KnowledgeItemOp = 'create' | 'update' | 'archive' | 'restore'

export type KnowledgeAcceptPolicy = 'auto_if_pass' | 'manual' | 'orchestrator_only'

export type KnowledgeDecisionActor = 'user' | 'orchestrator'

export type KnowledgeDeltaStatus = 'proposed' | 'accepted' | 'applied' | 'rejected'

export type KnowledgeConflict = {
  type: string
  message: string
  item_id?: string
  target_ref?: string
} & Record<string, unknown>

export type KnowledgeProposalItem = {
  item_id: string
  kind: string
  op: KnowledgeItemOp
  target_ref?: string
  target_revision?: number
  fields: Record<string, unknown>
  evidence_refs: string[]
  source_refs: string[]
  change_reason: string
  accept_policy: KnowledgeAcceptPolicy
} & Record<string, unknown>

export type KnowledgeProposalBundle = {
  schema_version: number
  bundle_id: string
  scope_ref: string
  branch_id?: string
  source_session_id: string
  source_review_id?: string
  generated_at: number
  proposal_items: KnowledgeProposalItem[]
} & Record<string, unknown>

export type KnowledgeDeltaTarget = {
  ref: string
  kind: string
  path?: string
} & Record<string, unknown>

export type KnowledgeDeltaChange = {
  item_id: string
  op: string
  kind: string
  target_ref?: string
  summary: string
} & Record<string, unknown>

export type KnowledgeRollbackInfo = {
  kind: 'soft' | 'hard'
  token?: string
} & Record<string, unknown>

export type KnowledgeDelta = {
  schema_version: number
  knowledge_delta_id: string
  status: KnowledgeDeltaStatus
  scope_ref: string
  branch_id?: string
  source_session_id: string
  source_review_id?: string
  generated_at: number
  targets: KnowledgeDeltaTarget[]
  changes: KnowledgeDeltaChange[]
  evidence_refs: string[]
  conflicts: KnowledgeConflict[]
  accepted_item_ids?: string[]
  rejected_item_ids?: string[]
  applied_at?: number
  rollback?: KnowledgeRollbackInfo
} & Record<string, unknown>

export type KnowledgeDecisionInput = {
  schema_version: number
  bundle_id: string
  delta_id: string
  actor?: KnowledgeDecisionActor
  accepted_item_ids: string[]
  rejected_item_ids: string[]
} & Record<string, unknown>

export type KnowledgeLatest = {
  bundle: KnowledgeProposalBundle | null
  delta: KnowledgeDelta | null
}
