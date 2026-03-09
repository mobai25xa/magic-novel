/**
 * @author Alpha
 * @date 2026-02-11
 * @description UniqueIdExtension — 为所有块级节点分配稳定的 UUID
 *
 * 行为规则：
 * 1. 新创建的节点（id=null）→ 自动分配新 UUID
 * 2. 从 HTML 解析时 → 优先读取已有 data-id，否则分配新 UUID
 * 3. 复制粘贴 → 检测重复 ID，重复的分配新 UUID（保留首次出现的）
 * 4. 剪切粘贴 → 源节点已删除，不会重复，UUID 自然保留
 * 5. 撤销操作 → undo 恢复原始 attrs（含原 UUID），不会被覆盖
 * 6. 段落拆分 → 前半段保留原 UUID，后半段（id=null）获得新 UUID
 */
import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { v4 as uuidv4 } from 'uuid'

const BLOCK_TYPES = ['paragraph', 'heading', 'blockquote']

export const UniqueIdExtension = Extension.create({
  name: 'uniqueId',

  addGlobalAttributes() {
    return BLOCK_TYPES.map((type) => ({
      types: [type],
      attributes: {
        id: {
          default: null,
          // 从 HTML 解析时，优先读取已有的 data-id 属性
          parseHTML: (element: HTMLElement) => {
            return element.getAttribute('data-id') || uuidv4()
          },
          renderHTML: (attributes: Record<string, unknown>) => {
            if (!attributes.id) {
              return {}
            }
            return { 'data-id': attributes.id }
          },
        },
      },
    }))
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('uniqueId'),
        appendTransaction: (transactions, _oldState, newState) => {
          const docChanged = transactions.some((tr) => tr.docChanged)
          if (!docChanged) {
            return null
          }

          const tr = newState.tr
          let modified = false
          const seenIds = new Set<string>()

          newState.doc.descendants((node, pos) => {
            if (!BLOCK_TYPES.includes(node.type.name)) return

            const id = node.attrs.id

            if (!id) {
              // 无 ID（新创建的节点，如段落拆分产生的后半段）→ 分配新 UUID
              tr.setNodeMarkup(pos, undefined, {
                ...node.attrs,
                id: uuidv4(),
              })
              modified = true
            } else if (seenIds.has(id)) {
              // 重复 ID（复制粘贴场景：源节点和粘贴节点有相同 ID）→ 给重复项分配新 UUID
              tr.setNodeMarkup(pos, undefined, {
                ...node.attrs,
                id: uuidv4(),
              })
              modified = true
            } else {
              // 正常 ID → 记录到已见集合
              seenIds.add(id)
            }
          })

          return modified ? tr : null
        },
      }),
    ]
  },
})
