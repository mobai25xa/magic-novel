/**
 * @author Alpha
 * @date 2026-02-11
 * @description 粘贴净化扩展 — 拦截外部粘贴内容，剥离所有不允许的样式和标签
 *
 * 白名单规则：
 * - 允许的 mark：bold, italic, strike, highlight
 * - 允许的节点属性：id（段落UUID）, level（标题级别）
 * - 允许的块级标签：p, h1-h3, blockquote, hr, br
 * - 移除但保留内容的标签：span, div, font, a, u, table 等
 * - 完全移除的标签：script, style, meta, link, svg, canvas, img
 */
import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Slice, Fragment, Node as ProseMirrorNode } from '@tiptap/pm/model'
import type { Schema } from '@tiptap/pm/model'

const ALLOWED_MARKS = new Set(['bold', 'italic', 'strike', 'highlight'])

const REMOVE_WITH_CONTENT = new Set([
  'script', 'style', 'meta', 'link', 'svg', 'canvas', 'noscript',
  'iframe', 'object', 'embed', 'applet', 'video', 'audio',
])

const UNWRAP_TAGS = new Set([
  'span', 'div', 'font', 'a', 'u', 'sub', 'sup',
  'table', 'tr', 'td', 'th', 'thead', 'tbody', 'tfoot',
  'ul', 'ol', 'li', 'dl', 'dt', 'dd',
  'section', 'article', 'header', 'footer', 'nav', 'aside', 'main',
  'figure', 'figcaption', 'details', 'summary',
  'abbr', 'cite', 'code', 'pre', 'kbd', 'samp', 'var',
  'small', 'big', 'center', 'label', 'form', 'fieldset', 'legend',
])

/**
 * 净化 HTML 字符串，移除所有不允许的标签和属性
 */
function sanitizeHTML(html: string): string {
  const parser = new DOMParser()
  const doc = parser.parseFromString(html, 'text/html')

  // 1. 完全移除危险标签（含内容）
  REMOVE_WITH_CONTENT.forEach(tag => {
    doc.querySelectorAll(tag).forEach(el => el.remove())
  })

  // 2. 解包多余标签（保留文本内容）
  // 需要反复处理，因为解包后可能暴露新的嵌套标签
  let maxIterations = 5
  while (maxIterations-- > 0) {
    let unwrapped = false
    UNWRAP_TAGS.forEach(tag => {
      doc.querySelectorAll(tag).forEach(el => {
        const parent = el.parentNode
        if (parent) {
          while (el.firstChild) {
            parent.insertBefore(el.firstChild, el)
          }
          parent.removeChild(el)
          unwrapped = true
        }
      })
    })
    if (!unwrapped) break
  }

  // 3. 移除所有元素的所有属性（除 data-id）
  doc.body.querySelectorAll('*').forEach(el => {
    const attrs = Array.from(el.attributes)
    for (const attr of attrs) {
      if (attr.name !== 'data-id') {
        el.removeAttribute(attr.name)
      }
    }
  })

  return doc.body.innerHTML
}

/**
 * 净化 ProseMirror Slice，移除不在白名单中的 marks
 */
function sanitizeSlice(slice: Slice, schema: Schema): Slice {
  const sanitizedContent = sanitizeFragment(slice.content, schema)
  return new Slice(sanitizedContent, slice.openStart, slice.openEnd)
}

function sanitizeFragment(fragment: Fragment, schema: Schema): Fragment {
  const nodes: ProseMirrorNode[] = []

  fragment.forEach(node => {
    // 过滤 marks —— 只保留白名单中的
    const allowedMarks = node.marks.filter(mark => ALLOWED_MARKS.has(mark.type.name))

    // 递归清理子节点
    const cleanContent = node.content.size > 0
      ? sanitizeFragment(node.content, schema)
      : node.content

    // 重建节点，保留安全的属性
    if (node.isText) {
      // 文本节点：只需更新 marks
      const newNode = node.mark(allowedMarks)
      nodes.push(newNode)
    } else {
      // 非文本节点：保留 id 和 level 属性，剥离其他
      const safeAttrs: Record<string, unknown> = {}
      if (node.attrs.id) safeAttrs.id = node.attrs.id
      if (node.attrs.level) safeAttrs.level = node.attrs.level

      try {
        const baseAttrs = node.type.createAndFill()?.attrs ?? {}
        const newNode = node.type.create(
          { ...baseAttrs, ...safeAttrs },
          cleanContent,
          allowedMarks,
        )
        nodes.push(newNode)
      } catch {
        // 如果无法创建节点（类型不匹配等），保留子内容
        cleanContent.forEach(child => nodes.push(child))
      }
    }
  })

  return Fragment.from(nodes)
}

export const PasteHandlerExtension = Extension.create({
  name: 'pasteHandler',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('pasteHandler'),
        props: {
          // 拦截 HTML 粘贴 —— 在 ProseMirror 解析之前净化 HTML
          transformPastedHTML(html: string): string {
            return sanitizeHTML(html)
          },
          // 拦截已解析的 Slice —— 最终防线，确保 marks 纯净
          transformPasted(slice: Slice): Slice {
            const { schema } = this as unknown as { schema: Schema }
            return sanitizeSlice(slice, schema)
          },
        },
      }),
    ]
  },
})
