# Developer Alpha 任务清单：编辑器核心引擎

> 负责人：Alpha
> 分支名：`feat/alpha-editor-core`
> 核心文件：`src/components/editor/extensions/` 目录

---

## 任务总览

```
编号    任务                       优先级   覆盖验收项                          预估复杂度
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
A-1    UniqueIdExtension 重构      P0      T-8.1.4~T-8.1.10, T-1.4.5~6       ★★★★☆
A-2    粘贴净化扩展                P0      T-1.3.3~5, T-8.3.2~4              ★★★★☆
A-3    查找替换重构                P0      T-3.2(bug), T-3.4, T-3.6, T-3.10  ★★★★☆
A-4    Slash Command 系统          P1      T-2.3.1~T-2.3.7                   ★★★★★
A-5    键盘快捷键扩展             P1      T-6.1, T-2.1.3, T-2.1.5           ★★★☆☆
A-6    撤销深度配置               P2      T-1.4.3                            ★☆☆☆☆
A-7    空行段落间距优化            P2      T-1.1.7                            ★☆☆☆☆
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**执行顺序：A-1 → A-2 → A-3 → A-5 → A-4 → A-6 → A-7**
（A-1 是所有 ID 行为的基础，必须最先完成）

---

## A-1：UniqueIdExtension 重构

### 问题分析

当前 `src/components/editor/extensions/unique-id.ts` 有以下缺陷：

```typescript
// 当前实现（有问题）
parseHTML: () => uuidv4(),  // 第16行：始终生成新 UUID，忽略已有 data-id
```

这导致：
- 从 HTML 粘贴时所有段落都获得新 UUID（即使是剪切粘贴也不保留原 ID）
- 从 HTML 加载内容时 UUID 丢失
- 复制粘贴"碰巧"正确但不是设计意图

### 实现要求

#### 1. 修复 `parseHTML`

```typescript
// 修复后应该是：
parseHTML: (element: HTMLElement) => {
  return element.getAttribute('data-id') || uuidv4()
},
```

#### 2. 处理复制粘贴的 UUID 去重

粘贴时需要区分：
- **剪切粘贴**：保留原 UUID（因为源节点已删除，不会重复）
- **复制粘贴**：生成新 UUID（因为源节点仍存在，会重复）

实现方案：在 `appendTransaction` 中检测重复 ID 并替换：

```typescript
appendTransaction: (transactions, oldState, newState) => {
  const docChanged = transactions.some((tr) => tr.docChanged)
  if (!docChanged) return null

  const tr = newState.tr
  let modified = false
  const seenIds = new Set<string>()

  newState.doc.descendants((node, pos) => {
    if (!BLOCK_TYPES.includes(node.type.name)) return

    const id = node.attrs.id

    if (!id) {
      // 无 ID → 赋新 UUID
      tr.setNodeMarkup(pos, undefined, { ...node.attrs, id: uuidv4() })
      modified = true
    } else if (seenIds.has(id)) {
      // 重复 ID → 赋新 UUID（复制粘贴场景）
      tr.setNodeMarkup(pos, undefined, { ...node.attrs, id: uuidv4() })
      modified = true
    } else {
      seenIds.add(id)
    }
  })

  return modified ? tr : null
},
```

#### 3. 确保 undo 不被干扰

`appendTransaction` 在 undo 后也会运行。undo 恢复的节点会带有原始 UUID，由于该 UUID 不在当前文档中（刚被 undo 恢复），不会触发去重，因此 UUID 正确保留。

需要验证这一假设。

### 自测项

```
☐ 创建5个段落 → 每个有唯一 UUID → Console 验证
☐ 在段落2和3之间插入 → 原段落 UUID 不变
☐ 删除段落3 → 其他段落 UUID 不变
☐ 复制段落2 → 粘贴 → 新段落有新 UUID，原段落 UUID 不变
☐ 剪切段落2 → 粘贴到段落4后 → 保留原 UUID
☐ 撤销删除段落 → UUID 与删除前一致
☐ 跨3段选中删除 → 合并后段落 UUID = 第一段的 UUID
☐ 段落中间按回车 → 前半段保留原 UUID，后半段新 UUID
☐ 保存 → 关闭 → 重新打开 → UUID 完全一致
☐ 从外部粘贴5段文字 → 每段新 UUID，不与已有冲突
```

### 交付文件

```
src/components/editor/extensions/unique-id.ts  ← 重构
```

---

## A-2：粘贴净化扩展

### 问题分析

当前编辑器没有任何粘贴拦截逻辑。从 Word、网页等外部来源粘贴的内容会携带 CSS 样式、字体信息、颜色等，污染纯净 JSON。

### 实现要求

新建 `src/components/editor/extensions/paste-handler.ts`：

```typescript
import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { DOMParser as ProseMirrorParser } from '@tiptap/pm/model'
import { Slice } from '@tiptap/pm/model'

