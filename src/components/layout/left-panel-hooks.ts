import { useEffect } from 'react'

import { listAssets, readMagicAsset, type AssetKind } from '@/features/assets-management'
import {
  assetTreeToEditorDoc,
  type KnowledgeAssetTree,
} from '@/features/assets-management/asset-editor-document'
import { loadChapterWordGoal } from '@/features/editor-reading'

type ToastVariant = 'default' | 'success' | 'warning' | 'destructive' | 'info'

type AddToast = (toast: { title: string; description?: string; variant?: ToastVariant }) => void

const ASSET_KINDS: { kind: AssetKind; label: string }[] = [
  { kind: 'worldview', label: '世界观' },
  { kind: 'outline', label: '大纲' },
  { kind: 'character', label: '人物' },
  { kind: 'lore', label: '资料' },
  { kind: 'prompt', label: '提示词' },
]

type ChapterWordGoalInput = {
  projectPath: string | null
  currentChapterPath: string | null
  setChapterWordGoal: (goal: number | null) => void
}

type SetCurrentAssetDoc = (input: {
  relativePath: string
  title: string | null
  content: unknown
}) => void

export function useChapterWordGoal({
  projectPath,
  currentChapterPath,
  setChapterWordGoal,
}: ChapterWordGoalInput) {
  useEffect(() => {
    const loadChapterGoal = async () => {
      if (!projectPath || !currentChapterPath) {
        setChapterWordGoal(null)
        return
      }

      try {
        const goal = await loadChapterWordGoal(projectPath, currentChapterPath)
        setChapterWordGoal(goal)
      } catch (error) {
        console.error('Failed to load chapter word goal:', error)
        setChapterWordGoal(null)
      }
    }

    loadChapterGoal()
  }, [currentChapterPath, projectPath, setChapterWordGoal])
}

export async function loadPinnedAssetOptions(input: {
  projectPath: string
  addToast: AddToast
  setPinnedAssetsOptions: (options: { value: string; label: string }[]) => void
  setPinnedAssetsDefault: (value: string | undefined) => void
  setPinnedAssetsDialogOpen: (open: boolean) => void
}) {
  try {
    const options: { value: string; label: string }[] = []
    for (const assetKind of ASSET_KINDS) {
      const list = await listAssets(input.projectPath, assetKind.kind)
      for (const item of list) {
        options.push({ value: `${assetKind.kind}:${item.id}`, label: `${assetKind.label} / ${item.title}` })
      }
    }

    input.setPinnedAssetsOptions(options)
    input.setPinnedAssetsDefault(options[0]?.value)
    input.setPinnedAssetsDialogOpen(true)
  } catch (error) {
    console.error('Failed to load pinned asset options:', error)
    input.addToast({ title: '加载失败', description: String(error), variant: 'destructive' })
  }
}

export async function handleLeftPanelAssetSelect(input: {
  projectPath: string
  relativePath: string
  setCurrentAssetDoc: SetCurrentAssetDoc
}) {
  try {
    const asset = (await readMagicAsset(input.projectPath, input.relativePath)) as KnowledgeAssetTree
    const title =
      asset && typeof asset === 'object' && 'title' in asset && typeof asset.title === 'string'
        ? asset.title
        : null
    const docContent = assetTreeToEditorDoc(asset)

    input.setCurrentAssetDoc({
      relativePath: input.relativePath,
      title,
      content: docContent,
    })
  } catch (error) {
    console.error('Failed to open asset:', error)
  }
}

