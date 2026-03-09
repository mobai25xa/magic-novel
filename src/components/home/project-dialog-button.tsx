import type { ReactNode } from 'react'

import { Button } from '@/magic-ui/components'

type ProjectDialogButtonProps = {
  children: ReactNode
  onClick: () => void
  disabled?: boolean
  variant?: 'default' | 'outline'
  className?: string
}

export function ProjectDialogButton({
  children,
  onClick,
  disabled,
  variant = 'default',
  className = '',
}: ProjectDialogButtonProps) {
  return (
    <Button
      onClick={onClick}
      disabled={disabled}
      variant={variant === 'outline' ? 'secondary' : 'default'}
      className={className}
    >
      {children}
    </Button>
  )
}
