import type {
  AgentLlmMessage,
  AgentLoopStopReason,
  AgentStateStatus,
  AgentToolTrace,
  AgentUiMessage,
} from '@/agent/types'
import type {
  ApprovalMode,
  CapabilityMode,
  ClarificationMode,
} from '@/platform/tauri/clients/agent-engine-client'

export type AgentToolTraceStage = NonNullable<AgentToolTrace['stage']>

export type ChatRole = AgentUiMessage['role']
export type ChatUiMessage = AgentUiMessage
export type ChatToolTrace = AgentToolTrace
export type OpenAiMessage = AgentLlmMessage

export type AgentRuntimeStatus = AgentStateStatus
export type AgentRuntimeStopReason = AgentLoopStopReason

export type RunChatTurnOptions = {
  approvalMode?: ApprovalMode
  capabilityMode?: CapabilityMode
  clarificationMode?: ClarificationMode
}

export type AgentUiTurnPhase =
  | 'queued'
  | 'planning'
  | 'tool_running'
  | 'synthesizing'
  | 'compacting'
  | 'completed'
  | 'cancelled'
  | 'failed'

export type AgentUiEventType =
  | 'TURN_STARTED'
  | 'PLAN_STARTED'
  | 'WORKER_STARTED'
  | 'WORKER_COMPLETED'
  | 'STREAMING_STARTED'
  | 'ASSISTANT_TEXT_DELTA'
  | 'THINKING_TEXT_DELTA'
  | 'TOOL_CALL_STARTED'
  | 'TOOL_CALL_PROGRESS'
  | 'TOOL_CALL_FINISHED'
  | 'WAITING_FOR_CONFIRMATION'
  | 'ASKUSER_REQUESTED'
  | 'ASKUSER_ANSWERED'
  | 'SYNTHESIS_STARTED'
  | 'COMPACTION_STARTED'
  | 'COMPACTION_FINISHED'
  | 'COMPACTION_FALLBACK'
  | 'TURN_COMPLETED'
  | 'TURN_CANCELLED'
  | 'TURN_FAILED'

export interface AgentUiTimelineEvent {
  id: string
  turn: number
  type: AgentUiEventType
  ts: number
  seq: number
  summary?: string
  callId?: string
  delta?: string
  meta?: Record<string, unknown>
}

export interface AgentUiTurnError {
  /** Backend error code */
  code: string
  /** Original error message */
  message: string
  /** Structured detail from TURN_FAILED.error_detail */
  detail?: {
    provider?: string
    model?: string
    retryable?: boolean
    diagnostic?: string
    http_status?: number
    retry_after_ms?: number
    category_hint?: string
    tool_name?: string
    schema_path?: string
    policy_source?: string
    capability_preset?: string
    exposure_reason?: string
    turn_failed_classification?: string
    provider_schema_error?: boolean
    provider_400_error?: boolean
    missing_tool_escalation?: boolean
    tool_call_count?: number
    rounds_executed?: number
    exposed_tools?: string[]
    skipped_tools?: Array<{
      tool_name?: string
      error?: string
    }>
  }
}

export interface AgentUiTurnState {
  turn: number
  phase: AgentUiTurnPhase
  startedAt: number
  updatedAt: number
  finishedAt?: number
  stopReason?: AgentLoopStopReason
  error?: string
  turnError?: AgentUiTurnError
}

export interface AgentUiToolStep {
  callId: string
  llmCallId?: string
  toolName: string
  status: 'running' | 'success' | 'error' | 'waiting_confirmation' | 'cancelled'
  startedAt: number
  finishedAt?: number
  durationMs?: number

  argsSummary?: string
  resultSummary?: string
  progress?: string

  inputPreview?: unknown
  outputPreview?: unknown
  rawOutput?: string

  retryable?: boolean
  errorMessage?: string
  errorCode?: string
  faultDomain?: AgentToolTrace['fault_domain']
  stage?: AgentToolTrace['stage']
  revisionBefore?: number
  revisionAfter?: number
  txId?: string

  summary?: string
}

export type FeedbackRating = 'unset' | 'positive' | 'negative'

export interface AgentUiTurnView {
  state: AgentUiTurnState
  userText: string
  answerText: string
  thinkingText: string
  toolSteps: AgentUiToolStep[]
  events: AgentUiTimelineEvent[]
}
