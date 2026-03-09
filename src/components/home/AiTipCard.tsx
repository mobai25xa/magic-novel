import { Lightbulb } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'

export function AiTipCard() {
  const { t } = useTranslation()

  return (
    <div className="bento-card bento-card-ai-tip">
      <div className="bento-stat-header">
        <span className="bento-ai-title">{t('workspace.aiTipTitle')}</span>
        <Lightbulb size={16} className="bento-ai-title-icon" />
      </div>

      <p className="bento-ai-tip-text">{t('workspace.aiTipContent')}</p>

      <button className="bento-ai-action">{t('workspace.aiTipAction')}</button>
    </div>
  )
}
