import type { FaultDomain } from '@/lib/tool-gateway/types'

export type AgentLoopStopReason = 'success' | 'cancel' | 'error' | 'limit'

export type AgentCompactionStrategy = 'threshold' | 'context_limit'

export interface AgentCompactionMeta {
  strategy: AgentCompactionStrategy
  summary_text: string
  anchors: string[]
  removed_count: number
  keep_recent_count: number
  source_window: {
    start_index: number
    end_index: number
  }
  anchor?: {
    anchor_index: number
    anchor_preview?: string
  }
  reason?: string
}

export interface AgentSessionReminder {
  kind: 'new_session' | 'resumed_session'
  payload: Record<string, unknown>
}

export type AgentStateStatus =
  | 'idle'
  | 'thinking'
  | 'waiting_confirmation'
  | 'waiting_askuser'
  | 'executing_tool'
  | 'compacting'

export type AgentTodoStatus = 'pending' | 'in_progress' | 'completed'

export interface AgentTodoItem {
  status: AgentTodoStatus
  text: string
}

export interface AgentTodoState {
  items: AgentTodoItem[]
  lastUpdatedAt: number
  sourceCallId?: string
}

export function createEmptyTodoState(): AgentTodoState {
  return {
    items: [],
    lastUpdatedAt: 0,
    sourceCallId: undefined,
  }
}

export interface AgentAskUserQuestion {
  index: number
  question: string
  topic: string
  options: string[]
}

export interface AgentAskUserAnswer {
  topic: string
  value: string
}

export interface AgentPendingAskUserRequest {
  callId: string
  turn: number
  /** Legacy DSL string — may be empty when structured questions are used */
  questionnaire: string
  questions: AgentAskUserQuestion[]
  requestedAt: number
}

export interface AgentToolTrace {
  turn: number
  call_id: string
  tool_name: string
  status: 'ok' | 'error'
  fault_domain?: FaultDomain
  error_code?: string
  error_message?: string
  duration_ms: number
  stage?: 'policy' | 'execute' | 'result'
  revision_before?: number
  revision_after?: number
  tx_id?: string
  preview?: Record<string, unknown>
  refs?: {
    path?: string
    entity_id?: string
    snapshot_id?: string
    changed_block_ids?: string[]
  }
}

export interface AgentUiMessage {
  id: string
  role: 'user' | 'assistant' | 'tool' | 'system'
  content: string
  ts: number
  turn?: number
  tool_name?: string
  tool_call_id?: string
}

export interface AgentLlmMessage {
  role: 'system' | 'user' | 'assistant' | 'tool'
  content?: string | null
  tool_calls?: unknown[]
  tool_call_id?: string
  name?: string
}

export interface AgentEditorState {
  /** 用户选中的文本（可能跨段落） */
  selectedText?: string
  /** 光标所在段落的完整文本 */
  cursorParagraph?: string
  /** 光标所在段落的索引（从 0 开始） */
  cursorParagraphIndex?: number
  /** 当前章节总段落数 */
  totalParagraphs?: number
}

