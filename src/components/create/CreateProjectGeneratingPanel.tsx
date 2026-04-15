import { LoaderCircle, Sparkles } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'

interface CreateProjectGeneratingPanelProps {
  projectName: string
}

export function CreateProjectGeneratingPanel(input: CreateProjectGeneratingPanelProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  return (
    <div className="rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-6">
      <div className="flex flex-col items-center justify-center gap-4 py-10 text-center">
        <div className="flex h-16 w-16 items-center justify-center rounded-full bg-[var(--bg-base)]">
          <LoaderCircle size={28} className="animate-spin" />
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-center gap-2 text-xs font-semibold uppercase tracking-[0.2em] opacity-60">
            <Sparkles size={14} />
            <span>{cp.generatingContractStageLabel}</span>
          </div>
          <h2 className="text-xl font-semibold">{cp.generatingContractTitle}</h2>
          <p className="max-w-2xl text-sm opacity-75">
            {cp.generatingContractSubtitle.replace('{name}', input.projectName)}
          </p>
          <p className="max-w-2xl text-sm opacity-60">{cp.generatingContractDescription}</p>
        </div>
      </div>
    </div>
  )
}
