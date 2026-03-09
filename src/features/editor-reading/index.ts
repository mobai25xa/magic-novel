import {
  createChapter as createChapterCommand,
  createVolume as createVolumeCommand,
  getProjectTree as getProjectTreeCommand,
  readChapter as readChapterCommand,
  setChapterWordGoal as setChapterWordGoalCommand,
  updateChapterMetadata as updateChapterMetadataCommand,
  type Chapter,
  type FileNode,
} from '@/lib/tauri-commands'

export type { Chapter, FileNode }

export async function loadChapterWordGoal(projectPath: string, chapterPath: string): Promise<number | null> {
  const chapter = await readChapterCommand(projectPath, chapterPath)
  return chapter.target_words ?? null
}

export async function addPinnedAssetToChapter(
  projectPath: string,
  chapterPath: string,
  selectedValue: string,
): Promise<{ status: 'invalid' | 'duplicate' | 'added' }> {
  const chapter = await readChapterCommand(projectPath, chapterPath)
  const existing = (chapter.pinned_assets || []).map((asset) => ({
    kind: asset.kind,
    assetId: asset.asset_id,
    nodeIds: asset.node_ids,
  }))

  const [kind, assetId] = String(selectedValue).split(':')
  if (!kind || !assetId) {
    return { status: 'invalid' }
  }

  const already = existing.some((asset) => asset.kind === kind && asset.assetId === assetId)
  if (already) {
    return { status: 'duplicate' }
  }

  await updateChapterMetadataCommand(projectPath, chapterPath, {
    pinnedAssets: [...existing, { kind, assetId }],
  })

  return { status: 'added' }
}

export {
  readChapterCommand as readChapter,
  createVolumeCommand as createVolume,
  createChapterCommand as createChapter,
  getProjectTreeCommand as getProjectTree,
  setChapterWordGoalCommand as setChapterWordGoal,
  updateChapterMetadataCommand as updateChapterMetadata,
}
