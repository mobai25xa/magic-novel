import { Settings2 } from 'lucide-react'

import { Button, type ButtonProps } from '@/magic-ui/components'

type StandardAiSettingsButtonProps = {
  label: string
  onClick: () => void
  disabled?: boolean
  variant?: ButtonProps['variant']
  className?: string
}

export function StandardAiSettingsButton(input: StandardAiSettingsButtonProps) {
  return (
    <Button
      variant={input.variant ?? 'outline'}
      onClick={input.onClick}
      disabled={input.disabled}
      className={`gap-2 ${input.className ?? ''}`.trim()}
    >
      <Settings2 size={14} />
      {input.label}
    </Button>
  )
}
