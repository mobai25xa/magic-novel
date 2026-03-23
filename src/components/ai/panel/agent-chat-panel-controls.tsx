import { SendHorizonal, Square } from 'lucide-react'

import type { AiChatViewMode } from '@/state/settings'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'

import { StandardAiModelSelect } from '../shared/StandardAiModelSelect'
import { useAiTranslations } from '../ai-hooks'

type AgentChatPanelControlsProps = {
  running: boolean
  inputValue: string
  canContinue?: boolean
  models: string[]
  selectedModel: string
  viewMode: AiChatViewMode
  onSelectModel: (model: string) => void
  onSelectViewMode: (mode: AiChatViewMode) => void
  onToggleRun: () => void
}

export function AgentChatPanelControls(input: AgentChatPanelControlsProps) {
  const ai = useAiTranslations()

  return (
    <div className="flex items-center justify-between gap-2">
      <div className="flex items-center gap-2 flex-1 min-w-0">
        <div className="w-40 min-w-[120px]">
          <StandardAiModelSelect
            models={input.models}
            selectedModel={input.selectedModel}
            onSelectModel={input.onSelectModel}
            size="sm"
          />
        </div>

        <div className="w-24">
          <Select value={input.viewMode} onValueChange={(value) => input.onSelectViewMode(value as AiChatViewMode)}>
            <SelectTrigger size="sm">
              <SelectValue placeholder={ai.panel.viewPlaceholder} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="compact">{ai.panel.viewCompact}</SelectItem>
              <SelectItem value="debug">{ai.panel.viewDebug}</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <button
        type="button"
        onClick={input.onToggleRun}
        disabled={!input.running && (!input.inputValue.trim() || input.canContinue === false)}
        className="chat-input-send-btn disabled:opacity-50"
      >
        {input.running ? <Square className="h-3.5 w-3.5" /> : <SendHorizonal className="h-3.5 w-3.5" />}
      </button>
    </div>
  )
}
