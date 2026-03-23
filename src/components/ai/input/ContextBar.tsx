import { BookOpen, FileText, MapPin, Package, User, X } from 'lucide-react'

import { useAiTranslations } from '../ai-hooks'
import { StandardAiModelSelect } from '../shared/StandardAiModelSelect'
import type { ChatContext, ChatContextType } from './chat-context-types'

type ContextBarProps = {
  contexts: ChatContext[]
  onRemoveContext: (contextId: string) => void
  models: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
}

const CONTEXT_ICON: Record<ChatContextType, typeof FileText> = {
  chapter: FileText,
  volume: BookOpen,
  character: User,
  location: MapPin,
  asset: Package,
  outline: FileText,
}

export function ContextBar(input: ContextBarProps) {
  const ai = useAiTranslations()
  const hasContexts = input.contexts.length > 0

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 min-h-[32px]">
      {hasContexts && (
        <div className="flex items-center gap-1.5 flex-1 min-w-0 flex-wrap">
          {input.contexts.map((ctx) => {
            const Icon = CONTEXT_ICON[ctx.type] ?? Package
            return (
              <span
                key={ctx.id}
                className="mention inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs"
              >
                <Icon className="h-3 w-3 text-muted-foreground shrink-0" />
                <span className="truncate max-w-[120px]">{ctx.label}</span>
                <button
                  type="button"
                  className="ml-0.5 rounded-full hover-bg p-0.5"
                  onClick={() => input.onRemoveContext(ctx.id)}
                  aria-label={`${ai.action.removeContext} ${ctx.label}`}
                >
                  <X className="h-2.5 w-2.5" />
                </button>
              </span>
            )
          })}
        </div>
      )}

      <div className={`${hasContexts ? '' : 'ml-auto'} shrink-0`}>
        <StandardAiModelSelect
          models={input.models}
          selectedModel={input.selectedModel}
          onSelectModel={input.onSelectModel}
          size="xs"
          variant="ghost"
          className="w-32"
        />
      </div>
    </div>
  )
}
