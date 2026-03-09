import {
  KeyRound,
  Timer,
  ServerCrash,
  WifiOff,
  Layers,
  FileWarning,
  type LucideIcon,
} from 'lucide-react'

import type { ErrorCategory } from './classify-error'

export const CATEGORY_ICONS: Record<ErrorCategory, LucideIcon> = {
  auth: KeyRound,
  rate_limit: Timer,
  server: ServerCrash,
  network: WifiOff,
  context_limit: Layers,
  client: FileWarning,
}

export const CATEGORY_COLORS: Record<ErrorCategory, string> = {
  auth: 'text-warning',
  rate_limit: 'text-warning',
  server: 'text-destructive',
  network: 'text-warning',
  context_limit: 'text-info',
  client: 'text-muted-foreground',
}

export const CATEGORY_BORDER_COLORS: Record<ErrorCategory, string> = {
  auth: 'border-l-[var(--color-warning)]',
  rate_limit: 'border-l-[var(--color-warning)]',
  server: 'border-l-[var(--color-danger)]',
  network: 'border-l-[var(--color-warning)]',
  context_limit: 'border-l-[var(--color-info)]',
  client: 'border-l-[var(--text-muted-foreground)]',
}

export const CATEGORY_BG_COLORS: Record<ErrorCategory, string> = {
  auth: 'tag-yellow-bg',
  rate_limit: 'tag-yellow-bg',
  server: 'bg-danger-soft',
  network: 'tag-yellow-bg',
  context_limit: 'tag-blue-bg',
  client: 'bg-secondary',
}
