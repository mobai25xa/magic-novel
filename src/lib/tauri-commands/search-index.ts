import {
  cancelSearchIndexClient,
  getSearchIndexStatusClient,
  rebuildSearchIndexClient,
  type RebuildSearchIndexInput,
  type SearchIndexStatus,
} from '@/platform/tauri/clients/search-index-client'

export async function getSearchIndexStatus(projectPath: string): Promise<SearchIndexStatus> {
  return getSearchIndexStatusClient(projectPath)
}

export async function rebuildSearchIndex(input: RebuildSearchIndexInput): Promise<SearchIndexStatus> {
  return rebuildSearchIndexClient(input)
}

export async function cancelSearchIndex(projectPath: string): Promise<SearchIndexStatus> {
  return cancelSearchIndexClient(projectPath)
}

export type { RebuildSearchIndexInput, SearchIndexStatus }
