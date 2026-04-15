import { useMemo, useRef, useState } from 'react'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import Highlight from '@tiptap/extension-highlight'
import { UniqueIdExtension } from './extensions/unique-id'
import { EditorToolbar } from './EditorToolbar'
import { FindReplacePanel } from './FindReplacePanel'
import { ScrollButtons } from './ScrollButtons'
import { useEditorStore } from '@/state/editor'
import { EDITOR_FONT_PRESETS, useSettingsStore } from '@/state/settings'

import { useLayoutStore } from '@/stores/layout-store'
import { useAutoSave } from '@/hooks/use-auto-save'

import { PasteHandlerExtension } from './extensions/paste-handler'
import { SlashCommandExtension } from './extensions/slash-command'
import { KeyboardShortcutsExtension } from './extensions/keyboard-shortcuts'
import {
  useEditorApiBridge,
  useEditorContentSync,
  useEditorIndentClass,
  useEditorRegistration,
  useFindReplaceShortcuts,
  useManualSaveDebugExpose,
} from './novel-editor-hooks'

interface NovelEditorProps {
  initialContent?: unknown
  onContentChange?: (content: unknown) => void
  documentKind?: 'chapter' | 'asset' | 'knowledge'
}

const DEFAULT_EDITOR_CONTENT = {
  type: 'doc',
  content: [
    {
      type: 'paragraph',
      content: [],
    },
  ],
}

function buildEditorCssVars(input: {
  editorFontSize: number
  editorLineHeight: number
  editorContentWidth: number
  editorFontFamily: keyof typeof EDITOR_FONT_PRESETS
  editorTextAlign: 'center' | 'left'
  isLeftPanelVisible: boolean
  isRightPanelVisible: boolean
}) {
  const visibleSidebars = Number(input.isLeftPanelVisible) + Number(input.isRightPanelVisible)
  const pagePadding = visibleSidebars === 2 ? 32 : visibleSidebars === 1 ? 24 : 20
  const contentMaxWidth = visibleSidebars === 2 ? 1320 : visibleSidebars === 1 ? 1560 : 1880

  return {
    '--editor-font-size': `${input.editorFontSize}px`,
    '--editor-line-height': String(input.editorLineHeight),
    '--editor-content-max-width': `${contentMaxWidth}px`,
    '--editor-font-family': EDITOR_FONT_PRESETS[input.editorFontFamily].fontFamily,
    '--editor-justify-content': input.editorTextAlign === 'center' ? 'center' : 'flex-start',
    '--editor-side-padding': `${pagePadding}px`,
  } as React.CSSProperties
}

function useNovelTiptapEditor(input: {
  initialContent?: unknown
  firstLineIndent: boolean
  documentKind: 'chapter' | 'asset' | 'knowledge'
  onContentChange?: (content: unknown) => void
  setIsDirty: (dirty: boolean) => void
}) {
  return useEditor({
    extensions: [
      StarterKit.configure({
        heading: {
          levels: [1, 2, 3],
        },
        undoRedo: {
          depth: 200,
        },
      }),
      Highlight.configure({
        multicolor: true,
      }),
      UniqueIdExtension,
      PasteHandlerExtension,
      SlashCommandExtension,
      KeyboardShortcutsExtension,
    ],
    content: input.initialContent || DEFAULT_EDITOR_CONTENT,
    onUpdate: ({ editor }) => {
      input.setIsDirty(true)
      input.onContentChange?.(editor.getJSON())
    },
    editorProps: {
      attributes: {
        class: `novel-editor-content max-w-none focus:outline-none min-h-full editor-content-kind-${input.documentKind}${input.firstLineIndent ? ' first-line-indent' : ''}`,
      },
    },
  })
}

export function NovelEditor({ initialContent, onContentChange, documentKind = 'chapter' }: NovelEditorProps) {
  const { setEditor, setIsDirty } = useEditorStore()
  const { firstLineIndent, editorFontSize, editorLineHeight, editorContentWidth, editorFontFamily, editorTextAlign } = useSettingsStore()
  const { isLeftPanelVisible, isRightPanelVisible } = useLayoutStore()
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const [showFindReplace, setShowFindReplace] = useState(false)

  const editorCssVars = useMemo(
    () =>
      buildEditorCssVars({
        editorFontSize,
        editorLineHeight,
        editorContentWidth,
        editorFontFamily,
        editorTextAlign,
        isLeftPanelVisible,
        isRightPanelVisible,
      }),
    [
      editorFontSize,
      editorLineHeight,
      editorContentWidth,
      editorFontFamily,
      editorTextAlign,
      isLeftPanelVisible,
      isRightPanelVisible,
    ]
  )

  const editor = useNovelTiptapEditor({
    initialContent,
    firstLineIndent,
    documentKind,
    onContentChange,
    setIsDirty,
  })

  useEditorRegistration(editor, setEditor)
  useEditorIndentClass(editor, firstLineIndent)
  useEditorContentSync({ editor, initialContent, setIsDirty })

  const { manualSave } = useAutoSave(editor)
  useManualSaveDebugExpose(manualSave)
  useEditorApiBridge(editor)
  useFindReplaceShortcuts(setShowFindReplace)

  return (
    <div className="flex h-full flex-col min-h-0">
      <EditorToolbar editor={editor} onToggleFindReplace={() => setShowFindReplace(!showFindReplace)} />
      {showFindReplace && (
        <FindReplacePanel editor={editor} isOpen={showFindReplace} onClose={() => setShowFindReplace(false)} />
      )}
      <div className="flex-1 min-h-0 relative overflow-hidden">
        <div ref={scrollContainerRef} style={editorCssVars} className="h-full overflow-auto editor-scroll-container">
          <EditorContent editor={editor} className="h-full" />
        </div>
        <ScrollButtons containerRef={scrollContainerRef} />
      </div>
    </div>
  )
}