// 允许的 mark 类型白名单
const ALLOWED_MARKS = ['bold', 'italic', 'strike', 'highlight']

// 允许的节点属性白名单
const ALLOWED_ATTRS = ['id', 'level'] // id 用于段落 UUID，level 用于标题级别

export const PasteHandlerExtension = Extension.create({
  name: 'pasteHandler',

  addProseMirrorPlugins() {
    const { schema } = this.editor

    return [
      new Plugin({
        key: new PluginKey('pasteHandler'),
        props: {
          // 拦截 HTML 粘贴
          transformPastedHTML(html: string): string {
            return sanitizeHTML(html)
          },
          // 拦截 Slice 粘贴（最终防线）
          transformPasted(slice: Slice): Slice {
            return sanitizeSlice(slice, schema)
          },
        },
      }),
    ]
  },
})
```

#### `sanitizeHTML` 函数实现要点

```typescript
function sanitizeHTML(html: string): string {
  const doc = new DOMParser().parseFromString(html, 'text/html')

  // 1. 移除所有 style 属性
  doc.querySelectorAll('[style]').forEach(el => el.removeAttribute('style'))

  // 2. 移除所有 class 属性
  doc.querySelectorAll('[class]').forEach(el => el.removeAttribute('class'))

  // 3. 移除不允许的标签，保留内容
  //    允许：p, h1, h2, h3, strong/b, em/i, s/del, br, blockquote, hr
  //    移除但保留内容：span, div, font, a, u, etc.
  //    完全移除：script, style, meta, link, img（Phase 1 不支持图片）
  const REMOVE_WITH_CONTENT = ['script', 'style', 'meta', 'link', 'svg', 'canvas']
  const UNWRAP_TAGS = ['span', 'div', 'font', 'a', 'u', 'sub', 'sup', 'table',
                       'tr', 'td', 'th', 'thead', 'tbody', 'tfoot',
                       'ul', 'ol', 'li', 'dl', 'dt', 'dd', 'section',
                       'article', 'header', 'footer', 'nav', 'aside', 'main']

  // 移除危险标签
  REMOVE_WITH_CONTENT.forEach(tag => {
    doc.querySelectorAll(tag).forEach(el => el.remove())
  })

  // 解包多余标签（保留内容）
  UNWRAP_TAGS.forEach(tag => {
    doc.querySelectorAll(tag).forEach(el => {
      const parent = el.parentNode
      if (parent) {
        while (el.firstChild) {
          parent.insertBefore(el.firstChild, el)
        }
        parent.removeChild(el)
      }
    })
  })

  // 4. 移除所有剩余的 style/class/id/data-* 属性（除 data-id）
  doc.querySelectorAll('*').forEach(el => {
    const attrs = Array.from(el.attributes)
    attrs.forEach(attr => {
      if (attr.name !== 'data-id' && attr.name !== 'level') {
        el.removeAttribute(attr.name)
      }
    })
  })

  return doc.body.innerHTML
}
```

#### `sanitizeSlice` 函数实现要点

```typescript
function sanitizeSlice(slice: Slice, schema: Schema): Slice {
  // 遍历 slice 中的所有节点，移除不在白名单中的 marks 和 attrs
  const sanitizedFragment = sanitizeFragment(slice.content, schema)
  return new Slice(sanitizedFragment, slice.openStart, slice.openEnd)
}

