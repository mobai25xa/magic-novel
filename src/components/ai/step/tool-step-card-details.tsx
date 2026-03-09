import { useMemo, useState } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../ai-hooks'
import { AiCodePreview } from '@/magic-ui/components'
import {
  copyStepPreview,
  formatPreviewText,
} from './tool-step-view-utils'
import { ToolStepDiffPreview } from './tool-step-diff-preview'

type ToolStepCardDetailsProps = {
  step: AgentUiToolStep
  sessionId: string
  turnId?: number
}

export function ToolStepCardDetails(input: ToolStepCardDetailsProps) {
  const ai = useAiTranslations()
  const [copied, setCopied] = useState<null | 'input' | 'output'>(null)

  const inputText = useMemo(() => formatPreviewText(input.step.inputPreview), [input.step.inputPreview])
  const outputText = useMemo(() => formatPreviewText(input.step.outputPreview ?? input.step.rawOutput), [
    input.step.outputPreview,
    input.step.rawOutput,
  ])

  const handleCopy = async (kind: 'input' | 'output') => {
    try {
      const ok = await copyStepPreview(kind === 'input' ? input.step.inputPreview : (input.step.outputPreview ?? input.step.rawOutput))
      if (!ok) return
      setCopied(kind)
      window.setTimeout(() => setCopied(null), 1200)
    } catch {
      // ignore
    }
  }

  return (
    <div className="mt-2 space-y-2">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-medium">{ai.tool.inputLabel}</div>
        <button
          type="button"
          onClick={() => { void handleCopy('input') }}
          className="text-[11px] underline hover:text-foreground"
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
          className="text-[11px] underline hover:text-foreground"
        >
          {copied === 'output' ? ai.action.copied : ai.action.copyOutput}
        </button>
      </div>
      <AiCodePreview className="max-h-48 overflow-auto">
        {outputText || ai.tool.empty}
      </AiCodePreview>

      <ToolStepDiffPreview
        step={input.step}
        sessionId={input.sessionId}
        turnId={input.turnId}
      />
    </div>
  )
}
