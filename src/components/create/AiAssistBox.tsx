import { BrainCircuit } from 'lucide-react'
import { Toggle } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

interface AiAssistBoxProps {
  enabled: boolean
  onToggle: () => void
}

export function AiAssistBox({ enabled, onToggle }: AiAssistBoxProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  return (
    <div className="ai-assist-box">
      <div className="ai-assist-icon">
        <BrainCircuit size={24} />
      </div>
      <div className="ai-assist-content">
        <h3 className="ai-assist-title">{cp.aiAssistTitle}</h3>
        <p className="ai-assist-desc">{cp.aiAssistDesc}</p>
        <div className="ai-assist-toggle">
          <Toggle checked={enabled} onChange={onToggle} aria-label={cp.aiAssistTitle} />
          <span className="ai-assist-toggle-label">{cp.aiAssistToggle}</span>
        </div>
      </div>
    </div>
  )
}
