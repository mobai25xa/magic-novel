import type { Editor } from '@tiptap/react'
import type { Translations } from '@/i18n/locales/zh'

type ToolbarAction = {
  key: string
  title: string
  onClick: () => void
  isActive?: boolean
  disabled?: boolean
  section: 'history' | 'heading' | 'text' | 'tool' | 'view'
}

function buildHistoryActions(editor: Editor, tb: Translations['toolbar']): ToolbarAction[] {
  return [
    {
      key: 'undo',
      title: tb.undo,
      onClick: () => editor.chain().focus().undo().run(),
      disabled: !editor.can().undo(),
      section: 'history',
    },
    {
      key: 'redo',
      title: tb.redo,
      onClick: () => editor.chain().focus().redo().run(),
      disabled: !editor.can().redo(),
      section: 'history',
    },
  ]
}

function buildHeadingActions(editor: Editor, tb: Translations['toolbar']): ToolbarAction[] {
  return [
    {
      key: 'heading-1',
      title: tb.heading1,
      onClick: () => editor.chain().focus().toggleHeading({ level: 1 }).run(),
      isActive: editor.isActive('heading', { level: 1 }),
      section: 'heading',
    },
    {
      key: 'heading-2',
      title: tb.heading2,
      onClick: () => editor.chain().focus().toggleHeading({ level: 2 }).run(),
      isActive: editor.isActive('heading', { level: 2 }),
      section: 'heading',
    },
    {
      key: 'heading-3',
      title: tb.heading3,
      onClick: () => editor.chain().focus().toggleHeading({ level: 3 }).run(),
      isActive: editor.isActive('heading', { level: 3 }),
      section: 'heading',
    },
    {
      key: 'blockquote',
      title: tb.quote,
      onClick: () => editor.chain().focus().toggleBlockquote().run(),
      isActive: editor.isActive('blockquote'),
      section: 'heading',
    },
    {
      key: 'horizontal-rule',
      title: tb.divider,
      onClick: () => editor.chain().focus().setHorizontalRule().run(),
      section: 'heading',
    },
  ]
}

function buildTextActions(editor: Editor, tb: Translations['toolbar']): ToolbarAction[] {
  return [
    {
      key: 'bold',
      title: tb.bold,
      onClick: () => editor.chain().focus().toggleBold().run(),
      isActive: editor.isActive('bold'),
      section: 'text',
    },
    {
      key: 'italic',
      title: tb.italic,
      onClick: () => editor.chain().focus().toggleItalic().run(),
      isActive: editor.isActive('italic'),
      section: 'text',
    },
    {
      key: 'strike',
      title: tb.strikethrough,
      onClick: () => editor.chain().focus().toggleStrike().run(),
      isActive: editor.isActive('strike'),
      section: 'text',
    },
    {
      key: 'highlight',
      title: tb.highlight,
      onClick: () => editor.chain().focus().toggleHighlight().run(),
      isActive: editor.isActive('highlight'),
      section: 'text',
    },
  ]
}

export function buildEditorToolbarActions(input: {
  editor: Editor
  isFullscreen: boolean
  toggleFullscreen: () => void
  onToggleFindReplace?: () => void
  toolbar: Translations['toolbar']
}) {
  const { editor, toolbar: tb } = input

  const actions: ToolbarAction[] = [
    ...buildHistoryActions(editor, tb),
    ...buildHeadingActions(editor, tb),
    ...buildTextActions(editor, tb),
    {
      key: 'find-replace',
      title: tb.findReplace,
      onClick: () => input.onToggleFindReplace?.(),
      section: 'tool',
    },
    {
      key: 'fullscreen',
      title: input.isFullscreen ? tb.exitFullscreen : tb.enterFullscreen,
      onClick: input.toggleFullscreen,
      section: 'view',
    },
  ]

  const sectionOrder: ToolbarAction['section'][] = ['history', 'heading', 'text', 'tool', 'view']
  return { actions, sectionOrder }
}

export type { ToolbarAction }
