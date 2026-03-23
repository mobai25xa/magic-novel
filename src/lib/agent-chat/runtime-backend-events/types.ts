// Agent event envelope shape (mirrors Rust EventEnvelope)

export interface AgentEventEnvelope {
  schema_version: number
  event_id: string
  ts: number
  session_id: string
  turn_id: number
  client_request_id?: string
  source: {
    kind: string
    worker_id?: string
    mission_id?: string
  }
  type: string
  payload: Record<string, unknown>
}

// Mission event envelope shape

export interface MissionEventEnvelope {
  schema_version: number
  event_id: string
  ts: number
  mission_id: string
  type: string
  payload: Record<string, unknown>
}

export type ToolChangeSet = {
  shouldRefreshTree: boolean
  shouldRefreshEditor: boolean
  chapterPath?: string
  projectPath?: string
}

/** Mission event state tracked in-memory (minimal UI in Phase 1) */
export interface MissionUiState {
  missionId: string
  state: string
  currentFeatureId?: string
  workerStatuses: Record<string, { featureId: string; status: string; summary?: string; updatedAt: number }>
  progressLog: Array<{ ts: number; message: string }>
  /** Optional P1: used to trigger UI refresh when Layer1 artifacts change. */
  layer1UpdatedAt?: number
  /** Optional P1: used to trigger UI refresh when ContextPack is rebuilt. */
  contextPackBuiltAt?: number

  /** Optional M3: used to trigger UI refresh when ReviewReport is recorded. */
  reviewUpdatedAt?: number
  /** Optional M3: backend indicates a decision is required to proceed. */
  reviewDecisionRequired?: boolean
  /** Optional M3: last decision payload (when emitted). */
  reviewDecision?: Record<string, unknown> | null

  /** Optional M3: auto-fix loop progress. */
  fixupAttempt?: number
  fixupMessage?: string
  fixupUpdatedAt?: number
  fixupInProgress?: boolean

  /** Optional M4: used to trigger UI refresh when Knowledge Writeback changes. */
  knowledgeUpdatedAt?: number
  /** Optional M4: backend indicates a decision is required to proceed. */
  knowledgeDecisionRequired?: boolean
  /** Optional M4: last decision payload (when emitted). */
  knowledgeDecision?: Record<string, unknown> | null

  /** Optional M5: used to trigger UI refresh when macro workflow state changes. */
  macroStateUpdatedAt?: number
  /** Optional M5: latest macro summary payload (for main chat stream). */
  macroId?: string
  macroCurrentIndex?: number
  macroCurrentStage?: string
  macroChapterCount?: number
  macroCompletedCount?: number
  macroWorkflowKind?: string
  macroLastTransitionAt?: number
  /** Optional M5: last completed chapter ref + summary. */
  macroChapterCompletedRef?: string
  macroChapterCompletedSummary?: string
  macroChapterCompletedAt?: number
}

