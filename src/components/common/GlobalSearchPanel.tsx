import { useMemo, useState } from 'react'

import { type SearchMode } from '@/features/search-retrieval'
import { useProjectStore } from '@/stores/project-store'

import { SearchEmptyState } from './global-search/global-search-empty-state'
import { SearchHeader } from './global-search/global-search-header'
import { SearchIndexBanner } from './global-search/global-search-index-banner'
import { SearchResultList } from './global-search/global-search-result-list'
import {
 useGlobalSearchIndexStatus,
 useGlobalSearchResults,
} from './global-search/use-global-search-panel-state'
import { mapVolumeTitles, toSearchGroups } from './global-search/global-search-utils'

interface GlobalSearchPanelProps {
 query?: string
 isOpen: boolean
 onClose: () => void
 onResultClick: (chapterPath: string) => void
}

export function GlobalSearchPanel({ query = '', isOpen, onClose, onResultClick }: GlobalSearchPanelProps) {
 const safeQuery = typeof query === 'string' ? query : ''
 const { projectPath, tree } = useProjectStore()
 const volumeTitles = useMemo(() => mapVolumeTitles(tree), [tree])
 const [mode, setMode] = useState<SearchMode>('keyword')
 const { activeHits, activeError, activeSearching, degraded, degradedReason } = useGlobalSearchResults({
 isOpen,
 query: safeQuery,
 mode,
 projectPath,
 })
 const { indexStatus, indexLoading, triggerRebuild, triggerCancel } = useGlobalSearchIndexStatus({
 isOpen,
 mode,
 projectPath,
 })

 if (!isOpen) {
 return null
 }

 const groups = toSearchGroups(activeHits, volumeTitles)

 return (
 <div
 className="search-panel absolute top-full left-0 right-0 mt-2 mx-4 z-50 max-h-96"
 >
 <SearchHeader
 count={groups.length}
 mode={mode}
 onModeChange={setMode}
 onClose={onClose}
 degraded={degraded}
 degradedReason={degradedReason}
 />

 <SearchIndexBanner
 mode={mode}
 status={indexStatus}
 loading={indexLoading}
 onBuild={triggerRebuild}
 onCancel={triggerCancel}
 />

 <div className="flex-1 overflow-y-auto">
 {groups.length === 0 ? (
 <SearchEmptyState query={safeQuery} isSearching={activeSearching} error={activeError} />
 ) : (
 <SearchResultList query={safeQuery} groups={groups} onClose={onClose} onResultClick={onResultClick} />
 )}
 </div>
 </div>
 )
}
