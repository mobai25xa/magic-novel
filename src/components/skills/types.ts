type SkillSource = 'builtin' | 'user'

export type SkillCategory = 'all' | 'polish' | 'logic' | 'roleplay'

export type SkillColorVariant = 1 | 2 | 3 | 4 | 5

export interface SkillCardData {
  name: string
  display_name: string
  description: string
  system_prompt_snippet: string
  enabled: boolean
  source: SkillSource
  colorVariant: SkillColorVariant
  tags: string[]
}
