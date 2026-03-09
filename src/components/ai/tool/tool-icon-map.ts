export type ToolIconName = 'create' | 'read' | 'edit' | 'delete' | 'move' | 'ls' | 'grep' | 'todo' | 'fallback'

const TOOL_ICON_NAME_MAP: Record<string, ToolIconName> = {
  create: 'create',
  read: 'read',
  edit: 'edit',
  delete: 'delete',
  move: 'move',
  ls: 'ls',
  grep: 'grep',
  todowrite: 'todo',
}

export function resolveToolIconName(toolName: string): ToolIconName {
  return TOOL_ICON_NAME_MAP[toolName] ?? 'fallback'
}
