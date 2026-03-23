import type { ReactNode } from 'react'
import { Loader2, SendHorizonal } from 'lucide-react'

import { ChatInput } from '@/components/ai/input/ChatInput'

type LightweightChatComposerProps = {
  inputValue: string
  onInputChange: (value: string) => void
  onSend: () => void | Promise<void>
  inputPlaceholder: string
  inputDisabled?: boolean
  sendDisabled?: boolean
  sendLabel: string
  footerActions?: ReactNode
  pending?: boolean
}

export function LightweightChatComposer(input: LightweightChatComposerProps) {
  return (
    <div className="chat-input-shell" data-disabled={input.inputDisabled ? 'true' : 'false'}>
      <ChatInput
        value={input.inputValue}
        onChange={input.onInputChange}
        onSend={() => {
          void input.onSend()
        }}
        disabled={input.inputDisabled}
        placeholder={input.inputPlaceholder}
      />

      <div className="chat-input-footer">
        <div className="chat-input-footer-left min-w-0 flex-1 flex-wrap gap-2">
          {input.footerActions}
        </div>

        <div className="chat-input-footer-right">
          <button
            type="button"
            onClick={() => {
              void input.onSend()
            }}
            disabled={input.sendDisabled}
            data-running={input.pending ? 'true' : 'false'}
            className="chat-input-send-btn"
            aria-label={input.sendLabel}
            title={input.sendLabel}
          >
            {input.pending ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <SendHorizonal className="h-3.5 w-3.5" />
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
