import { useMemo, useState } from 'react'
import { Bot, CircleHelp, Compass, History, Lock, LockOpen, RefreshCw, Sparkles, X } from 'lucide-react'

import { ChatInput } from '@/components/ai/input/ChatInput'
import { ActionBar } from '@/components/ai/input/ActionBar'
import { TurnCardAssistantBlock } from '@/components/ai/message/turn-card-assistant-block'
import { TurnCardUserBlock } from '@/components/ai/message/turn-card-user-block'
import { useChatTranscriptScroll } from '@/components/ai/panel/agent-chat-panel-scroll'
import { AgentChatPanelViewScrollJump } from '@/components/ai/panel/view/agent-chat-panel-view-scroll'
import type { OpenQuestionStatus } from '@/features/inspiration/types'
import { useTranslation } from '@/hooks/use-translation'
import { Badge, Button, Tabs, Tab, TabPanel, Tag } from '@/magic-ui/components'
import { cn } from '@/lib/utils'
import { useSettingsStore } from '@/state/settings'

import {
  CONSENSUS_FIELD_IDS,
  getConsensusField,
  toConsensusItems,
} from './inspiration-helpers'
import { InspirationSessionPanel } from './InspirationSessionPanel'
import type { useInspirationWorkflow } from './use-inspiration-workflow'

type InspirationWorkspaceViewModel = ReturnType<typeof useInspirationWorkflow>

interface InspirationWorkspaceProps {
  data: InspirationWorkspaceViewModel
  preserveInspirationSession: boolean
  setPreserveInspirationSession: (value: boolean) => void
  onGenerateVariants: () => void | Promise<void>
  onSkipToCreateForm: () => void
}

function resolveStatusVariant(status: OpenQuestionStatus) {
  switch (status) {
    case 'resolved':
      return 'success'
    case 'dismissed':
      return 'secondary'
    default:
      return 'warning'
  }
}

