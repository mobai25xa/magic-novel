export type ChatContextType = 'chapter' | 'volume' | 'character' | 'location' | 'asset' | 'outline'

export type ChatContext = {
  id: string
  type: ChatContextType
  label: string
  path: string
}
