import StarterKit from '@tiptap/starter-kit'
import Mention from '@tiptap/extension-mention'
import type { EditorOptions } from '@tiptap/core'

import type { MentionItem } from './mention-data'
import { getMentionItems } from './mention-data'
import { createMentionSuggestionRenderer } from './mention-suggestion-renderer'

type ChatInputEditorOptionsInput = {
  value: string
  placeholder?: string
  onSend: () => void
  onTextChange: (next: string) => void
  onMentionSelect: (item: MentionItem) => void
}

export function createChatInputEditorOptions(input: ChatInputEditorOptionsInput) {
  const options: Partial<EditorOptions> = {
    extensions: [
      StarterKit.configure({
        heading: false,
        blockquote: false,
        bulletList: false,
        orderedList: false,
        codeBlock: false,
        horizontalRule: false,
      }),
      Mention.configure({
        HTMLAttributes: {
          class: 'mention',
        },
        suggestion: {
          char: '@',
          items: ({ query }: { query: string }) => getMentionItems(query),
          render: createMentionSuggestionRenderer(input.onMentionSelect),
        },
        renderLabel({ node }) {
          return `@${node.attrs.label ?? ''}`
        },
      }),
    ],
    content: input.value ? `<p>${escapeHtml(input.value)}</p>` : '',
    editorProps: {
      attributes: {
        class:
          'w-full min-h-[92px] max-h-[220px] overflow-y-auto overflow-x-hidden px-4 py-4 text-sm leading-relaxed focus:outline-none whitespace-pre-wrap break-words',
        'data-placeholder': input.placeholder ?? '',
      },
      handleKeyDown: (_view, event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          const mentionPopup = document.querySelector('.mention-suggestion-popup')
          if (mentionPopup) {
            return false
          }

          event.preventDefault()
          input.onSend()
          return true
        }
        return false
      },
    },
    onUpdate: ({ editor }) => {
      input.onTextChange(editor.getText())
    },
  }

  return options
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/\n/g, '<br>')
}