export function InspirationWorkspace(input: InspirationWorkspaceProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const [mobileTab, setMobileTab] = useState<'consensus' | 'chat'>('chat')
  const [dismissedMissingFieldsKey, setDismissedMissingFieldsKey] = useState<string | null>(null)

  const fieldLabels = useMemo(() => ({
    story_core: cp.inspirationFieldStoryCore,
    premise: cp.inspirationFieldPremise,
    genre_tone: cp.inspirationFieldGenreTone,
    protagonist: cp.inspirationFieldProtagonist,
    worldview: cp.inspirationFieldWorldview,
    core_conflict: cp.inspirationFieldConflict,
    selling_points: cp.inspirationFieldSellingPoints,
    audience: cp.inspirationFieldAudience,
    ending_direction: cp.inspirationFieldEnding,
  }), [cp])

  const requiredMissingSet = new Set(input.data.missingRequiredFields)
  const missingFieldsKey = input.data.missingRequiredFields.join('|')
  const missingBannerDismissed = dismissedMissingFieldsKey === missingFieldsKey

  const consensusPanel = (
    <div className="min-w-0 space-y-4">
      <div className="overflow-hidden rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Compass size={16} />
              <span>{cp.inspirationConsensusTitle}</span>
            </div>
            <p className="mt-2 break-words text-xs opacity-70">{cp.inspirationConsensusDescription}</p>
          </div>
          <Badge variant="soft" color="default">
            {input.data.fieldsWithContentCount}/{CONSENSUS_FIELD_IDS.length}
          </Badge>
        </div>

        <div className="mt-4 space-y-3">
          {CONSENSUS_FIELD_IDS.map((fieldId) => {
            const field = getConsensusField(input.data.consensus, fieldId)
            const draftItems = toConsensusItems(field.draft_value)
            const confirmedItems = toConsensusItems(field.confirmed_value)

            return (
              <div key={fieldId} className="overflow-hidden rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] p-3">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <div className="text-sm font-medium">{fieldLabels[fieldId]}</div>
                      {requiredMissingSet.has(fieldId) ? (
                        <Tag size="sm" variant="outline-warning">{cp.inspirationRequiredField}</Tag>
                      ) : null}
                      {field.locked ? (
                        <Tag size="sm" variant="outline-info">{cp.inspirationLocked}</Tag>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    {field.draft_value ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={input.data.runningTurn}
                        onClick={() => input.data.confirmConsensusField(fieldId)}
                      >
                        {cp.inspirationConfirmDraft}
                      </Button>
                    ) : null}
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={input.data.runningTurn}
                      onClick={() => input.data.toggleConsensusLock(fieldId)}
                    >
                      {field.locked ? <LockOpen size={14} /> : <Lock size={14} />}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={input.data.runningTurn}
                      onClick={() => input.data.clearConsensusField(fieldId)}
                    >
                      {translations.common.delete}
                    </Button>
                  </div>
                </div>

                {confirmedItems.length > 0 ? (
                  <div className="mt-3 space-y-2">
                    <div className="text-[11px] uppercase tracking-[0.18em] opacity-60">
                      {cp.inspirationConfirmedLabel}
                    </div>
                    <ConsensusValueList items={confirmedItems} tone="confirmed" />
                  </div>
                ) : null}

                {draftItems.length > 0 ? (
                  <div className="mt-3 space-y-2">
                    <div className="text-[11px] uppercase tracking-[0.18em] opacity-60">
                      {cp.inspirationDraftLabel}
                    </div>
                    <ConsensusValueList items={draftItems} tone="draft" />
                  </div>
                ) : null}

                {!field.draft_value && !field.confirmed_value ? (
                  <p className="mt-3 text-sm opacity-60">{cp.inspirationEmptyField}</p>
                ) : null}
              </div>
            )
          })}
        </div>
      </div>

      <div className="overflow-hidden rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex items-center gap-2 text-sm font-semibold">
          <CircleHelp size={16} />
          <span>{cp.inspirationOpenQuestionsTitle}</span>
        </div>
        <div className="mt-4 space-y-3">
          {input.data.openQuestions.length > 0 ? input.data.openQuestions.map((question) => (
            <div key={question.question_id} className="overflow-hidden rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] p-3">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1 space-y-2">
                  <div className="break-words text-sm">{question.question}</div>
                  <div className="flex flex-wrap gap-2">
                    <Tag size="sm" variant={resolveStatusVariant(question.status)}>{question.status}</Tag>
                    <Tag size="sm" variant="outline">{question.importance}</Tag>
                  </div>
                </div>
                {question.status === 'open' ? (
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => input.data.updateOpenQuestionStatus(question.question_id, 'resolved')}
                    >
                      {cp.inspirationResolveQuestion}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => input.data.updateOpenQuestionStatus(question.question_id, 'dismissed')}
                    >
                      {cp.inspirationDismissQuestion}
                    </Button>
                  </div>
                ) : null}
              </div>
            </div>
          )) : (
            <p className="text-sm opacity-70">{cp.inspirationNoOpenQuestions}</p>
          )}
        </div>
      </div>
    </div>
  )

  return (
    <div className="flex flex-1 min-h-0 flex-col gap-4">
      <div className="block space-y-4 xl:hidden">
        <WorkspaceHeaderCard input={input} />
        {!missingBannerDismissed && input.data.missingRequiredFields.length > 0 ? (
          <MissingFieldsBanner
            title={cp.inspirationMissingFieldsTitle}
            value={input.data.missingRequiredFields.map((fieldId) => fieldLabels[fieldId]).join(' / ')}
            closeLabel={translations.common.close}
            onClose={() => setDismissedMissingFieldsKey(missingFieldsKey)}
          />
        ) : null}

        <Tabs value={mobileTab} onValueChange={(value) => setMobileTab(value as 'consensus' | 'chat')}>
          <Tab value="chat">{cp.inspirationChatTab}</Tab>
          <Tab value="consensus">{cp.inspirationConsensusTab}</Tab>
          <TabPanel value="chat" className="mt-4">
            <ChatPanel input={input} />
          </TabPanel>
          <TabPanel value="consensus" className="mt-4">
            {consensusPanel}
          </TabPanel>
        </Tabs>
      </div>

      <div className="hidden min-h-0 flex-1 gap-5 xl:grid xl:grid-cols-[minmax(340px,420px)_minmax(0,1fr)] 2xl:grid-cols-[minmax(360px,460px)_minmax(0,1fr)]">
        <div className="min-w-0 min-h-0 overflow-hidden">
          <div className="h-full editor-shell-outline-scroll !p-0">
            <div className="space-y-4">
              <WorkspaceHeaderCard input={input} />
              {!missingBannerDismissed && input.data.missingRequiredFields.length > 0 ? (
                <MissingFieldsBanner
                  title={cp.inspirationMissingFieldsTitle}
                  value={input.data.missingRequiredFields.map((fieldId) => fieldLabels[fieldId]).join(' / ')}
                  closeLabel={translations.common.close}
                  onClose={() => setDismissedMissingFieldsKey(missingFieldsKey)}
                />
              ) : null}
              {consensusPanel}
            </div>
          </div>
        </div>
        <div className="min-w-0 min-h-0 overflow-hidden">
          <ChatPanel input={input} />
        </div>
      </div>
    </div>
  )
}

