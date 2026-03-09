import { useMemo, useState } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../../ai-hooks'
import { AiCodePreview, AiToolContent } from '@/magic-ui/components'
import { copyStepPreview, formatPreviewText } from '../tool-view-utils'

type GenericToolViewProps = {
  step: AgentUiToolStep
}

export function GenericToolView({ step }: GenericToolViewProps) {
  const ai = useAiTranslations()
  const [copied, setCopied] = useState<null | 'input' | 'output'>(null)

  const inputText = useMemo(() => formatPreviewText(step.inputPreview), [step.inputPreview])
  const outputText = useMemo(
    () => formatPreviewText(step.outputPreview ?? step.rawOutput),
    [step.outputPreview, step.rawOutput],
  )

  const handleCopy = async (kind: 'input' | 'output') => {
    try {
      const ok = await copyStepPreview(kind === 'input' ? step.inputPreview : (step.outputPreview ?? step.rawOutput))
      if (!ok) return
      setCopied(kind)
      window.setTimeout(() => setCopied(null), 1200)
    } catch {
      // ignore
    }
  }

  return (
    <AiToolContent className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-medium">{ai.tool.inputLabel}</div>
        <button
          type="button"
          onClick={() => { void handleCopy('input') }}
          className="text-[11px] text-muted-foreground underline hover:text-foreground transition-colors"
        >
          {copied === 'input' ? ai.action.copied : ai.action.copyInput}
        </button>
      </div>
      <AiCodePreview className="max-h-40 overflow-auto">
        {inputText || ai.tool.empty}
      </AiCodePreview>

      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-medium">{ai.tool.outputLabel}</div>
        <button
          type="button"
          onClick={() => { void handleCopy('output') }}
          className="text-[11px] text-muted-foreground underline hover:text-foreground transition-colors"
        >
          {copied === 'output' ? ai.action.copied : ai.action.copyOutput}
        </button>
      </div>
      <AiCodePreview className="max-h-48 overflow-auto">
        {outputText || ai.tool.empty}
      </AiCodePreview>
    </AiToolContent>
  )
}
