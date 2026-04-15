import type { LucideIcon } from 'lucide-react'

export interface GenreOption {
  id: string
  name: string
  icon: LucideIcon
}

export interface CreatePageProps {
  onCreated: (path: string) => void | Promise<void>
}

export type CreateProjectNarrativePov =
  | 'first_person'
  | 'third_limited'
  | 'third_omniscient'

export interface CreateProjectDraft {
  name: string
  author: string
  description: string
  coverImage: string
  selectedGenres: string[]
  customGenres: string
  targetTotalWords: string
  plannedVolumes: string
  targetWordsPerChapter: string
  narrativePov: CreateProjectNarrativePov
  tone: string
  audience: string
  protagonistSeed: string
  counterpartSeed: string
  worldSeed: string
  endingDirection: string
  aiAssist: boolean
}

export type CreateProjectFormErrors = Partial<Record<
  'name' | 'author' | 'description' | 'projectType' | 'targetTotalWords',
  string
>>

export type CreateProjectWorkflowStage =
  | 'ideation'
  | 'launch_sheet'
  | 'generating_contract'