function sanitizeFragment(fragment: Fragment, schema: Schema): Fragment {
  const nodes: ProseMirrorNode[] = []

  fragment.forEach(node => {
    // 过滤 marks
    const allowedMarks = node.marks.filter(mark =>
      ALLOWED_MARKS.includes(mark.type.name)
    )

    // 清理 attrs（只保留白名单）
    const cleanAttrs: Record<string, unknown> = {}
    for (const key of ALLOWED_ATTRS) {
      if (node.attrs[key] !== undefined && node.attrs[key] !== null) {
        cleanAttrs[key] = node.attrs[key]
      }
    }

    // 递归清理子节点
    const cleanContent = node.content.size > 0
      ? sanitizeFragment(node.content, schema)
      : node.content

    // 重建节点
    const cleanNode = node.type.create(
      { ...node.type.defaultAttrs, ...cleanAttrs },
      cleanContent,
      allowedMarks
    )

    nodes.push(cleanNode)
  })

  return Fragment.from(nodes)
}
```

### 自测项

```
☐ 从记事本粘贴3段纯文本 → 正确分段，每段有 UUID
☐ 从 Word 粘贴彩色加粗文字 → 只保留粗体，无颜色
☐ 从网页粘贴带链接的文字 → 链接被去除，文字保留
☐ 粘贴后 Console 执行 editor.getJSON() → 无 style/class/color/font 字段
☐ 粘贴后保存的 JSON 文件中搜索 "style" → 0 结果
☐ 粘贴 <script>alert(1)</script> → 不执行，不残留
☐ 从 Excel 粘贴表格 → 变为纯文本段落
```

### 交付文件

```
src/components/editor/extensions/paste-handler.ts  ← 新建
```

---

## A-3：查找替换重构

### 问题分析

当前 `FindReplacePanel.tsx` 有以下问题：

1. **致命 Bug**：使用 `doc.textContent` 的纯文本 index 计算 ProseMirror 位置，但 ProseMirror 位置体系中节点之间有额外偏移（每个块级节点的开始/结束标记各占1个位置），导致搜索跳转位置错误
2. 缺少所有匹配项的背景高亮（当前只高亮当前项）
3. 缺少 Ctrl+H 快捷键
4. 缺少正则搜索
5. 全部替换后的撤销行为未验证

### 实现要求

#### 1. 修复位置计算

使用 `doc.descendants` 遍历来计算正确的 ProseMirror 位置：

```typescript
function findAllMatches(
  doc: ProseMirrorNode,
  searchText: string,
  caseSensitive: boolean
): Array<{ from: number; to: number }> {
  const matches: Array<{ from: number; to: number }> = []
  const search = caseSensitive ? searchText : searchText.toLowerCase()

  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return

    const text = caseSensitive ? node.text : node.text.toLowerCase()
    let index = 0

    while (index < text.length) {
      const foundIndex = text.indexOf(search, index)
      if (foundIndex === -1) break

      matches.push({
        from: pos + foundIndex,
        to: pos + foundIndex + searchText.length,
      })

      index = foundIndex + 1
    }
  })

  return matches
}
```

#### 2. 所有匹配项背景高亮

使用 TipTap 的 Decoration 来为所有匹配项添加背景色：

```typescript
// 方案 A：使用 ProseMirror decorations（推荐）
import { DecorationSet, Decoration } from '@tiptap/pm/view'

// 在搜索时创建 decorations：
function createSearchDecorations(
  matches: Array<{ from: number; to: number }>,
  currentIndex: number
): DecorationSet {
  const decorations = matches.map((match, i) => {
    const className = i === currentIndex
      ? 'search-match-current'  // 当前匹配：橙色背景
      : 'search-match'          // 其他匹配：黄色背景
    return Decoration.inline(match.from, match.to, { class: className })
  })
  return DecorationSet.create(doc, decorations)
}
```

在 `editor.css` 中添加样式：

```css
.search-match {
  background-color: rgba(254, 240, 138, 0.7); /* 黄色 */
}
.search-match-current {
  background-color: rgba(251, 146, 60, 0.7); /* 橙色 */
}
```

**实现方式**：将搜索高亮作为一个 ProseMirror Plugin 实现，在 FindReplacePanel 控制开/关和 decorations 更新。

#### 3. 添加 Ctrl+H 快捷键

在 NovelEditor.tsx 的 keydown 监听中（或通过 keyboard-shortcuts 扩展）：

```typescript
if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
  e.preventDefault()
  setShowFindReplace(true)
  // 同时确保替换行展开
}
```

**注意**：Ctrl+H 由 A-5 的快捷键扩展通过事件发射实现，FindReplacePanel 监听 `EVENTS.FIND_REPLACE_OPEN` 来打开并展开替换行。

#### 4. 正则搜索（可选但推荐）

添加正则开关按钮：

```typescript
const [useRegex, setUseRegex] = useState(false)

