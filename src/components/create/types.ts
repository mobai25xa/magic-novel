import type { LucideIcon } from 'lucide-react'

export interface GenreOption {
  id: string
  name: string
  icon: LucideIcon
}

export interface CreatePageProps {
  onCreated: (path: string) => void
}
