import { Button } from '@/magic-ui/components'

import type { ActiveCastV0, ChapterCardV0, RecentFactsV0 } from '@/components/ai/layer1-artifacts-card'
import type { ActiveCastDraft } from '@/components/ai/active-cast-editor'
import type { ChapterCardDraft } from '@/components/ai/chapter-card-editor'
import type { RecentFactsDraft } from '@/components/ai/recent-facts-editor'
import { Layer1ArtifactsCard } from '@/components/ai/layer1-artifacts-card'
import type { ContextPackV0 } from '@/components/ai/contextpack-card'
import { ContextPackCard } from '@/components/ai/contextpack-card'

type Layer1ContextPackSectionProps = {
  layer1Error: string | null
  contextPackError: string | null
  buildingContextPack: boolean
  chapterCard: ChapterCardV0 | null
  recentFacts: RecentFactsV0 | null
  activeCast: ActiveCastV0 | null
  contextPack: ContextPackV0 | null
  contextPackStale: boolean
  onSaveChapterCard: (draft: ChapterCardDraft) => void | Promise<void>
  onSaveRecentFacts: (draft: RecentFactsDraft) => void | Promise<void>
  onSaveActiveCast: (draft: ActiveCastDraft) => void | Promise<void>
  onCreateDefaultChapterCard: () => void
  onInferScopeFromCurrentChapter: () => void
  onBuildContextPack: () => void
  onFetchLatestContextPack: () => void
}

export function Layer1ContextPackSection({
  layer1Error,
  contextPackError,
  buildingContextPack,
  chapterCard,
  recentFacts,
  activeCast,
  contextPack,
  contextPackStale,
  onSaveChapterCard,
  onSaveRecentFacts,
  onSaveActiveCast,
  onCreateDefaultChapterCard,
  onInferScopeFromCurrentChapter,
  onBuildContextPack,
  onFetchLatestContextPack,
}: Layer1ContextPackSectionProps) {
  return (
    <div className="space-y-2">
      {layer1Error ? (
        <p className="text-xs text-muted-foreground">Layer1 unavailable: {layer1Error}</p>
      ) : null}

      <Layer1ArtifactsCard
        chapter_card={chapterCard}
        recent_facts={recentFacts}
        active_cast={activeCast}
        stale={contextPackStale}
        onSaveChapterCard={onSaveChapterCard}
        onSaveRecentFacts={onSaveRecentFacts}
        onSaveActiveCast={onSaveActiveCast}
        onCreateDefaultChapterCard={onCreateDefaultChapterCard}
        onInferScopeFromCurrentChapter={onInferScopeFromCurrentChapter}
        onBuildContextPack={onBuildContextPack}
      />

      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          className="text-xs"
          onClick={onBuildContextPack}
          disabled={buildingContextPack}
        >
          {buildingContextPack ? 'Building…' : 'Build/Refresh'}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="text-xs"
          onClick={onFetchLatestContextPack}
          disabled={buildingContextPack}
        >
          Fetch latest
        </Button>
      </div>

      {contextPackError ? (
        <p className="text-xs text-muted-foreground">ContextPack unavailable: {contextPackError}</p>
      ) : null}

      <ContextPackCard contextpack={contextPack} stale={contextPackStale} />
    </div>
  )
}
