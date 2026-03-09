/**
 * @author Beta
 * @date 2026-02-11
 * @description 原子化操作模块 — 不依赖 React，只依赖 TipTap Editor 实例
 *
 * 设计原则：
 * 1. 每个函数接收 Editor 实例作为第一个参数
 * 2. 不使用 useState/useContext/useRef 等 React API
 * 3. 不从 Zustand store 读取状态
 * 4. 通过 Editor 的 transaction 系统执行，保证 undo 一致性
 */
import type { Editor } from '@tiptap/react'
import { TextSelection } from '@tiptap/pm/state'
import { serializeToMarkdown } from './markdown-serializer'

const BLOCK_TYPES = ['paragraph', 'heading', 'blockquote']

// ─── 读取操作 ──────────────────────────────────────

/** 获取完整 JSON 内容 */
export function operationGetJSON(editor: Editor): object {
  return editor.getJSON()
}

/** 获取 Markdown 内容 */
export function operationGetMarkdown(editor: Editor): string {
  return serializeToMarkdown(editor.getJSON())
}

/** 获取纯文本 */
export function operationGetText(editor: Editor): string {
  return editor.getText()
}

/** 获取字数（不含空白字符） */
export function operationGetWordCount(editor: Editor): number {
  const text = editor.getText()
  return text.replace(/\s/g, '').length
}

/** 获取所有段落 ID */
export function operationGetAllParagraphIds(editor: Editor): string[] {
  const ids: string[] = []
  editor.state.doc.descendants((node) => {
    if (BLOCK_TYPES.includes(node.type.name) && node.attrs.id) {
      ids.push(node.attrs.id)
    }
  })
  return ids
}

/** 获取指定段落的纯文本 */
export function operationGetParagraphText(editor: Editor, id: string): string | null {
  let result: string | null = null
  editor.state.doc.descendants((node) => {
    if (result !== null) return false
    if (node.attrs.id === id) {
      result = node.textContent
      return false
    }
  })
  return result
}

/** 获取光标位置（段落ID + 偏移量） */
export function operationGetCursorPosition(editor: Editor): {
  paragraphId: string | null
  offset: number
} {
  const { $from } = editor.state.selection

  // 向上遍历找到最近的块级节点
  let paragraphId: string | null = null
  const offset = $from.parentOffset

  for (let depth = $from.depth; depth >= 0; depth--) {
    const node = $from.node(depth)
    if (BLOCK_TYPES.includes(node.type.name)) {
      paragraphId = node.attrs.id || null
      break
    }
  }

  return { paragraphId, offset }
}

// ─── 写入操作 ──────────────────────────────────────

/** 在光标位置插入文字 */
export function operationInsertText(editor: Editor, text: string): void {
  editor.chain().focus().insertContent(text).run()
}

/** 替换指定段落中的文字 */
export function operationReplaceText(
  editor: Editor,
  paragraphId: string,
  oldText: string,
  newText: string
): boolean {
  let replaced = false

  editor.state.doc.descendants((node, pos) => {
    if (replaced) return false
    if (node.attrs.id !== paragraphId) return

    const text = node.textContent
    const index = text.indexOf(oldText)
    if (index === -1) return false

    // 计算精确的 ProseMirror 位置
    // pos 是节点起始位置，+1 是进入节点内部
    const from = pos + 1 + index
    const to = from + oldText.length

    const { state, view } = editor
    const tr = state.tr.replaceWith(from, to, state.schema.text(newText))
    view.dispatch(tr)
    replaced = true
    return false
  })

  return replaced
}

/** 在指定段落后插入新段落，返回新段落的 UUID */
export function operationInsertParagraphAfter(
  editor: Editor,
  paragraphId: string,
  content: string
): string {
  let insertPos = editor.state.doc.content.size

  editor.state.doc.descendants((node, pos) => {
    if (node.attrs.id === paragraphId) {
      insertPos = pos + node.nodeSize
      return false
    }
    return true
  })

  // 插入新段落（UUID 将由 UniqueIdExtension 自动分配）
  editor.chain().focus().insertContentAt(insertPos, {
    type: 'paragraph',
    content: content ? [{ type: 'text', text: content }] : [],
  }).run()

  // 获取新段落的 UUID
  const newNode = editor.state.doc.nodeAt(insertPos)
  return newNode?.attrs.id || ''
}

/** 删除指定段落 */
export function operationDeleteParagraph(editor: Editor, paragraphId: string): boolean {
  let deleted = false

  editor.state.doc.descendants((node, pos) => {
    if (deleted) return false
    if (node.attrs.id !== paragraphId) return

    const { state, view } = editor
    const tr = state.tr.delete(pos, pos + node.nodeSize)
    view.dispatch(tr)
    deleted = true
    return false
  })

  return deleted
}

// ─── 光标操作 ──────────────────────────────────────

/** 移动光标到指定段落开头 */
export function operationMoveCursorToParagraph(editor: Editor, paragraphId: string): void {
  editor.state.doc.descendants((node, pos) => {
    if (node.attrs.id === paragraphId) {
      // pos+1 = 段落内部开头
      const selection = TextSelection.create(editor.state.doc, pos + 1)
      const tr = editor.state.tr.setSelection(selection).scrollIntoView()
      editor.view.dispatch(tr)
      editor.view.focus()
      return false
    }
    return true
  })
}

/** 移动光标到文档末尾 */
export function operationMoveCursorToEnd(editor: Editor): void {
  const endPos = editor.state.doc.content.size - 1
  const selection = TextSelection.create(editor.state.doc, endPos)
  const tr = editor.state.tr.setSelection(selection).scrollIntoView()
  editor.view.dispatch(tr)
  editor.view.focus()
}

// ─── 复合操作（用于与现有 React 代码桥接）──────────

/** 查找文本，返回所有匹配的段落ID和偏移 */
export function operationFindText(
  editor: Editor,
  text: string
): Array<{ paragraphId: string; offset: number; from: number; to: number }> {
  const results: Array<{ paragraphId: string; offset: number; from: number; to: number }> = []

  editor.state.doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return

    const content = node.text
    let index = 0
    while (index < content.length) {
      const found = content.indexOf(text, index)
      if (found === -1) break

      // 找到包含此文本节点的段落
      const $pos = editor.state.doc.resolve(pos + found)
      let paragraphId = ''
      for (let d = $pos.depth; d >= 0; d--) {
        const ancestor = $pos.node(d)
        if (BLOCK_TYPES.includes(ancestor.type.name)) {
          paragraphId = ancestor.attrs.id || ''
          break
        }
      }

      results.push({
        paragraphId,
        offset: found,
        from: pos + found,
        to: pos + found + text.length,
      })

      index = found + 1
    }
  })

  return results
}
