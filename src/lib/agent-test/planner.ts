export type PlannerAction =
  | { tool: 'create'; intent: 'create_chapter'; title: string }
  | { tool: 'read'; intent: 'read_snapshot' | 'read_json'; view: 'snapshot' | 'json' }
  | { tool: 'edit'; intent: 'preview' | 'commit'; dry_run: boolean }

export function planAction(message: string): PlannerAction | null {
  const m = message.trim().toLowerCase()

  if (!m) return null

  if (m.includes('create chapter') || m.includes('创建章节')) {
    const title = extractQuoted(message) || '新章节'
    return { tool: 'create', intent: 'create_chapter', title }
  }

  if (m.includes('read json') || m.includes('读取json')) {
    return { tool: 'read', intent: 'read_json', view: 'json' }
  }

  if (m.includes('read') || m.includes('读取') || m.includes('markdown')) {
    return { tool: 'read', intent: 'read_snapshot', view: 'snapshot' }
  }

  if (m.includes('preview') || m.includes('预览')) {
    return { tool: 'edit', intent: 'preview', dry_run: true }
  }

  if (m.includes('commit') || m.includes('提交')) {
    return { tool: 'edit', intent: 'commit', dry_run: false }
  }

  return null
}

function extractQuoted(input: string): string | null {
  const match = input.match(/["“](.+?)["”]/)
  return match ? match[1] : null
}
