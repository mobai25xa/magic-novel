/**
 * @author Beta
 * @date 2026-02-11
 * @description TipTap JSON -> Markdown 转换器
 *
 * 输出要求：
 * - 纯净 Markdown，无 HTML 标签
 * - 无 style 属性
 * - 标题用 # 标记
 * - 粗体用 **，斜体用 *，删除线用 ~~
 * - 引用用 >
 * - 分割线用 ---
 */

interface TiptapNode {
  type: string
  attrs?: Record<string, unknown>
  content?: TiptapNode[]
  text?: string
  marks?: Array<{ type: string; attrs?: Record<string, unknown> }>
}

export function serializeToMarkdown(json: TiptapNode): string {
  if (!json || !json.content) return ''
  return json.content.map(serializeNode).join('\n\n')
}

function serializeNode(node: TiptapNode): string {
  switch (node.type) {
    case 'heading': {
      const level = (node.attrs?.level as number) || 1
      const prefix = '#'.repeat(level)
      return `${prefix} ${serializeInline(node.content)}`
    }

    case 'paragraph':
      return serializeInline(node.content)

    case 'blockquote': {
      const inner = node.content?.map(serializeNode).join('\n') || ''
      return inner
        .split('\n')
        .map((line) => `> ${line}`)
        .join('\n')
    }

    case 'horizontalRule':
      return '---'

    case 'hardBreak':
      return '  \n'

    default:
      return serializeInline(node.content)
  }
}

function serializeInline(content: TiptapNode[] | undefined): string {
  if (!content) return ''
  return content
    .map((node) => {
      if (node.type === 'text') {
        let text = node.text || ''
        const marks = node.marks || []

        // 按嵌套顺序包裹 marks
        for (const mark of marks) {
          switch (mark.type) {
            case 'bold':
              text = `**${text}**`
              break
            case 'italic':
              text = `*${text}*`
              break
            case 'strike':
              text = `~~${text}~~`
              break
          }
        }

        return text
      }
      if (node.type === 'hardBreak') {
        return '  \n'
      }
      return ''
    })
    .join('')
}
