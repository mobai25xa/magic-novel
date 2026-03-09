import { useEffect, useRef } from 'react'

import { useAiTranslations } from '../ai-hooks'

type AgentChatPanelInputProps = {
  value: string
  disabled?: boolean
  onChange: (value: string) => void
  onSend: () => void
}

export function AgentChatPanelInput(input: AgentChatPanelInputProps) {
  const ai = useAiTranslations()
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)

  useEffect(() => {
    if (!textareaRef.current) {
      return
    }

    textareaRef.current.style.height = 'auto'
    textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 220)}px`
  }, [input.value])

  return (
    <textarea
      ref={textareaRef}
      value={input.value}
      onChange={(event) => input.onChange(event.target.value)}
      rows={3}
      placeholder={ai.panel.inputPlaceholder}
      disabled={input.disabled}
      className="w-full resize-none bg-transparent px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 min-h-[72px]"
      onKeyDown={(event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault()
          input.onSend()
        }
      }}
    />
  )
}
