export interface ToolLsInput {
  project_path: string
  path?: string
  offset?: number
  limit?: number
  call_id?: string
}

export interface ToolLsItem {
  kind: 'volume' | 'chapter' | 'knowledge_root' | 'knowledge_folder' | 'knowledge_file'
  name: string
  path: string
  title?: string
  child_count?: number
  chapter_id?: string
}

export interface ToolLsOutput {
  cwd: string
  items: ToolLsItem[]
}