// 搜索时
if (useRegex) {
  try {
    const regex = new RegExp(searchText, caseSensitive ? 'g' : 'gi')
    // 使用 regex.exec() 遍历文本
  } catch {
    // 正则语法错误 → 不搜索，显示错误提示
  }
} else {
  // 使用 indexOf
}
```

#### 5. 全部替换可撤销

确保 `handleReplaceAll` 在单个 transaction 中完成所有替换：

```typescript
const handleReplaceAll = () => {
  // 当前已经在一个 tr 中完成，但需要确保 dispatch 后不会产生多个 undo 步骤
  // 验证：全部替换后按一次 Ctrl+Z → 所有替换都撤销
}
```

### 自测项

```
☐ 搜索 "他说" → 所有匹配项黄色高亮，当前项橙色
☐ Enter 跳到下一个 → 视图滚动到该位置
☐ Shift+Enter 跳到上一个
☐ Ctrl+H → 打开面板且替换行展开
☐ 替换当前 → 替换并跳到下一个
☐ 全部替换 → 显示替换数量，Ctrl+Z 一次撤销全部
☐ 大小写开关 → 搜索结果实时更新
☐ 正则搜索 "他[说道]+" → 匹配"他说""他道""他说道"
☐ Esc 关闭 → 高亮消失，光标回到之前位置
☐ 空文档搜索 → 不崩溃
☐ 跨段落的搜索位置准确（在第3段中搜索到的词，点击确实跳到第3段）
```

### 交付文件

```
src/components/editor/FindReplacePanel.tsx  ← 重构
src/styles/editor.css                       ← 添加搜索高亮样式（协调 Gamma）
```

---

## A-4：Slash Command 系统

### 实现要求

新建 `src/components/editor/extensions/slash-command.tsx`

需要安装依赖：`@tiptap/suggestion`（已在 guide.md 中批准）

```bash
pnpm add @tiptap/suggestion
```

#### 核心架构

```
SlashCommandExtension (TipTap Extension)
  └── 使用 @tiptap/suggestion 的 Suggestion 插件
      └── 触发字符: "/"
      └── 触发条件: 行首或空段落（段落中间不触发）
      └── 渲染: React 浮动面板
```

#### 命令列表

```typescript
const SLASH_COMMANDS = [
  { id: 'h1',      label: '标题 1',   description: '章节标题',     icon: Heading1,    action: (editor) => editor.chain().focus().toggleHeading({ level: 1 }).run() },
  { id: 'h2',      label: '标题 2',   description: '节标题',       icon: Heading2,    action: (editor) => editor.chain().focus().toggleHeading({ level: 2 }).run() },
  { id: 'h3',      label: '标题 3',   description: '子标题',       icon: Heading3,    action: (editor) => editor.chain().focus().toggleHeading({ level: 3 }).run() },
  { id: 'text',    label: '正文',     description: '恢复为正文',   icon: Type,        action: (editor) => editor.chain().focus().setParagraph().run() },
  { id: 'quote',   label: '引用',     description: '引用块',       icon: Quote,       action: (editor) => editor.chain().focus().toggleBlockquote().run() },
  { id: 'divider', label: '分割线',   description: '水平分割线',   icon: Minus,       action: (editor) => editor.chain().focus().setHorizontalRule().run() },
]
```

#### 触发条件检查

```typescript
// 在 suggestion 的 allow 回调中
allow: ({ state, range }) => {
  const { $from } = state.selection
  const textBefore = $from.parent.textBetween(
    0,
    $from.parentOffset,
    undefined,
    '\ufffc'
  )

  // 只在行首（前面无其他字符）或空段落触发
  // "/" 前面只允许有空格
  const trimmed = textBefore.trimStart()
  return trimmed === '/' || trimmed === ''
}
```

#### UI 渲染（React 组件）

```tsx
function SlashCommandList({ items, command, selectedIndex }) {
  return (
    <div className="bg-card border border-border rounded-lg shadow-lg py-1 w-56 max-h-64 overflow-y-auto">
      {items.map((item, index) => (
        <button
          key={item.id}
          onClick={() => command(item)}
          className={cn(
            "w-full flex items-center gap-3 px-3 py-2 text-sm text-left hover:bg-accent",
            index === selectedIndex && "bg-accent"
          )}
        >
          <item.icon className="h-4 w-4 text-muted-foreground" />
          <div>
            <div className="font-medium">{item.label}</div>
            <div className="text-xs text-muted-foreground">{item.description}</div>
          </div>
        </button>
      ))}
    </div>
  )
}
```

#### 键盘导航

- 上/下键：切换选中项
- Enter：执行选中命令
- Esc：关闭面板
- 继续输入：过滤列表（如 "/h" 只显示 h1/h2/h3）

### 自测项

```
☐ 空段落输入 "/" → 弹出命令面板
☐ 输入 "/h" → 列表过滤，只显示 h1/h2/h3
☐ 上下键切换 → 选中项高亮
☐ 回车确认 → 执行对应命令（如段落变为 H1）
☐ Esc → 关闭面板
☐ 段落行首输入 "/" → 弹出
☐ 段落中间输入 "/" → 不弹出
☐ 选择 /divider → 插入水平线
```

### 交付文件

```
src/components/editor/extensions/slash-command.tsx  ← 新建
```

---

## A-5：键盘快捷键扩展

### 实现要求

新建 `src/components/editor/extensions/keyboard-shortcuts.ts`

```typescript
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
      'Mod-0': () => this.editor.chain().focus().setParagraph().run(),

      // 查找替换（通过事件总线通知 UI 层）
      'Mod-h': () => {
        eventBus.emit(EVENTS.FIND_REPLACE_OPEN)
        return true
      },

      // 全屏切换
      'F11': () => {
        eventBus.emit(EVENTS.FULLSCREEN_TOGGLE)
        return true
      },
    }
  },
})
```

### 事件常量

需要在 `src/lib/events.ts` 的 `EVENTS` 对象中添加：

```typescript
FIND_REPLACE_OPEN: 'find-replace-open',
FULLSCREEN_TOGGLE: 'fullscreen-toggle',
```

### 在 NovelEditor.tsx 中注册

在 `extensions` 数组中添加：

```typescript
// === Alpha: Extensions Registration ===
import { KeyboardShortcutsExtension } from './extensions/keyboard-shortcuts'
import { PasteHandlerExtension } from './extensions/paste-handler'
import { SlashCommandExtension } from './extensions/slash-command'

