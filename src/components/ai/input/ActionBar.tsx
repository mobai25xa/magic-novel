import { GitCompareArrows, SendHorizonal, Square } from 'lucide-react'

import { Badge, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'
import type { ApprovalMode } from '@/state/settings'

import { useAiTranslations } from '../ai-hooks'

type ActionBarProps = {
  models: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  approvalMode: ApprovalMode
  running: boolean
  inputEmpty: boolean
  canContinue?: boolean
  onAttach?: () => void
  onToggleRun: () => void
}

export function ActionBar(input: ActionBarProps) {
  const ai = useAiTranslations()
  const sendDisabled = !input.running && (input.inputEmpty || input.canContinue === false)
  const approvalColor = input.approvalMode === 'auto' ? 'warning' : 'info'
  const approvalLabel = input.approvalMode === 'auto'
    ? ai.panel.approvalModeAutoRun
    : ai.panel.approvalModeConfirmWrites

  return (
    <div className="chat-input-footer">
      <div className="chat-input-footer-left">
        <button
          type="button"
          className="chat-input-icon-btn"
          onClick={input.onAttach}
          aria-label={ai.input.addReference}
        >
          @
        </button>

        <div className="chat-input-model-wrap flex items-center gap-2">
          <Select value={input.selectedModel} onValueChange={input.onSelectModel}>
            <SelectTrigger size="sm" variant="ghost" width="auto" className="chat-input-model-selector">
              <GitCompareArrows className="h-[13px] w-[13px]" />
              <SelectValue placeholder={ai.panel.modelPlaceholder} />
            </SelectTrigger>
            <SelectContent>
              {input.models.map((model) => (
                <SelectItem key={model} value={model}>
                  {model}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Badge variant="soft" color={approvalColor} className="hidden sm:inline-flex">
            {ai.panel.approvalModeShortLabel}: {approvalLabel}
          </Badge>
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
