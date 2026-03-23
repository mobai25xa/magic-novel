import type { ReactNode } from 'react'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'

import { useAiTranslations } from '../ai-hooks'

type StandardAiModelSelectProps = {
  models: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  disabled?: boolean
  size?: 'default' | 'sm' | 'xs'
  variant?: 'default' | 'ghost'
  width?: 'full' | 'auto'
  className?: string
  icon?: ReactNode
}

export function StandardAiModelSelect(input: StandardAiModelSelectProps) {
  const ai = useAiTranslations()

  return (
    <Select value={input.selectedModel} onValueChange={input.onSelectModel} disabled={input.disabled}>
      <SelectTrigger
        size={input.size}
        variant={input.variant}
        width={input.width}
        className={input.className}
        disabled={input.disabled}
        aria-label={ai.panel.modelPlaceholder}
      >
        {input.icon}
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
  )
}
