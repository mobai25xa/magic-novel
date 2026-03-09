import { FileText, X } from 'lucide-react'

import { AiAttachmentItemShell } from '@/magic-ui/components'

import type { ChatContext } from './chat-context-types'
import { useAiTranslations } from '../ai-hooks'

type AttachmentPreviewProps = {
  contexts: ChatContext[]
  onRemove: (id: string) => void
}

const ATTACHMENT_THRESHOLD = 100

export function AttachmentPreview(input: AttachmentPreviewProps) {
  const ai = useAiTranslations()
  // Only show contexts whose labels are long enough to warrant a preview card
  const attachments = input.contexts.filter((c) => c.label.length > ATTACHMENT_THRESHOLD)

  if (attachments.length === 0) return null

  return (
    <div className="px-3 py-1.5 space-y-1.5">
      {attachments.map((ctx) => (
        <AiAttachmentItemShell
          key={ctx.id}
        >
          <FileText className="h-3.5 w-3.5 text-muted-foreground mt-0.5 shrink-0" />
          <div className="flex-1 min-w-0">
            <div className="font-medium truncate">{ctx.label}</div>
            <div className="text-muted-foreground line-clamp-2 mt-0.5">{ctx.path}</div>
          </div>
          <button
            type="button"
            className="rounded-full hover-bg p-0.5 shrink-0"
            onClick={() => input.onRemove(ctx.id)}
            aria-label={`${ai.action.removeAttachment} ${ctx.label}`}
          >
            <X className="h-3 w-3" />
          </button>
        </AiAttachmentItemShell>
      ))}
    </div>
  )
}
