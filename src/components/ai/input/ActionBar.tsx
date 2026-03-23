import { GitCompareArrows, SendHorizonal, Square } from 'lucide-react'

import { Badge } from '@/magic-ui/components'
import type { ApprovalMode } from '@/state/settings'
import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'
import { StandardAiModelSelect } from '../shared/StandardAiModelSelect'

type ActionBarProps = {
  models: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  approvalMode: ApprovalMode
  disabled?: boolean
  modelsDisabled?: boolean
  running: boolean
  inputEmpty: boolean
  canContinue?: boolean
  onAttach?: () => void
  showAttachButton?: boolean
  showApprovalBadge?: boolean
  onToggleRun: () => void
}

export function ActionBar(input: ActionBarProps) {
  const ai = useAiTranslations()
  const showAttachButton = input.showAttachButton !== false
  const showApprovalBadge = input.showApprovalBadge !== false
  const sendDisabled = !input.running && (input.disabled || input.inputEmpty || input.canContinue === false)
  const approvalColor = input.approvalMode === 'auto' ? 'warning' : 'info'
  const approvalLabel = input.approvalMode === 'auto'
    ? ai.panel.approvalModeAutoRun
    : ai.panel.approvalModeConfirmWrites

  return (
    <div className="chat-input-footer">
      <div className="chat-input-footer-left">
        {showAttachButton ? (
          <button
            type="button"
            className="chat-input-icon-btn"
            onClick={input.onAttach}
            aria-label={ai.input.addReference}
            disabled={input.disabled}
          >
            @
          </button>
        ) : null}

        <div className={cn('chat-input-model-wrap flex items-center gap-2', !showAttachButton && 'chat-input-model-wrap--leading')}>
          <StandardAiModelSelect
            models={input.models}
            selectedModel={input.selectedModel}
            onSelectModel={input.onSelectModel}
            disabled={input.modelsDisabled}
            size="sm"
            variant="ghost"
            width="auto"
            className="chat-input-model-selector"
            icon={<GitCompareArrows className="h-[13px] w-[13px]" />}
          />
          {showApprovalBadge ? (
            <Badge variant="soft" color={approvalColor} className="hidden sm:inline-flex">
              {ai.panel.approvalModeShortLabel}: {approvalLabel}
            </Badge>
          ) : null}
        </div>
      </div>

      <div className="chat-input-footer-right">
        <button
          type="button"
          onClick={input.onToggleRun}
          disabled={sendDisabled}
          data-running={input.running ? 'true' : 'false'}
          className="chat-input-send-btn"
          aria-label={input.running ? ai.action.stop : ai.action.send}
        >
          {input.running ? (
            <Square className="h-3.5 w-3.5" />
          ) : (
            <SendHorizonal className="h-3.5 w-3.5" />
          )}
        </button>
      </div>
    </div>
  )
}
