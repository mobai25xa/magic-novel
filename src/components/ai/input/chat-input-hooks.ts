import { useEffect, useMemo } from 'react'
import { useEditor } from '@tiptap/react'

import type { ChatContext } from './chat-context-types'
import type { MentionItem } from './mention-data'
import { createChatInputEditorOptions } from './chat-input-editor-options'
import { createMentionSuggestionRenderer } from './mention-suggestion-renderer'

type UseChatInputEditorInput = {
  value: string
  onChange: (value: string) => void
  onSend: () => void
  disabled?: boolean
  placeholder?: string
  onMentionSelect?: (context: ChatContext) => void
}

function mentionItemToContext(item: MentionItem): ChatContext {
  return {
    id: item.id,
    type: item.type === 'volume' ? 'chapter' : item.type,
    label: item.label,
    path: item.path,
  }
}

export function useChatInputEditor(input: UseChatInputEditorInput) {
  const { value, onChange, onSend, disabled, placeholder, onMentionSelect } = input

  const handleMentionSelect = useMemo(() => {
    return (item: MentionItem) => {
      const context = mentionItemToContext(item)
      onMentionSelect?.(context)
    }
  }, [onMentionSelect])

  const editor = useEditor(useMemo(() => {
    const options = createChatInputEditorOptions({
      value,
      placeholder,
      onSend,
      onMentionSelect: handleMentionSelect,
      onTextChange: onChange,
    })

    return {
      ...options,
      content: undefined,
    }
  }, [handleMentionSelect, onChange, onSend, placeholder, value]))

  useEffect(() => {
    if (!editor) {
      return
    }

    const currentText = editor.getText()
    if (currentText === value) {
      return
    }

    editor.commands.setContent(value ? `<p>${escapeHtml(value)}</p>` : '')
  }, [value, editor])

  useEffect(() => {
    if (!editor) {
      return
    }
    editor.setEditable(!disabled)
  }, [disabled, editor])

  useEffect(() => {
    if (!editor) {
      return
    }

    const mentionExtension = editor.extensionManager.extensions.find((extension) => extension.name === 'mention')
    if (!mentionExtension || !mentionExtension.options?.suggestion) {
      return
    }

    mentionExtension.options.suggestion.render = createMentionSuggestionRenderer(handleMentionSelect)
  }, [editor, handleMentionSelect])

  return editor
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/\n/g, '<br>')
}
