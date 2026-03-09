import type { ReactNode } from 'react'

import { Button } from '@/magic-ui/components'

type HomeDialogButtonProps = {
  children: ReactNode
  onClick: () => void
  disabled?: boolean
  variant?: 'default' | 'outline'
  size?: 'sm' | 'default' | 'lg'
  className?: string
}

export function HomeDialogButton({
  children,
  onClick,
  disabled,
  variant = 'default',
  size = 'default',
  className = '',
}: HomeDialogButtonProps) {
  return (
    <Button
      onClick={onClick}
      disabled={disabled}
      variant={variant === 'outline' ? 'outline' : 'default'}
      size={size}
      className={className}
    >
      {children}
    </Button>
  )
}

