/**
 * @author Alpha
 * @date 2026-02-11
 * @description 自定义键盘快捷键扩展
 *
 * 快捷键列表：
 * - Ctrl+1/2/3 → 切换为 H1/H2/H3 标题
 * - Ctrl+0 → 恢复为正文段落
 * - Ctrl+H → 触发 FIND_REPLACE_OPEN 事件（打开查找替换面板，展开替换行）
 * - F11 → 触发 FULLSCREEN_TOGGLE 事件（全屏/退出全屏）
 */
import { Extension } from '@tiptap/core'
import { eventBus, EVENTS } from '@/lib/events'

export const KeyboardShortcutsExtension = Extension.create({
  name: 'customKeyboardShortcuts',

  addKeyboardShortcuts() {
    return {
      // 标题快捷键
      'Mod-1': () => this.editor.chain().focus().toggleHeading({ level: 1 }).run(),
      'Mod-2': () => this.editor.chain().focus().toggleHeading({ level: 2 }).run(),
      'Mod-3': () => this.editor.chain().focus().toggleHeading({ level: 3 }).run(),

      // 恢复为正文段落
      'Mod-0': () => this.editor.chain().focus().setParagraph().run(),

      // 查找替换（通过事件总线通知 FindReplacePanel 打开并展开替换行）
      'Mod-h': () => {
        eventBus.emit(EVENTS.FIND_REPLACE_OPEN)
        return true
      },

      // 全屏切换（通过事件总线通知 FullscreenMode 组件）
      'F11': () => {
        eventBus.emit(EVENTS.FULLSCREEN_TOGGLE)
        return true
      },
    }
  },
})
