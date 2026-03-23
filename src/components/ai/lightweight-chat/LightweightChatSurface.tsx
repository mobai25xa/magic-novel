import { GitCompareArrows, Loader2, MessageCircleMore, Sparkles } from 'lucide-react'

import { StandardAiModelSelect } from '@/components/ai/shared/StandardAiModelSelect'
import { StandardAiSettingsButton } from '@/components/ai/shared/StandardAiSettingsButton'

import { LightweightChatComposer } from './LightweightChatComposer'
import type { LightweightChatMessage, LightweightChatSurfaceProps } from './lightweight-chat-types'

function ChatBubble(input: {
  message: LightweightChatMessage
  userRoleLabel: string
  assistantRoleLabel: string
  pending?: boolean
}) {
  const isAssistant = input.message.role === 'assistant'

  return (
    <div className={`flex ${isAssistant ? 'justify-start' : 'justify-end'}`}>
      <div
        className={`max-w-[88%] rounded-2xl px-4 py-3 text-sm leading-6 ${
          isAssistant
            ? 'border border-[var(--border-primary)] bg-[var(--bg-panel)]'
            : 'text-white'
        } ${input.pending ? 'opacity-80' : ''}`}
        style={isAssistant
          ? undefined
          : { background: 'linear-gradient(135deg, #0f766e 0%, #14b8a6 100%)' }}
      >
        <div className="mb-1 flex items-center gap-2 text-[11px] uppercase tracking-[0.2em] opacity-60">
          <span>{isAssistant ? input.assistantRoleLabel : input.userRoleLabel}</span>
          {input.pending ? <Loader2 size={12} className="animate-spin" /> : null}
        </div>
        <div className="whitespace-pre-wrap break-words">{input.message.content}</div>
      </div>
    </div>
  )
}

function EmptyState(input: { title: string; description: string }) {
  return (
    <div className="rounded-2xl border border-dashed border-[var(--border-primary)] px-4 py-10 text-center">
      <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-[var(--bg-panel)]">
        <Sparkles size={20} />
      </div>
      <div className="font-medium">{input.title}</div>
      <p className="mt-2 text-sm opacity-75">{input.description}</p>
    </div>
  )
}

export function LightweightChatSurface(input: LightweightChatSurfaceProps) {
  const optimisticUserMessage = input.pendingUserMessage?.trim() || ''
  const hasMessages = input.messages.length > 0 || Boolean(optimisticUserMessage) || Boolean(input.pendingAssistant)

  return (
    <div className="space-y-6">
      {(input.title || input.description || input.statusBadge) ? (
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            {input.title ? <h2 className="text-xl font-semibold">{input.title}</h2> : null}
            {input.description ? <p className="text-sm opacity-75">{input.description}</p> : null}
          </div>
          {input.statusBadge}
        </div>
      ) : null}

      <div className="chat-input-shell">
        <div className="chat-input-footer border-t-0">
          <div className="chat-input-footer-left min-w-0 flex-1 flex-wrap gap-2">
            <div className="chat-input-model-wrap flex min-w-0 items-center gap-2">
              <StandardAiModelSelect
                models={input.availableModels}
                selectedModel={input.selectedModel}
                onSelectModel={input.onSelectModel}
                disabled={input.modelsDisabled || input.availableModels.length === 0}
                size="sm"
                variant="ghost"
                width="auto"
                className="chat-input-model-selector"
                icon={<GitCompareArrows className="h-[13px] w-[13px]" />}
              />
            </div>

            {input.toolbarActions ? (
              <div className="flex flex-wrap items-center gap-2">
                {input.toolbarActions}
              </div>
            ) : null}
          </div>

          <div className="chat-input-footer-right flex-wrap gap-2">
            <StandardAiSettingsButton
              label={input.labels.openSettingsLabel}
              onClick={input.onOpenSettings}
              variant={input.settingsButtonVariant}
            />
          </div>
        </div>
      </div>

      <div className={input.sidebar ? 'grid gap-4 xl:grid-cols-[minmax(0,1.35fr)_minmax(280px,0.9fr)]' : 'space-y-4'}>
        <div className="space-y-4 rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <MessageCircleMore size={16} />
              <span>{input.labels.messagesTitle}</span>
            </div>
            {input.messageHeaderActions}
          </div>

          {input.error ? (
            <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 px-4 py-4 text-sm">
              {input.errorTitle ? <div className="font-medium">{input.errorTitle}</div> : null}
              <div className={input.errorTitle ? 'mt-1 opacity-80' : 'opacity-80'}>{input.error}</div>
              {input.errorActions ? (
                <div className="mt-3 flex flex-wrap gap-3">
                  {input.errorActions}
                </div>
              ) : null}
            </div>
          ) : null}

          <div className="max-h-[560px] space-y-3 overflow-x-hidden overflow-y-auto rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] p-3">
            {hasMessages ? (
              <>
                {input.messages.map((message) => (
                  <ChatBubble
                    key={message.id}
                    message={message}
                    userRoleLabel={input.labels.userRole}
                    assistantRoleLabel={input.labels.assistantRole}
                  />
                ))}

                {optimisticUserMessage ? (
                  <ChatBubble
                    message={{
                      id: '__pending_user__',
                      role: 'user',
                      content: optimisticUserMessage,
                    }}
                    userRoleLabel={input.labels.userRole}
                    assistantRoleLabel={input.labels.assistantRole}
                    pending
                  />
                ) : null}

                {input.pendingAssistant ? (
                  <ChatBubble
                    message={{
                      id: '__pending_assistant__',
                      role: 'assistant',
                      content: typeof input.pendingAssistant.content === 'string'
                        && input.pendingAssistant.content.length > 0
                        ? input.pendingAssistant.content
                        : input.pendingAssistant.label,
                    }}
                    userRoleLabel={input.labels.userRole}
                    assistantRoleLabel={input.labels.assistantRole}
                    pending
                  />
                ) : null}
              </>
            ) : (
              <EmptyState
                title={input.labels.emptyTitle}
                description={input.labels.emptyDescription}
              />
            )}
          </div>

          <LightweightChatComposer
            inputValue={input.inputValue}
            onInputChange={input.onInputChange}
            onSend={input.onSend}
            inputPlaceholder={input.labels.inputPlaceholder}
            inputDisabled={input.inputDisabled}
            sendDisabled={input.sendDisabled}
            sendLabel={input.labels.sendLabel}
            footerActions={input.composerActions}
            pending={input.composerPending}
          />
        </div>

        {input.sidebar ? (
          <div className="space-y-4">
            {input.sidebar}
          </div>
        ) : null}
      </div>
    </div>
  )
}
