/**
 * @author Beta
 * @date 2026-02-11
 * @description Headless Editor API — 可在 Console 中直接调用
 */
import type { Editor } from '@tiptap/react'
import {
  operationGetJSON,
  operationGetMarkdown,
  operationGetText,
  operationGetWordCount,
  operationGetAllParagraphIds,
  operationGetParagraphText,
  operationGetCursorPosition,
  operationInsertText,
  operationReplaceText,
  operationInsertParagraphAfter,
  operationDeleteParagraph,
  operationMoveCursorToParagraph,
  operationMoveCursorToEnd,
} from './operations'

export class EditorAPI {
  private editor: Editor

  constructor(editor: Editor) {
    this.editor = editor
  }

  /** 更新底层 editor 实例（用于 editor 重建时） */
  updateEditor(editor: Editor) {
    this.editor = editor
  }

  // ─── 读取操作 ────────────────────────────────────

  /**
   * 获取编辑器内容
   * @param format - 'json' | 'markdown' | 'text'
   */
  getContent(format: 'json' | 'markdown' | 'text'): unknown {
    switch (format) {
      case 'json':
        return operationGetJSON(this.editor)
      case 'markdown':
        return operationGetMarkdown(this.editor)
      case 'text':
        return operationGetText(this.editor)
      default:
        throw new Error(`Unknown format: ${format}`)
    }
  }

  /** 获取字数（不含空白） */
  getWordCount(): number {
    return operationGetWordCount(this.editor)
  }

  /** 获取所有段落 ID */
  getAllParagraphIds(): string[] {
    return operationGetAllParagraphIds(this.editor)
  }

  /** 获取指定段落的纯文本 */
  getParagraphText(id: string): string | null {
    return operationGetParagraphText(this.editor, id)
  }

  /** 获取光标位置 */
  getCursorPosition(): { paragraphId: string | null; offset: number } {
    return operationGetCursorPosition(this.editor)
  }

  // ─── 写入操作 ────────────────────────────────────

  /** 在光标位置插入文字 */
  insertText(text: string): void {
    operationInsertText(this.editor, text)
  }

  /** 替换指定段落中的文字 */
  replaceText(paragraphId: string, oldText: string, newText: string): boolean {
    return operationReplaceText(this.editor, paragraphId, oldText, newText)
  }

  /** 在指定段落后插入新段落 */
  insertParagraphAfter(paragraphId: string, content: string): string {
    return operationInsertParagraphAfter(this.editor, paragraphId, content)
  }

  /** 删除指定段落 */
  deleteParagraph(paragraphId: string): boolean {
    return operationDeleteParagraph(this.editor, paragraphId)
  }

  // ─── 光标操作 ────────────────────────────────────

  /** 光标跳转到指定段落开头 */
  moveCursorToParagraph(paragraphId: string): void {
    operationMoveCursorToParagraph(this.editor, paragraphId)
  }

  /** 光标跳转到文档末尾 */
  moveCursorToEnd(): void {
    operationMoveCursorToEnd(this.editor)
  }
}