extensions: [
  StarterKit.configure({ ... }),
  Highlight.configure({ ... }),
  UniqueIdExtension,
  PasteHandlerExtension,       // Alpha
  SlashCommandExtension,        // Alpha
  KeyboardShortcutsExtension,   // Alpha
],
```

### 自测项

```
☐ Ctrl+1 → 当前段落变为 H1
☐ Ctrl+2 → H2
☐ Ctrl+3 → H3
☐ Ctrl+0 → 恢复为正文段落
☐ Ctrl+H → 查找替换面板打开（替换行展开）
☐ F11 → 全屏切换事件发射（Gamma 实现监听）
☐ 以上快捷键不与中文输入法冲突
☐ 以上快捷键不与 Tauri 系统快捷键冲突
```

### 交付文件

```
src/components/editor/extensions/keyboard-shortcuts.ts  ← 新建
src/lib/events.ts                                        ← 修改（添加事件常量）
```

---

## A-6：撤销深度配置

### 实现要求

在 `NovelEditor.tsx` 的 StarterKit 配置中设置 history 深度：

```typescript
StarterKit.configure({
  heading: { levels: [1, 2, 3] },
  history: {
    depth: 200,  // 确保 ≥ 100 步，设为 200 留余量
  },
}),
```

### 自测项

```
☐ 输入200个字符（每个一步） → Ctrl+Z 连续撤销 150 步 → 不崩溃，每步正确
```

### 交付文件

```
src/components/editor/NovelEditor.tsx  ← 修改 StarterKit 配置（仅 history 字段）
```

---

## A-7：空行段落间距优化

### 实现要求

在 `src/styles/editor.css` 中调整段落间距：

```css
.ProseMirror p {
  margin: 0.75em 0;  /* 从 0.5em 增大到 0.75em */
}

/* 空段落有最小高度，确保可见 */
.ProseMirror p:empty::before {
  content: '\00a0';  /* 不可见空格，确保空段落有行高 */
}
```

### 自测项

```
☐ 连续两次回车 → 产生一个空段落，视觉上有明显间距
☐ 空段落有 UUID（通过 Console 验证）
☐ 空段落可以点击并输入文字
```

### 交付文件

```
src/styles/editor.css  ← 修改（协调 Gamma，只改段落间距部分）
```

---

## NovelEditor.tsx 集成指南

Alpha 在 NovelEditor.tsx 中的修改范围（用注释标记）：

```tsx
// === Alpha: Extensions Registration START ===
import { PasteHandlerExtension } from './extensions/paste-handler'
import { SlashCommandExtension } from './extensions/slash-command'
import { KeyboardShortcutsExtension } from './extensions/keyboard-shortcuts'
// === Alpha: Extensions Registration END ===

const editor = useEditor({
  extensions: [
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
      history: { depth: 200 },          // A-6
    }),
    Highlight.configure({ multicolor: true }),
    UniqueIdExtension,                   // A-1（重构后）
    PasteHandlerExtension,              // A-2
    SlashCommandExtension,              // A-4
    KeyboardShortcutsExtension,         // A-5
  ],
  // ... 其余不变
})
```

**注意**：不要修改 NovelEditor.tsx 中 Alpha 标记区域外的任何代码。
