import { useEffect } from 'react'
import type { Editor } from '@tiptap/react'

import { eventBus, EVENTS } from '@/lib/events'
import { EditorAPI } from '@/lib/editor-api'

export function useEditorRegistration(editor: Editor | null, setEditor: (editor: Editor | null) => void) {
  useEffect(() => {
    if (editor) {
      setEditor(editor)
    }

    return () => {
      setEditor(null)
    }
  }, [editor, setEditor])
}

export function useEditorIndentClass(editor: Editor | null, firstLineIndent: boolean) {
  useEffect(() => {
    if (!editor) return

    const baseClass = 'novel-editor-content max-w-none focus:outline-none min-h-full'
    const className = firstLineIndent ? `${baseClass} first-line-indent` : baseClass

    editor.setOptions({
      editorProps: {
        attributes: {
          class: className,
        },
      },
    })
  }, [editor, firstLineIndent])
}

type EditorContentSyncInput = {
  editor: Editor | null
  initialContent?: unknown
  setIsDirty: (value: boolean) => void
}

export function useEditorContentSync({ editor, initialContent, setIsDirty }: EditorContentSyncInput) {
  useEffect(() => {
    if (!editor || !initialContent) return

    const currentContent = JSON.stringify(editor.getJSON())
    const nextContent = JSON.stringify(initialContent)

    if (currentContent !== nextContent) {
      editor.commands.setContent(initialContent as Parameters<typeof editor.commands.setContent>[0])
      setIsDirty(false)
    }
  }, [editor, initialContent, setIsDirty])
}

export function useManualSaveDebugExpose(manualSave: () => Promise<void>) {
  useEffect(() => {
    if (typeof window === 'undefined') return

    ;(window as unknown as { __manualSave?: typeof manualSave }).__manualSave = manualSave
  }, [manualSave])
}

export function useEditorApiBridge(editor: Editor | null) {
  useEffect(() => {
    if (editor) {
      const api = new EditorAPI(editor)
      ;(window as unknown as { editor?: { api: EditorAPI } }).editor = { api }
      eventBus.emit(EVENTS.EDITOR_READY)
    }

    return () => {
      delete (window as unknown as { editor?: unknown }).editor
      eventBus.emit(EVENTS.EDITOR_DESTROYED)
    }
  }, [editor])
}

export function useFindReplaceShortcuts(setShowFindReplace: (value: boolean) => void) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 'f') {
        event.preventDefault()
        setShowFindReplace(true)
      }

      if (event.key === 'Escape') {
        setShowFindReplace(false)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [setShowFindReplace])

  useEffect(() => {
    const handleFindReplaceOpen = () => setShowFindReplace(true)
    eventBus.on(EVENTS.FIND_REPLACE_OPEN, handleFindReplaceOpen)
    return () => eventBus.off(EVENTS.FIND_REPLACE_OPEN, handleFindReplaceOpen)
  }, [setShowFindReplace])
}
