import type {
 GlobalSearchResult,
 SearchIndexFeatureStatus,
} from '@/features/search-retrieval'

export type SearchMode = 'keyword' | 'semantic' | 'hybrid'

export type SearchResultGroup = {
 path: string
 title: string
 volumeTitle?: string
 mainSnippet: string
 extraSnippets: string[]
 score: number
}

export type SearchState = {
 query: string
 mode: SearchMode
 hits: GlobalSearchResult[]
 isSearching: boolean
 error: string | null
 degraded: boolean
 degradedReason?: string
}

export type SearchHeaderProps = {
 count: number
 mode: SearchMode
 onModeChange: (mode: SearchMode) => void
 onClose: () => void
 degraded: boolean
 degradedReason?: string
}

export type SearchIndexBannerProps = {
 mode: SearchMode
 status: SearchIndexFeatureStatus | null
 loading: boolean
 onBuild: () => void
 onCancel: () => void
}
