import type {
  AiProposal,
  AssetKind,
  AssetSummary,
  DailyStats,
  MagicAssetNode,
  MonthSummary,
  WritingSession,
} from '@/platform/tauri/clients/assets-client'
import {
  appendChapterHistoryEventClient,
  copyAssetClient,
  createMagicAssetFileClient,
  createMagicAssetFolderClient,
  deleteMagicAssetPathClient,
  endWritingSessionClient,
  getAiProposalClient,
  getChapterHistoryClient,
  getConsecutiveDaysClient,
  getMagicAssetsTreeClient,
  getMonthStatsClient,
  getWritingStatsClient,
  getYearStatsClient,
  listAiProposalsClient,
  listAssetsClient,
  readAssetClient,
  readMagicAssetClient,
  recordWordsWrittenClient,
  saveAiProposalClient,
  saveAssetClient,
  saveMagicAssetClient,
  startWritingSessionClient,
  updateMagicAssetFolderTitleClient,
  updateMagicAssetTitleClient,
  updateProposalStatusClient,
  updateWritingSessionClient,
} from '@/platform/tauri/clients/assets-client'

export async function listAssets(projectPath: string, kind: AssetKind): Promise<AssetSummary[]> {
  return listAssetsClient(projectPath, kind)
}

export async function readAsset(projectPath: string, kind: AssetKind, assetId: string): Promise<unknown> {
  return readAssetClient(projectPath, kind, assetId)
}

export async function saveAsset(projectPath: string, kind: AssetKind, asset: unknown): Promise<void> {
  return saveAssetClient(projectPath, kind, asset)
}

export async function copyAsset(
  fromProjectPath: string,
  toProjectPath: string,
  kind: AssetKind,
  assetId: string,
): Promise<string> {
  return copyAssetClient(fromProjectPath, toProjectPath, kind, assetId)
}

export async function getMagicAssetsTree(projectPath: string): Promise<MagicAssetNode[]> {
  return getMagicAssetsTreeClient(projectPath)
}

export async function readMagicAsset(projectPath: string, relativePath: string): Promise<unknown> {
  return readMagicAssetClient(projectPath, relativePath)
}

export async function saveMagicAsset(projectPath: string, relativePath: string, asset: unknown): Promise<void> {
  return saveMagicAssetClient(projectPath, relativePath, asset)
}

export async function createMagicAssetFolder(
  projectPath: string,
  parentRelativeDir: string,
  title: string,
): Promise<string> {
  return createMagicAssetFolderClient(projectPath, parentRelativeDir, title)
}

export async function createMagicAssetFile(
  projectPath: string,
  parentRelativeDir: string,
  assetKind: AssetKind,
  title: string,
): Promise<string> {
  return createMagicAssetFileClient(projectPath, parentRelativeDir, assetKind, title)
}

export async function updateMagicAssetTitle(projectPath: string, relativePath: string, newTitle: string): Promise<void> {
  return updateMagicAssetTitleClient(projectPath, relativePath, newTitle)
}

export async function updateMagicAssetFolderTitle(projectPath: string, relativeDir: string, newTitle: string): Promise<void> {
  return updateMagicAssetFolderTitleClient(projectPath, relativeDir, newTitle)
}

export async function deleteMagicAssetPath(projectPath: string, relativePath: string): Promise<void> {
  return deleteMagicAssetPathClient(projectPath, relativePath)
}

export async function saveAiProposal(projectPath: string, proposal: AiProposal): Promise<string> {
  return saveAiProposalClient(projectPath, proposal)
}

export async function getAiProposal(projectPath: string, proposalId: string): Promise<AiProposal> {
  return getAiProposalClient(projectPath, proposalId)
}

export async function updateProposalStatus(projectPath: string, proposalId: string, status: string): Promise<void> {
  return updateProposalStatusClient(projectPath, proposalId, status)
}

export async function appendChapterHistoryEvent(projectPath: string, chapterId: string, event: unknown): Promise<void> {
  return appendChapterHistoryEventClient(projectPath, chapterId, event)
}

export async function getChapterHistory(projectPath: string, chapterId: string): Promise<unknown[]> {
  return getChapterHistoryClient(projectPath, chapterId)
}

export async function listAiProposals(projectPath: string, chapterId?: string): Promise<AiProposal[]> {
  return listAiProposalsClient(projectPath, chapterId)
}

export async function startWritingSession(
  projectPath: string,
  chapterPath: string | null,
  currentWordCount: number,
  rootDir?: string,
): Promise<string> {
  return startWritingSessionClient(projectPath, chapterPath, currentWordCount, rootDir)
}

export async function updateWritingSession(
  currentWordCount: number,
  activeDurationSecs: number,
  idleDurationSecs: number,
  rootDir?: string,
): Promise<void> {
  return updateWritingSessionClient(currentWordCount, activeDurationSecs, idleDurationSecs, rootDir)
}

export async function endWritingSession(finalWordCount: number, rootDir?: string): Promise<void> {
  return endWritingSessionClient(finalWordCount, rootDir)
}

export async function recordWordsWritten(wordCount: number, rootDir?: string): Promise<void> {
  return recordWordsWrittenClient(wordCount, rootDir)
}

export async function getWritingStats(days: number, rootDir?: string): Promise<DailyStats[]> {
  return getWritingStatsClient(days, rootDir)
}

export async function getMonthStats(year: number, month: number, rootDir?: string): Promise<DailyStats[]> {
  return getMonthStatsClient(year, month, rootDir)
}

export async function getYearStats(year: number, rootDir?: string): Promise<MonthSummary[]> {
  return getYearStatsClient(year, rootDir)
}

export async function getConsecutiveDays(rootDir?: string): Promise<number> {
  return getConsecutiveDaysClient(rootDir)
}

export type {
  AiProposal,
  AssetKind,
  AssetSummary,
  DailyStats,
  MagicAssetNode,
  MonthSummary,
  WritingSession,
}
