import type { AgentSessionMeta } from '@/lib/agent-chat/session'
import type { AiChatViewMode, ApprovalMode, CapabilityMode } from '@/state/settings'

import type { AgentPanelError } from '../../agent-chat-panel-utils'
import type { ChatContext } from '../../input/chat-context-types'
import type { AgentChatPanelHydrationStatus, AgentChatPanelRuntimeState } from '../agent-chat-panel-state'

export type AgentChatPanelViewProps = {
  sessionId: string
  turnIds: number[]
  latestTurnSignature: string
  input: string
  running: boolean
  lastError: AgentPanelError | null
  sessionError: string | null
  wasSessionResumed: boolean
  sessionRuntimeState: AgentChatPanelRuntimeState
  sessionHydrationStatus?: AgentChatPanelHydrationStatus
  sessionCanContinue: boolean
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  sessionWarnings: string[]
  historyStateBySessionId: Record<string, string>
  models: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  approvalMode: ApprovalMode
  capabilityMode: CapabilityMode
  viewMode: AiChatViewMode
  onSelectViewMode: (mode: AiChatViewMode) => void
  inputDisabled?: boolean
  inputPlaceholder?: string
  onInputChange: (value: string) => void
  onSend: () => Promise<void>
  onCancel: () => void
  canStartNewSession?: boolean
  onStartNewSession: () => Promise<void>
  onOpenHistoryPage: () => void
  onCloseHistoryPage: () => void
  historyPageOpen: boolean
  onResumeSession: (sessionId: string) => Promise<void>
  onRenameSession: (sessionId: string, title: string) => Promise<void>
  onDeleteSession: (sessionId: string) => Promise<void>
  historyEnabled: boolean
  sessionLoading: boolean
  sessionList: AgentSessionMeta[]
  currentSessionId?: string
  onRetryStep: (turnId: number, callId: string) => void
  contexts: ChatContext[]
  onAddContext: (context: ChatContext) => void
  onRemoveContext: (contextId: string) => void
  elapsedTime?: string
  showTimer?: boolean
  projectPath: string
  missionId: string
  onClosePanel?: () => void
}