function WorkspaceHeaderCard({ input }: { input: InspirationWorkspaceProps }) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  return (
    <div className="flex flex-wrap items-start justify-between gap-3 rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.2em] opacity-60">
          <Sparkles size={14} />
          <span>{cp.inspirationStageLabel}</span>
        </div>
        <div className="text-lg font-semibold">{cp.inspirationTitle}</div>
        <p className="max-w-2xl text-sm opacity-75">{cp.inspirationSubtitle}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Badge
          variant={input.data.runtimeState === 'running' ? 'soft' : 'outline'}
          color={input.data.runtimeState === 'running' ? 'primary' : 'default'}
        >
          {input.data.runtimeState === 'running' ? cp.inspirationStatusRunning : cp.inspirationStatusReady}
        </Badge>
        <Button variant="outline" onClick={() => input.data.loadSnapshot()} disabled={input.data.loadingSession}>
          <RefreshCw size={14} className={cn(input.data.loadingSession && 'animate-spin')} />
          {translations.common.retry}
        </Button>
        <Button
          onClick={() => {
            void input.onGenerateVariants()
          }}
          disabled={input.data.generatingVariants || input.data.runningTurn || input.data.missingRequiredFields.length > 0}
        >
          {input.data.generatingVariants ? cp.inspirationGeneratingVariants : cp.inspirationGenerateVariants}
        </Button>
        <Button variant="ghost" onClick={input.onSkipToCreateForm}>
          {cp.inspirationSkipToForm}
        </Button>
      </div>
    </div>
  )
}

function MissingFieldsBanner(input: {
  title: string
  value: string
  closeLabel: string
  onClose: () => void
}) {
  return (
    <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="font-medium">{input.title}</div>
          <div className="mt-1 break-words opacity-80">
            {input.value}
          </div>
        </div>
        <Button
          size="sm"
          variant="ghost"
          className="shrink-0"
          aria-label={input.closeLabel}
          onClick={input.onClose}
        >
          <X size={14} />
        </Button>
      </div>
    </div>
  )
}

