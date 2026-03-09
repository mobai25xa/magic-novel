/**
 * @author Gamma
 * @date 2026-02-11
 * @description 编辑器底部状态栏
 */
/* eslint-disable react-hooks/set-state-in-effect */
import { useEffect, useState, useCallback } from 'react'
import type { Editor } from '@tiptap/react'
import { useEditorStore } from '@/state/editor'
import { useTranslation } from '@/hooks/use-translation'

interface StatusBarProps {
  editor: Editor | null
}

interface StatusInfo {
  line: number
  column: number
}

export function StatusBar({ editor }: StatusBarProps) {
  const { isDirty, isSaving } = useEditorStore()
  const { translations } = useTranslation()
  const ed = translations.editor
  const sb = translations.statusBar
  const [status, setStatus] = useState<StatusInfo>({
    line: 1,
    column: 1,
  })

  const updateStatus = useCallback(() => {
    if (!editor) return

    const { from } = editor.state.selection
    const { line, column } = calculateLineColumn(editor, from)

    setStatus({ line, column })
  }, [editor])

  // 监听内容变化和选区变化
  useEffect(() => {
    if (!editor) return

    const handleUpdate = () => updateStatus()
    const handleSelectionUpdate = () => updateStatus()

    editor.on('update', handleUpdate)
    editor.on('selectionUpdate', handleSelectionUpdate)

    // 初始计算
    updateStatus()

    return () => {
      editor.off('update', handleUpdate)
      editor.off('selectionUpdate', handleSelectionUpdate)
    }
  }, [editor, updateStatus])

  // 保存状态文本
  const getSaveStatus = (): string => {
    if (isSaving) return ed.saving
    if (isDirty) return ed.unsaved
    return ed.saved
  }

  // 保存状态颜色
  const getSaveStatusColor = (): string => {
    if (isSaving) return 'text-warning'
    if (isDirty) return 'text-warning'
    return ''
  }

  if (!editor) return null

  return (
    <div className="status-bar">
      <span>{sb.line} {status.line}, {sb.column} {status.column}</span>

      <div className="flex-1" />

      <span className={getSaveStatusColor()}>
        {getSaveStatus()}
      </span>
    </div>
  )
}

/**
 * 计算光标所在的行号和列号
 */
function calculateLineColumn(editor: Editor, pos: number): { line: number; column: number } {
  let line = 1
  let column = 1

  editor.state.doc.descendants((node, nodePos) => {
    if (nodePos >= pos) return false

    if (['paragraph', 'heading', 'blockquote'].includes(node.type.name)) {
      if (nodePos + node.nodeSize <= pos) {
        line++
      } else if (nodePos < pos) {
        const offsetInNode = pos - nodePos - 1
        const textContent = node.textContent
        const textBefore = textContent.slice(0, Math.min(offsetInNode, textContent.length))
        column = textBefore.length + 1

        const lines = textBefore.split('\n')
        if (lines.length > 1) {
          line += lines.length - 1
          column = lines[lines.length - 1].length + 1
        }
      }
    }
    return true
  })

  return { line, column }
}
