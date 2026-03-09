import { useEffect, useState } from 'react'
import type { Dispatch, SetStateAction } from 'react'

import {
 cancelSearchIndexFeature,
 getSearchIndexStatusFeature,
 rebuildSearchIndexFeature,
 searchGlobalFeature,
 type SearchIndexFeatureStatus,
 type SearchMode,
} from '@/features/search-retrieval'

import type { SearchState } from './global-search-types'

function toErrorMessage(error: unknown) {
 return error instanceof Error ? error.message : String(error)
}

function emptySearchState(mode: SearchMode): SearchState {
 return {
 query: '',
 mode,
 hits: [],
 isSearching: false,
 error: null,
 degraded: false,
 degradedReason: undefined,
 }
}

type SearchQueryInput = {
 isOpen: boolean
 query: string
 mode: SearchMode
 projectPath: string | null
}

export function useGlobalSearchResults(input: SearchQueryInput) {
 const [searchState, setSearchState] = useState<SearchState>(emptySearchState('keyword'))

 const safeQuery = typeof input.query === 'string' ? input.query : ''
 const trimmedQuery = safeQuery.trim()
 const canSearch = Boolean(input.isOpen && input.projectPath && trimmedQuery)
 const pending = canSearch && (searchState.query !== trimmedQuery || searchState.mode !== input.mode)

 const isActive = searchState.query === trimmedQuery && searchState.mode === input.mode
 const activeHits = isActive ? searchState.hits : []
 const activeError = isActive ? searchState.error : null
 const activeSearching = (isActive && searchState.isSearching) || pending
 const degraded = isActive && searchState.degraded
 const degradedReason = isActive ? searchState.degradedReason : undefined

 useEffect(() => {
 const timer = createSearchTimer({
 canSearch,
 mode: input.mode,
 projectPath: input.projectPath,
 query: trimmedQuery,
 setSearchState,
 })

 return () => {
 timer.cancel()
 }
 }, [canSearch, input.mode, input.projectPath, trimmedQuery])

 return {
 activeHits,
 activeError,
 activeSearching,
 degraded,
 degradedReason,
 }
}

type SearchTimerInput = {
 canSearch: boolean
 mode: SearchMode
 projectPath: string | null
 query: string
 setSearchState: Dispatch<SetStateAction<SearchState>>
}

type SearchTimerHandle = {
 cancel: () => void
}

function createSearchTimer(input: SearchTimerInput): SearchTimerHandle {
 if (!input.canSearch || !input.projectPath) {
 return { cancel: () => {} }
 }

 let cancelled = false

 const timer = window.setTimeout(() => {
 if (cancelled) return

 input.setSearchState((prev) => ({
 ...prev,
 query: input.query,
 mode: input.mode,
 hits: [],
 isSearching: true,
 error: null,
 degraded: false,
 degradedReason: undefined,
 }))

 searchGlobalFeature({
 projectPath: input.projectPath as string,
 query: input.query,
 mode: input.mode,
 })
 .then((next) => {
 if (cancelled) return
 input.setSearchState({
 query: input.query,
 mode: input.mode,
 hits: next.hits,
 isSearching: false,
 error: null,
 degraded: next.degraded,
 degradedReason: next.degradedReason,
 })
 })
 .catch((error: unknown) => {
 if (cancelled) return
 input.setSearchState({
 query: input.query,
 mode: input.mode,
 hits: [],
 isSearching: false,
 error: toErrorMessage(error),
 degraded: false,
 degradedReason: undefined,
 })
 })
 }, 150)

 return {
 cancel: () => {
 cancelled = true
 window.clearTimeout(timer)
 },
 }
}

export function useGlobalSearchIndexStatus(input: {
 isOpen: boolean
 mode: SearchMode
 projectPath: string | null
}) {
 const [indexStatus, setIndexStatus] = useState<SearchIndexFeatureStatus | null>(null)
 const [indexLoading, setIndexLoading] = useState(false)

 useEffect(() => {
 if (!input.isOpen || !input.projectPath || input.mode === 'keyword') {
 return
 }

 let cancelled = false

 const loadStatus = () => {
 getSearchIndexStatusFeature(input.projectPath as string).then((next) => {
 if (cancelled) return
 setIndexStatus(next)
 })
 }

 loadStatus()
 const timer = window.setInterval(loadStatus, 1000)

 return () => {
 cancelled = true
 window.clearInterval(timer)
 }
 }, [input.isOpen, input.mode, input.projectPath])

 const triggerRebuild = async () => {
 if (!input.projectPath) return
 setIndexLoading(true)
 const next = await rebuildSearchIndexFeature(input.projectPath, true)
 setIndexStatus(next)
 setIndexLoading(false)
 }

 const triggerCancel = async () => {
 if (!input.projectPath) return
 setIndexLoading(true)
 const next = await cancelSearchIndexFeature(input.projectPath)
 setIndexStatus(next)
 setIndexLoading(false)
 }

 return {
 indexStatus,
 indexLoading,
 triggerRebuild,
 triggerCancel,
 }
}
