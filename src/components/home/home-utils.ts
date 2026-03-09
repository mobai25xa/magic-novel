import type { FileNode as BackendFileNode } from '@/features/project-home'

export function convertFileNode(node: BackendFileNode) {
  return {
    kind: node.kind,
    name: node.name,
    path: node.path,
    children: node.children?.map(convertFileNode),
    chapterId: node.chapter_id,
    title: node.title,
    textLengthNoWhitespace: node.word_count ?? node.text_length_no_whitespace,
    status: node.status,
    updatedAt: node.updated_at,
  }
}

export function sanitizeFilename(name: string) {
  return name.replace(/[\\/:*?"<>|]/g, '_').trim() || 'untitled'
}

export function formatRelativeDate(timestamp: number, input: {
  today: string
  yesterday: string
  daysAgo: string
  weeksAgo: string
  monthsAgo: string
  yearsAgo: string
}) {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return input.today
  if (days === 1) return input.yesterday
  if (days < 7) return `${days}${input.daysAgo}`
  if (days < 30) return `${Math.floor(days / 7)}${input.weeksAgo}`
  if (days < 365) return `${Math.floor(days / 30)}${input.monthsAgo}`
  return `${Math.floor(days / 365)}${input.yearsAgo}`
}

export function formatLocaleNumber(num: number, language: string) {
  return num.toLocaleString(language === 'en' ? 'en-US' : 'zh-CN')
}
