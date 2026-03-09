/**
 * @author Alpha
 * @date 2026-02-11
 * @description Slash Command 扩展 — "/" 触发的命令面板
 *
 * 功能：
 * - 空行或行首输入 "/" 触发命令面板
 * - 支持 /h1 /h2 /h3 /quote /divider /text 六个命令
 * - 支持文字过滤、上下键导航、Enter 确认、Esc 关闭
 * - 段落中间输入 "/" 不触发
 * - 不依赖 tippy.js，使用 ReactRenderer + 手动 DOM 定位
 */
import { Extension } from '@tiptap/core'
import { ReactRenderer } from '@tiptap/react'
import Suggestion from '@tiptap/suggestion'
import type { Editor as CoreEditor } from '@tiptap/core'
import type { SuggestionOptions, SuggestionProps } from '@tiptap/suggestion'
import type { EditorState } from '@tiptap/pm/state'
import {
  useState,
  useEffect,
  useRef,
  useCallback,
  forwardRef,
  useImperativeHandle,
} from 'react'
import { cn } from '@/lib/utils'
import { useTranslation } from '@/hooks/use-translation'
import { SLASH_COMMANDS, type SlashCommandItem } from './slash-command-config'

// ─── 命令列表 UI 组件 ──────────────────────────

interface SlashCommandListRef {
  onKeyDown: (props: { event: KeyboardEvent }) => boolean
}

interface SlashCommandListProps {
  items: SlashCommandItem[]
  command: (item: SlashCommandItem) => void
}

const SlashCommandList = forwardRef<SlashCommandListRef, SlashCommandListProps>(
  ({ items, command }, ref) => {
    const [selectedIndex, setSelectedIndex] = useState(0)
    const listRef = useRef<HTMLDivElement>(null)
    const { translations } = useTranslation()

    useEffect(() => {
      const list = listRef.current
      if (!list) return
      const selected = list.children[selectedIndex] as HTMLElement
      if (selected) {
        selected.scrollIntoView({ block: 'nearest' })
      }
    }, [selectedIndex])

    const selectItem = useCallback(
      (index: number) => {
        const item = items[index]
        if (item) {
          command(item)
        }
      },
      [items, command],
    )

    useImperativeHandle(ref, () => ({
      onKeyDown: ({ event }: { event: KeyboardEvent }) => {
        if (event.key === 'ArrowUp') {
          setSelectedIndex((prev) => (prev - 1 + items.length) % items.length)
          return true
        }
        if (event.key === 'ArrowDown') {
          setSelectedIndex((prev) => (prev + 1) % items.length)
          return true
        }
        if (event.key === 'Enter') {
          selectItem(selectedIndex)
          return true
        }
        return false
      },
    }))

    if (items.length === 0) {
      return (
        <div className="slash-menu py-2 px-3">
          <span className="text-xs" style={{ color: "var(--text-muted-foreground)" }}>{translations.slashCommand.noMatch}</span>
        </div>
      )
    }

    return (
      <div
        ref={listRef}
        className="slash-menu"
      >
        {items.map((item, index) => {
          const Icon = item.icon
          return (
            <button
              key={item.id}
              onClick={() => selectItem(index)}
              className={cn(
                'slash-item',
                index === selectedIndex && 'slash-item-active',
              )}
            >
              <Icon className="h-4 w-4 flex-shrink-0" style={{ color: "var(--text-muted-foreground)" }} />
              <div className="min-w-0">
                <div className="font-medium truncate">{item.label}</div>
                <div className="text-xs truncate" style={{ color: "var(--text-muted-foreground)" }}>{item.description}</div>
              </div>
            </button>
          )
        })}
      </div>
    )
  },
)

SlashCommandList.displayName = 'SlashCommandList'

// ─── Suggestion 渲染：手动 DOM 定位（无 tippy.js 依赖）──

function createSuggestionRenderer() {
  return () => {
    let component: ReactRenderer | null = null
    let popup: HTMLDivElement | null = null

    return {
      onStart: (props: SuggestionProps<SlashCommandItem>) => {
        component = new ReactRenderer(SlashCommandList, {
          props: {
            items: props.items,
            command: props.command,
          },
          editor: props.editor,
        })

        popup = document.createElement('div')
        popup.style.position = 'fixed'
        popup.style.zIndex = '50'
        popup.appendChild(component.element)
        document.body.appendChild(popup)

        updatePosition(popup, props.clientRect)
      },

      onUpdate: (props: SuggestionProps<SlashCommandItem>) => {
        component?.updateProps({
          items: props.items,
          command: props.command,
        })

        updatePosition(popup, props.clientRect)
      },

      onKeyDown: (props: { event: KeyboardEvent }) => {
        if (props.event.key === 'Escape') {
          cleanup()
          return true
        }

        return (component?.ref as SlashCommandListRef)?.onKeyDown(props) ?? false
      },

      onExit: () => {
        cleanup()
      },
    }

    function updatePosition(
      el: HTMLDivElement | null,
      clientRect: (() => DOMRect | null) | null | undefined,
    ) {
      if (!el || !clientRect) return
      const rect = clientRect()
      if (!rect) return

      el.style.left = `${rect.left}px`
      el.style.top = `${rect.bottom + 4}px`
    }

    function cleanup() {
      if (popup) {
        popup.remove()
        popup = null
      }
      component?.destroy()
      component = null
    }
  }
}

// ─── 扩展定义 ─────────────────────────────────

export const SlashCommandExtension = Extension.create({
  name: 'slashCommand',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        startOfLine: false,
        command: ({
          editor,
          range,
          props,
        }: {
          editor: CoreEditor
          range: { from: number; to: number }
          props: SlashCommandItem
        }) => {
          // 删除 "/" 触发字符及后续查询文本
          editor.chain().focus().deleteRange(range).run()
          // 执行命令
          props.action(editor)
        },
        allow: ({ state, range }: { state: EditorState; range: { from: number; to: number } }) => {
          const $from = state.doc.resolve(range.from)
          const textBefore = $from.parent.textBetween(
            0,
            $from.parentOffset,
            undefined,
            '\ufffc',
          )
          // 只在行首或前面只有空白时触发（段落中间不触发）
          const trimmed = textBefore.trim()
          return trimmed === '' || trimmed === '/'
        },
        items: ({ query }: { query: string }) => {
          const q = query.toLowerCase()
          return SLASH_COMMANDS.filter((item) => {
            if (!q) return true
            return (
              item.id.includes(q) ||
              item.label.toLowerCase().includes(q) ||
              item.keywords.some((kw) => kw.includes(q))
            )
          })
        },
        render: createSuggestionRenderer(),
      } as Partial<SuggestionOptions<SlashCommandItem>>,
    }
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        ...this.options.suggestion,
      }),
    ]
  },
})
