import { useCallback } from 'react'
import { EditorContent } from '@tiptap/react'
import type { Editor } from '@tiptap/core'

import type { ChatContext } from './chat-context-types'
import { useChatInputEditor } from './chat-input-hooks'

type ChatInputProps = {
  value: string
  onChange: (value: string) => void
  onSend: () => void
  disabled?: boolean
  placeholder?: string
  onMentionSelect?: (context: ChatContext) => void
}

export function extractMentions(editor: Editor): ChatContext[] {
  const mentions: ChatContext[] = []
  editor.state.doc.descendants((node) => {
    if (node.type.name === 'mention') {
      mentions.push({
        id: node.attrs.id ?? '',
        type: node.attrs.mentionType ?? 'asset',
        label: node.attrs.label ?? '',
        path: node.attrs.path ?? '',
      })
    }
  })
  return mentions
}

export function ChatInput(props: ChatInputProps) {
  const editor = useChatInputEditor(props)

  const handleContainerClick = useCallback(() => {
    if (props.disabled) {
      return
    }
    editor?.commands.focus()
  }, [editor, props.disabled])

  if (!editor) return null

  return (
    <div
      className={`chat-input-editor relative ${props.disabled ? 'cursor-not-allowed' : 'cursor-text'}`}
      onClick={handleContainerClick}
    >
      <EditorContent editor={editor} />
      {editor.isEmpty && (
        <div className="absolute top-4 left-4 text-sm text-muted-foreground pointer-events-none">
          {props.placeholder}
        </div>
      )}
    </div>
  )
}

