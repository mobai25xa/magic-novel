import type { Editor as CoreEditor } from '@tiptap/core'
import {
  Heading1,
  Heading2,
  Heading3,
  Type,
  Quote,
  Minus,
  type LucideIcon,
} from 'lucide-react'
import type { Translations } from '@/i18n/locales/zh'

export interface SlashCommandItem {
  id: string
  label: string
  description: string
  icon: LucideIcon
  keywords: string[]
  action: (editor: CoreEditor) => void
}

export function buildSlashCommands(sc: Translations['slashCommand']): SlashCommandItem[] {
  return [
    {
      id: 'h1',
      label: sc.heading1,
      description: sc.heading1Desc,
      icon: Heading1,
      keywords: ['h1', 'heading1', '标题', 'title'],
      action: (editor) => editor.chain().focus().toggleHeading({ level: 1 }).run(),
    },
    {
      id: 'h2',
      label: sc.heading2,
      description: sc.heading2Desc,
      icon: Heading2,
      keywords: ['h2', 'heading2', '标题'],
      action: (editor) => editor.chain().focus().toggleHeading({ level: 2 }).run(),
    },
    {
      id: 'h3',
      label: sc.heading3,
      description: sc.heading3Desc,
      icon: Heading3,
      keywords: ['h3', 'heading3', '标题'],
      action: (editor) => editor.chain().focus().toggleHeading({ level: 3 }).run(),
    },
    {
      id: 'text',
      label: sc.paragraph,
      description: sc.paragraphDesc,
      icon: Type,
      keywords: ['text', 'paragraph', '正文', '段落'],
      action: (editor) => editor.chain().focus().setParagraph().run(),
    },
    {
      id: 'quote',
      label: sc.blockquote,
      description: sc.blockquoteDesc,
      icon: Quote,
      keywords: ['quote', 'blockquote', '引用'],
      action: (editor) => editor.chain().focus().toggleBlockquote().run(),
    },
    {
      id: 'divider',
      label: sc.horizontalRule,
      description: sc.horizontalRuleDesc,
      icon: Minus,
      keywords: ['divider', 'hr', 'line', '分割', '分隔'],
      action: (editor) => editor.chain().focus().setHorizontalRule().run(),
    },
  ]
}

/** @deprecated Use buildSlashCommands() instead */
export const SLASH_COMMANDS = buildSlashCommands({
  noMatch: '无匹配命令',
  heading1: '标题 1',
  heading1Desc: '章节标题',
  heading2: '标题 2',
  heading2Desc: '节标题',
  heading3: '标题 3',
  heading3Desc: '子标题',
  paragraph: '正文',
  paragraphDesc: '恢复为正文段落',
  blockquote: '引用',
  blockquoteDesc: '引用块',
  horizontalRule: '分割线',
  horizontalRuleDesc: '水平分割线',
})
