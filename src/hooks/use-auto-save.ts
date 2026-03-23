import type { Editor } from '@tiptap/react'
import { useEditorStore } from '@/stores/editor-store'
import { useProjectStore } from '@/stores/project-store'
import { useSettingsStore } from '@/stores/settings-store'
import { useAutoSaveSave } from './use-auto-save-save'
import { useAutoSaveSession } from './use-auto-save-session'

export function useAutoSave(editor: Editor | null) {
  const { currentDocKind, currentChapterPath, currentAssetPath, isDirty, setIsDirty, setIsSaving } = useEditorStore()
  const { projectPath, setTree } = useProjectStore()
  const { projectsRootDir } = useSettingsStore()

  const activeDocPath = currentDocKind === 'chapter' ? currentChapterPath : currentAssetPath

  useAutoSaveSession({
    editor,
    projectPath,
    activeDocPath,
    projectsRootDir,
  })

  return useAutoSaveSave({
    editor,
    projectPath,
    activeDocPath,
    currentDocKind,
    isDirty,
    setIsDirty,
    setIsSaving,
    setTree,
  })
}
