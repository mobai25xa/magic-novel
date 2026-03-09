import type { ComponentPropsWithoutRef, ReactNode } from 'react'

import { Button } from '@/magic-ui/components'

type SettingsButtonProps = Omit<ComponentPropsWithoutRef<typeof Button>, 'variant'> & {
  children: ReactNode
  variant?: 'default' | 'outline'
}

export function SettingsButton({ children, variant = 'default', className = '', ...props }: SettingsButtonProps) {
  return (
    <Button
      {...props}
      variant={variant === 'outline' ? 'settingsOutline' : 'default'}
      className={className}
    >
      {children}
    </Button>
  )
}
