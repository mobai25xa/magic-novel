import { useCallback, useEffect, useMemo, useRef } from 'react'
import type { Editor } from '@tiptap/react'
import debounce from 'lodash.debounce'

import { saveChapterContent } from '@/features/content-editing'
import { saveKnowledgeDocumentContent } from '@/features/content-editing/save-knowledge-document-content'
import { saveKnowledgeAssetContent } from '@/features/content-editing/save-knowledge-asset-content'
import { eventBus, EVENTS } from '@/lib/events'
import { useEditorStore } from '@/stores/editor-store'

const AUTO_SAVE_DELAY = 2000

type UseAutoSaveSaveInput = {
  editor: Editor | null
  projectPath: string | null
  activeDocPath: string | null
  currentDocKind: 'chapter' | 'asset' | 'knowledge' | null
  isDirty: boolean
  setIsDirty: (value: boolean) => void
  setIsSaving: (value: boolean) => void
  setTree: (tree: unknown[]) => void
}

type PerformSaveInput = Omit<
  UseAutoSaveSaveInput,
  'isDirty'
>

function usePerformSave({
  editor,
  projectPath,
  activeDocPath,
  currentDocKind,
  setIsDirty,
  setIsSaving,
  setTree,
}: PerformSaveInput) {
  const lastSaveTime = useRef<number>(Date.now())

  return useCallback(async () => {
    if (!editor || !projectPath || !activeDocPath || !currentDocKind) {
      return
    }

    try {
      setIsSaving(true)

      const newTree = currentDocKind === 'asset'
        ? await saveKnowledgeAssetContent({
            editor,
            projectPath,
            assetRelativePath: activeDocPath,
          })
        : currentDocKind === 'knowledge'
          ? await saveKnowledgeDocumentContent({
              editor,
              projectPath,
              knowledgePath: activeDocPath,
            })
          : await saveChapterContent(editor, projectPath, activeDocPath)

      setIsDirty(false)
      const now = Date.now()
      lastSaveTime.current = now
      useEditorStore.getState().setLastSavedAt(now)

      if (newTree) {
        setTree(newTree)
      }

      if (currentDocKind === 'chapter') {
        eventBus.emit(EVENTS.CHAPTER_SAVED, {
          projectPath,
          chapterPath: activeDocPath,
        })
      }
    } catch (error) {
      console.error('Auto-save failed:', error)
    } finally {
      setIsSaving(false)
    }
  }, [activeDocPath, currentDocKind, editor, projectPath, setIsDirty, setIsSaving, setTree])
}

function useDebouncedAutoSave(performSave: () => Promise<void>) {
  return useMemo(
    () =>
      debounce(() => {
        void performSave()
      }, AUTO_SAVE_DELAY),
    [performSave],
  )
}

function useAutoSaveOnDirty(input: {
  isDirty: boolean
  activeDocPath: string | null
  editor: Editor | null
  debouncedSave: ReturnType<typeof debounce>
}) {
  const { isDirty, activeDocPath, editor, debouncedSave } = input

  useEffect(() => {
    if (isDirty && activeDocPath && editor) {
      debouncedSave()
    }
  }, [activeDocPath, debouncedSave, editor, isDirty])
}

function useManualSaveShortcut(input: {
  performSave: () => Promise<void>
  debouncedSave: ReturnType<typeof debounce>
}) {
  const { performSave, debouncedSave } = input

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 's') {
        event.preventDefault()
        debouncedSave.cancel()
        void performSave()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      debouncedSave.cancel()
    }
  }, [debouncedSave, performSave])
}

function useDebounceCleanup(debouncedSave: ReturnType<typeof debounce>) {
  useEffect(() => {
    return () => {
      debouncedSave.cancel()
    }
  }, [debouncedSave])
}

export function useAutoSaveSave(input: UseAutoSaveSaveInput) {
  const { editor, projectPath, activeDocPath, currentDocKind, isDirty, setIsDirty, setIsSaving, setTree } = input

  const performSave = usePerformSave({
    editor,
    projectPath,
    activeDocPath,
    currentDocKind,
    setIsDirty,
    setIsSaving,
    setTree,
  })
  const debouncedSave = useDebouncedAutoSave(performSave)

  useAutoSaveOnDirty({
    isDirty,
    activeDocPath,
    editor,
    debouncedSave,
  })
  useManualSaveShortcut({ performSave, debouncedSave })
  useDebounceCleanup(debouncedSave)

  return { manualSave: performSave }
}