function ChatPanel({ input }: { input: InspirationWorkspaceProps }) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const approvalMode = useSettingsStore((state) => state.approvalMode)
  const [sessionPanelOpen, setSessionPanelOpen] = useState(false)
  const sendDisabled = input.data.loadingSession || input.data.runningTurn || !input.data.chatInput.trim()
  const transcriptItems = useMemo(() => [
    ...input.data.messages.map((message) => ({
      key: message.id,
      role: message.role,
      content: message.content,
      pending: false,
    })),
    ...(input.data.pendingUserMessage
      ? [{
          key: '__pending_user__',
          role: 'user' as const,
          content: input.data.pendingUserMessage,
          pending: true,
        }]
      : []),
    ...(input.data.pendingAssistant
      ? [{
          key: '__pending_assistant__',
          role: 'assistant' as const,
          content: input.data.pendingAssistant.content || input.data.pendingAssistant.label,
          pending: true,
        }]
      : []),
  ], [
    input.data.messages,
    input.data.pendingAssistant,
    input.data.pendingUserMessage,
  ])
  const latestTranscriptSignature = useMemo(() => {
    const lastItem = transcriptItems[transcriptItems.length - 1]

    return [
      input.data.sessionId,
      input.data.runtimeState,
      transcriptItems.length,
      lastItem?.key ?? '',
      lastItem?.role ?? '',
      lastItem?.pending ? 'pending' : 'settled',
      lastItem?.content.length ?? 0,
    ].join('|')
  }, [input.data.runtimeState, input.data.sessionId, transcriptItems])
  const {
    scrollRef,
    scrollState,
    handleScroll,
    jumpToLatest,
  } = useChatTranscriptScroll({
    contentSignature: latestTranscriptSignature,
    itemCount: transcriptItems.length,
    streaming: input.data.runningTurn,
    sessionKey: input.data.sessionId,
  })

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col gap-4 rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-semibold">
          <Bot size={16} />
          <span>{cp.inspirationMessagesTitle}</span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline" color="default">{cp.inspirationAssistantRole}</Badge>
          <Button
            size="sm"
            variant="outline"
            disabled={input.data.loadingSession}
            onClick={() => setSessionPanelOpen(true)}
          >
            <History size={14} className="mr-1.5" />
            {cp.inspirationSessionEntry}
          </Button>
        </div>
      </div>

      {input.data.chatError ? (
        <div className="rounded-2xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-sm">
          {input.data.chatError}
        </div>
      ) : null}

      <div
        ref={scrollRef}
        className="editor-shell-ai-scroll min-h-0 flex-1 overflow-x-hidden rounded-2xl border border-[var(--border-primary)]"
        onScroll={handleScroll}
      >
        {transcriptItems.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-[var(--border-primary)] px-4 py-10 text-center">
            <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-[var(--bg-panel)]">
              <Sparkles size={20} />
            </div>
            <div className="font-medium">{cp.inspirationEmptyTitle}</div>
            <p className="mt-2 text-sm opacity-75">{cp.inspirationEmptyDescription}</p>
          </div>
        ) : (
          transcriptItems.map((item, index) => (
            item.role === 'user' ? (
              <TurnCardUserBlock
                key={item.key}
                userText={item.content}
              />
            ) : (
              <TurnCardAssistantBlock
                key={item.key}
                assistantText={item.content}
                elapsedLabel=""
                loading={item.pending}
                streaming={item.pending}
                turnId={index + 1}
                feedbackRating="unset"
                onRate={() => {}}
                label={cp.inspirationAssistantRole}
                showElapsedLabel={false}
                showFooterActions={false}
              />
            )
          ))
        )}

        <AgentChatPanelViewScrollJump
          autoScrollLocked={scrollState.autoScrollLocked}
          unseenCount={scrollState.unseenCount}
          onJump={jumpToLatest}
        />
      </div>

      <div className="space-y-2">
        <div className="chat-input-shell" data-disabled={input.data.loadingSession ? 'true' : 'false'}>
          <ChatInput
            value={input.data.chatInput}
            onChange={input.data.setChatInput}
            onSend={() => {
              void input.data.sendMessage()
            }}
            disabled={input.data.loadingSession}
            placeholder={cp.inspirationInputPlaceholder}
          />

          <ActionBar
            models={input.data.availableModels}
            selectedModel={input.data.selectedModel}
            onSelectModel={input.data.onSelectModel}
            approvalMode={approvalMode}
            disabled={input.data.loadingSession}
            modelsDisabled={input.data.loadingSession || input.data.availableModels.length === 0 || input.data.runningTurn}
            running={input.data.runningTurn}
            inputEmpty={!input.data.chatInput.trim()}
            canContinue={!input.data.loadingSession}
            showAttachButton={false}
            showApprovalBadge={false}
            onToggleRun={() => {
              if (input.data.runningTurn) {
                void input.data.cancelTurn()
                return
              }
              if (sendDisabled) {
                return
              }
              void input.data.sendMessage()
            }}
          />
        </div>

        <div className="text-xs opacity-60">
          {cp.inspirationComposerHint}
        </div>
      </div>

      <InspirationSessionPanel
        open={sessionPanelOpen}
        onOpenChange={setSessionPanelOpen}
        sessionId={input.data.sessionId}
        sessionList={input.data.sessionList}
        loadingSession={input.data.loadingSession}
        loadingSessionList={input.data.loadingSessionList}
        sessionListError={input.data.sessionListError}
        runningTurn={input.data.runningTurn}
        preserveInspirationSession={input.preserveInspirationSession}
        setPreserveInspirationSession={input.setPreserveInspirationSession}
        loadSessionList={input.data.loadSessionList}
        openSession={input.data.openSession}
        newSession={input.data.newSession}
        renameSession={input.data.renameSession}
        deleteSession={input.data.deleteSession}
      />
    </div>
  )
}

function ConsensusValueList(input: { items: string[]; tone: 'confirmed' | 'draft' }) {
  return (
    <div className="space-y-2">
      {input.items.map((item) => (
        <div
          key={`${input.tone}_${item}`}
          className={cn(
            'max-w-full rounded-2xl border px-3 py-2 text-sm leading-6 whitespace-pre-wrap break-words',
            input.tone === 'confirmed'
              ? 'border-emerald-500/20 bg-emerald-500/10 text-[var(--text-main)]'
              : 'border-sky-500/20 bg-sky-500/10 text-[var(--text-main)]',
          )}
        >
          {item}
        </div>
      ))}
    </div>
  )
}
