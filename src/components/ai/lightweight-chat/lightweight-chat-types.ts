import type { ReactNode } from 'react'

import type { ButtonProps } from '@/magic-ui/components'

export type LightweightChatMessageRole = 'user' | 'assistant'

export interface LightweightChatMessage {
  id: string
  role: LightweightChatMessageRole
  content: string
}

export interface LightweightChatPendingAssistant {
  label: string
  content?: string
}

export interface LightweightChatSurfaceLabels {
  modelLabel: string
  messagesTitle: string
  assistantRole: string
  userRole: string
  emptyTitle: string
  emptyDescription: string
  inputPlaceholder: string
  sendLabel: string
  openSettingsLabel: string
}

export interface LightweightChatSurfaceProps {
  title?: string
  description?: string
  statusBadge?: ReactNode
  toolbarActions?: ReactNode
  messageHeaderActions?: ReactNode
  sidebar?: ReactNode
  composerActions?: ReactNode
  error?: ReactNode
  errorTitle?: string
  errorActions?: ReactNode
  messages: LightweightChatMessage[]
  pendingUserMessage?: string | null
  pendingAssistant?: LightweightChatPendingAssistant | null
  availableModels: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  modelsDisabled?: boolean
  settingsButtonVariant?: ButtonProps['variant']
  onOpenSettings: () => void
  inputValue: string
  onInputChange: (value: string) => void
  onSend: () => void | Promise<void>
  inputDisabled?: boolean
  sendDisabled?: boolean
  composerPending?: boolean
  labels: LightweightChatSurfaceLabels
}
