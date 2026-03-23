import { invokeTauri } from './core'

export interface AssetSummary {
  id: string
  title: string
  modified_at?: number
}

export type AssetKind = 'lore' | 'prompt' | 'worldview' | 'outline' | 'character'

export type AssetLibraryNode =
  | { kind: 'dir'; name: string; path: string; title?: string; children: AssetLibraryNode[] }
  | {
      kind: 'file'
      name: string
      path: string
      title?: string
      asset_id?: string
      asset_kind?: string
      modified_at?: number
    }

export interface DailyStats {
  date: string
  word_count: number
  writing_duration_secs: number
  thinking_duration_secs: number
  sessions: WritingSession[]
}

export interface WritingSession {
  session_id: string
  project_path: string
  chapter_path?: string
  start_time: number
  end_time?: number
  start_word_count: number
  end_word_count?: number
  active_duration_secs: number
  idle_duration_secs: number
}

export interface MonthSummary {
  year: number
  month: number
  total_words: number
  writing_days: number
  daily_words: number[]
}

export async function listAssetsClient(projectPath: string, kind: AssetKind): Promise<AssetSummary[]> {
  return invokeTauri('list_assets', { projectPath, kind })
}

export async function readAssetClient(projectPath: string, kind: AssetKind, assetId: string): Promise<unknown> {
  return invokeTauri('read_asset', { projectPath, kind, assetId })
}

export async function saveAssetClient(projectPath: string, kind: AssetKind, asset: unknown): Promise<void> {
  return invokeTauri('save_asset', { projectPath, kind, asset })
}

export async function copyAssetClient(
  fromProjectPath: string,
  toProjectPath: string,
  kind: AssetKind,
  assetId: string,
): Promise<string> {
  return invokeTauri('copy_asset', { fromProjectPath, toProjectPath, kind, assetId })
}

export async function getAssetsTreeClient(projectPath: string): Promise<AssetLibraryNode[]> {
  return invokeTauri('get_assets_tree', { projectPath })
}

export async function readAssetFileClient(projectPath: string, relativePath: string): Promise<unknown> {
  return invokeTauri('read_asset_file', { projectPath, relativePath })
}

export async function saveAssetFileClient(
  projectPath: string,
  relativePath: string,
  asset: unknown,
): Promise<void> {
  return invokeTauri('save_asset_file', { projectPath, relativePath, asset })
}

export async function createAssetFolderClient(
  projectPath: string,
  parentRelativeDir: string,
  title: string,
): Promise<string> {
  return invokeTauri('create_asset_folder', { projectPath, parentRelativeDir, title })
}

export async function createAssetFileClient(
  projectPath: string,
  parentRelativeDir: string,
  assetKind: AssetKind,
  title: string,
): Promise<string> {
  return invokeTauri('create_asset_file', { projectPath, parentRelativeDir, assetKind, title })
}

export async function updateAssetFileTitleClient(
  projectPath: string,
  relativePath: string,
  newTitle: string,
): Promise<void> {
  return invokeTauri('update_asset_file_title', { projectPath, relativePath, newTitle })
}

export async function updateAssetFolderTitleClient(
  projectPath: string,
  relativeDir: string,
  newTitle: string,
): Promise<void> {
  return invokeTauri('update_asset_folder_title', { projectPath, relativeDir, newTitle })
}

export async function deleteAssetPathClient(projectPath: string, relativePath: string): Promise<void> {
  return invokeTauri('delete_asset_path', { projectPath, relativePath })
}

export async function appendChapterHistoryEventClient(
  projectPath: string,
  chapterId: string,
  event: unknown,
): Promise<void> {
  return invokeTauri('append_chapter_history_event', { projectPath, chapterId, event })
}

export async function getChapterHistoryClient(projectPath: string, chapterId: string): Promise<unknown[]> {
  return invokeTauri('get_chapter_history', { projectPath, chapterId })
}

export interface AiProposal {
  schema_version: number
  proposal_id: string
  chapter_id: string
  status: string
  prompt: string
  target: {
    type: string
    block_id?: string
    position?: string
  }
  context_refs?: {
    lore_asset_ids?: string[]
    prompt_asset_ids?: string[]
    node_ids?: string[]
  }
  model: {
    provider?: string
    name: string
    temperature?: number
    top_p?: number
  }
  output: {
    format: string
    text?: string
    tiptap_json?: unknown
  }
  created_at: number
  reviewed_at?: number
}

export async function saveAiProposalClient(projectPath: string, proposal: AiProposal): Promise<string> {
  return invokeTauri('save_ai_proposal', { projectPath, proposal })
}

export async function getAiProposalClient(projectPath: string, proposalId: string): Promise<AiProposal> {
  return invokeTauri('get_ai_proposal', { projectPath, proposalId })
}

export async function updateProposalStatusClient(
  projectPath: string,
  proposalId: string,
  status: string,
): Promise<void> {
  return invokeTauri('update_proposal_status', { projectPath, proposalId, status })
}

export async function listAiProposalsClient(projectPath: string, chapterId?: string): Promise<AiProposal[]> {
  return invokeTauri('list_ai_proposals', { projectPath, chapterId })
}

export async function recordWordsWrittenClient(wordCount: number, rootDir?: string): Promise<void> {
  return invokeTauri('record_words_written', { wordCount, rootDir })
}

export async function startWritingSessionClient(
  projectPath: string,
  chapterPath: string | null,
  currentWordCount: number,
  rootDir?: string,
): Promise<string> {
  return invokeTauri('start_writing_session', { projectPath, chapterPath, currentWordCount, rootDir })
}

export async function updateWritingSessionClient(
  currentWordCount: number,
  activeDurationSecs: number,
  idleDurationSecs: number,
  rootDir?: string,
): Promise<void> {
  return invokeTauri('update_writing_session', { currentWordCount, activeDurationSecs, idleDurationSecs, rootDir })
}

export async function endWritingSessionClient(finalWordCount: number, rootDir?: string): Promise<void> {
  return invokeTauri('end_writing_session', { finalWordCount, rootDir })
}

export async function getWritingStatsClient(days: number, rootDir?: string): Promise<DailyStats[]> {
  return invokeTauri('get_writing_stats', { days, rootDir })
}

export async function getMonthStatsClient(
  year: number,
  month: number,
  rootDir?: string,
): Promise<DailyStats[]> {
  return invokeTauri('get_month_stats', { year, month, rootDir })
}

export async function getYearStatsClient(year: number, rootDir?: string): Promise<MonthSummary[]> {
  return invokeTauri('get_year_stats', { year, rootDir })
}

export async function getConsecutiveDaysClient(rootDir?: string): Promise<number> {
  return invokeTauri('get_consecutive_days', { rootDir })
}
