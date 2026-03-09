import { useTranslation } from '@/hooks/use-translation'

export function DiscoverEmptyState() {
  const { translations } = useTranslation()
  const d = translations.discover
  return (
    <div className="empty-state rounded-lg p-12 text-center">
      <div className="mb-2" style={{ color: "var(--text-secondary-foreground)" }}>
        {d.noData}
      </div>
      <div className="text-sm text-muted-foreground">
        {d.noDataHint}
      </div>
    </div>
  )
}
