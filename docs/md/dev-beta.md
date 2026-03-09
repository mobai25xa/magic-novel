# Developer Beta 任务清单：API 与数据安全

> 负责人：Beta
> 分支名：`feat/beta-api-safety`
> 核心文件：`src/lib/editor-api.ts`、`src/lib/operations.ts`、后端 `src-tauri/src/`

---

## 任务总览

```
编号    任务                         优先级   覆盖验收项                        预估复杂度
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
B-1    Headless API (editor.api)     P0      T-8.2.1~T-8.2.3                  ★★★★★
B-2    原子化操作模块                P0      T-8.4.1~T-8.4.3                  ★★★★☆
B-3    Markdown 序列化器             P1      T-4.1.4, T-8.3.5                 ★★★☆☆
B-4    双格式保存（后端）            P1      T-4.1.4                           ★★★☆☆
B-5    关闭窗口保护                 P0      T-4.4.1, T-4.4.2                 ★★★☆☆
B-6    章节切换保护                 P1      T-4.2.3, T-4.3.2                 ★★☆☆☆
B-7    启动恢复上次文件             P2      T-4.2.1                           ★★☆☆☆
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**执行顺序：B-2 → B-1 → B-3 → B-4 → B-5 → B-6 → B-7**
（B-2 的原子化操作是 B-1 Headless API 的底层基础）

---

## B-1：Headless API (editor.api)

### 问题分析

验收标准 T-8.2 要求一套完整的可在 Console 中调用的 API：`editor.api.xxx()`。当前只有 `window.__manualSave` 暴露到全局，其余 API 完全不存在。

### 实现要求

新建 `src/lib/editor-api.ts`：

```typescript
/**
 * @author Beta
 * @date 2026-02-XX
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
```

#### 在 NovelEditor.tsx 中挂载

在 Beta 标记区域内添加：

```typescript
// === Beta: API Mount START ===
import { EditorAPI } from '@/lib/editor-api'

useEffect(() => {
  if (editor) {
    const api = new EditorAPI(editor)
    ;(window as any).editor = { api }
  }
  return () => {
    delete (window as any).editor
  }
}, [editor])
// === Beta: API Mount END ===
```

### 关键要求

1. **所有 API 操作必须通过 editor 的 transaction 系统**，确保和手动操作共享同一个 undo 栈
2. Console 中执行 `editor.api.insertText("test")` 后，`Ctrl+Z` 可以撤销
3. API 操作后编辑器 UI 实时更新

### 自测项（在 Console 中执行）

```javascript
// 读取测试
editor.api.getContent('json')         // → 返回 JSON 对象
editor.api.getContent('markdown')     // → 返回 Markdown 字符串
editor.api.getContent('text')         // → 返回纯文本
editor.api.getWordCount()             // → 返回数字，与状态栏一致
editor.api.getAllParagraphIds()        // → 返回字符串数组
editor.api.getParagraphText('某ID')   // → 返回文字
editor.api.getCursorPosition()        // → 返回 { paragraphId, offset }

// 写入测试
editor.api.insertText("测试文字")     // → 光标位置出现文字
// Ctrl+Z → 文字消失

const ids = editor.api.getAllParagraphIds()
editor.api.replaceText(ids[0], "旧", "新")  // → 第一段中"旧"被替换为"新"

const newId = editor.api.insertParagraphAfter(ids[0], "新段落")
// → 第一段后出现新段落

editor.api.deleteParagraph(newId)
// → 新段落消失

// 光标测试
editor.api.moveCursorToParagraph(ids[1])  // → 光标跳到第2段
editor.api.moveCursorToEnd()               // → 光标跳到文档末尾

// 连续操作测试：连续执行 10 个操作 → 每个都正确反映在 UI 上
```

### 交付文件

```
src/lib/editor-api.ts  ← 新建
```

---

## B-2：原子化操作模块

### 问题分析

验收标准 T-8.4 要求所有操作是独立函数，不依赖 React 状态，可在纯 Node.js 环境中导入（理论上）。当前所有操作都内嵌在 React 组件和 hooks 中。

### 实现要求

新建 `src/lib/operations.ts`：

```typescript
/**
 * @author Beta
 * @date 2026-02-XX
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

// ─── 读取操作 ──────────────────────────────────────

/** 获取完整 JSON 内容 */
export function operationGetJSON(editor: Editor): object {
  return editor.getJSON()
}

/** 获取 Markdown 内容 */
export function operationGetMarkdown(editor: Editor): string {
  // 依赖 B-3 的 markdown-serializer
  // 如果 tiptap-markdown 扩展已安装：
  //   return editor.storage.markdown.getMarkdown()
  // 否则使用自定义序列化器：
  const { serializeToMarkdown } = require('./markdown-serializer')
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
    if (['paragraph', 'heading', 'blockquote'].includes(node.type.name) && node.attrs.id) {
      ids.push(node.attrs.id)
    }
  })
  return ids
}

/** 获取指定段落的纯文本 */
export function operationGetParagraphText(editor: Editor, id: string): string | null {
  let result: string | null = null
  editor.state.doc.descendants((node) => {
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
  let offset = $from.parentOffset

  for (let depth = $from.depth; depth >= 0; depth--) {
    const node = $from.node(depth)
    if (['paragraph', 'heading', 'blockquote'].includes(node.type.name)) {
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
  // 新段落位于 insertPos 位置
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
        if (['paragraph', 'heading', 'blockquote'].includes(ancestor.type.name)) {
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
```

### 关键设计约束

1. **不 import React 的任何东西** — 确保理论上可在 Node.js 中导入
2. **不从 Zustand store 读取** — 所有状态通过 Editor 实例获取
3. **所有写入操作使用 editor.chain() 或 editor.view.dispatch(tr)** — 保证进入 undo 栈
4. **每个函数是纯函数或副作用隔离在 Editor 内** — 同一输入同一输出

### 自测项

```javascript
// 在 Console 中通过 window.editor.api 的底层调用测试

// 或直接导入模块测试（如果有 HMR 的话）：
// import { operationGetWordCount } from './lib/operations'
// const editor = window.__tiptapEditor  // 假设有暴露

// 验证：React 按钮调用 vs Console 调用 vs 直接函数调用 → 结果一致
// 验证：操作函数不依赖 React 状态
// 验证：TypeScript 编译通过，无 React import
```

### 交付文件

```
src/lib/operations.ts  ← 新建
```

---

## B-3：Markdown 序列化器

### 实现要求

新建 `src/lib/markdown-serializer.ts`

两种方案（选其一）：

#### 方案 A：使用 tiptap-markdown 扩展（推荐）

```bash
pnpm add tiptap-markdown
```

```typescript
// 在 NovelEditor.tsx 中注册
import { Markdown } from 'tiptap-markdown'

extensions: [
  // ...
  Markdown.configure({
    html: false,           // 不输出 HTML 标签
    tightLists: true,
    bulletListMarker: '-',
  }),
]

// 在 operations.ts 中使用
export function operationGetMarkdown(editor: Editor): string {
  return editor.storage.markdown.getMarkdown()
}
```

#### 方案 B：自行实现序列化器

```typescript
/**
 * @author Beta
 * @description TipTap JSON → Markdown 转换器
 */

export function serializeToMarkdown(json: any): string {
  if (!json || !json.content) return ''
  return json.content.map(serializeNode).join('\n\n')
}

function serializeNode(node: any): string {
  switch (node.type) {
    case 'heading':
      const prefix = '#'.repeat(node.attrs?.level || 1)
      return `${prefix} ${serializeInline(node.content)}`

    case 'paragraph':
      return serializeInline(node.content)

    case 'blockquote':
      const inner = node.content?.map(serializeNode).join('\n') || ''
      return inner.split('\n').map(line => `> ${line}`).join('\n')

    case 'horizontalRule':
      return '---'

    case 'hardBreak':
      return '  \n'

    default:
      return serializeInline(node.content)
  }
}

function serializeInline(content: any[] | undefined): string {
  if (!content) return ''
  return content.map(node => {
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
  }).join('')
}
```

### 输出要求

- 纯净 Markdown，无 HTML 标签
- 无 style 属性
- 标题用 `#` 标记
- 粗体用 `**`，斜体用 `*`，删除线用 `~~`
- 引用用 `>`
- 分割线用 `---`

### 自测项

```
☐ 纯文本段落 → 正确输出
☐ H1 标题 → "# 标题"
☐ 粗体文字 → "**粗体**"
☐ 粗斜体 → "***粗斜体***"
☐ 引用块 → "> 引用内容"
☐ 分割线 → "---"
☐ 输出无 HTML 标签
☐ 输出无 style 属性
```

### 交付文件

```
src/lib/markdown-serializer.ts  ← 新建
```

---

## B-4：双格式保存（后端）

### 问题分析

当前 `save_chapter` 只保存 JSON 文件。验收要求同时保存 `.tiptap.json`（编辑器格式）和 `.md`（Markdown 格式）。

### 实现方案

在前端 `performSave` 中，保存 JSON 后额外保存 Markdown：

#### 前端修改 `src/hooks/use-auto-save.ts`

```typescript
// 在 performSave 函数中，保存 JSON 成功后
const content = editor.getJSON()
await saveChapter(projectPath, currentChapterPath, content)

// 额外保存 Markdown 文件
const markdown = operationGetMarkdown(editor)
const mdPath = currentChapterPath.replace(/\.json$/, '.md')
await saveChapterMarkdown(projectPath, mdPath, markdown)
```

#### 后端新增命令 `src-tauri/src/commands/chapter.rs`

```rust
#[command]
pub async fn save_chapter_markdown(
    project_path: String,
    markdown_path: String,
    content: String,
) -> Result<(), AppError> {
    let full_path = PathBuf::from(&project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&markdown_path);

    std::fs::write(&full_path, content)?;
    Ok(())
}
```

#### 前端注册命令 `src/lib/tauri-commands.ts`

```typescript
export async function saveChapterMarkdown(
  projectPath: string,
  markdownPath: string,
  content: string
): Promise<void> {
  return invoke('save_chapter_markdown', { projectPath, markdownPath, content })
}
```

### 自测项

```
☐ Ctrl+S 保存 → 在文件管理器中看到 {chapter_id}.json 和 {chapter_id}.md
☐ .json 文件内容纯净，无 CSS
☐ .md 文件可读，内容与 JSON 一致
☐ 自动保存也产生 .md 文件
```

### 交付文件

```
src/hooks/use-auto-save.ts               ← 修改（添加 Markdown 保存调用）
src/lib/tauri-commands.ts                 ← 添加 saveChapterMarkdown 函数
src-tauri/src/commands/chapter.rs         ← 添加 save_chapter_markdown 命令
src-tauri/src/lib.rs                      ← 注册新命令
```

---

## B-5：关闭窗口保护

### 问题分析

`WindowControls.tsx` 的 `handleClose` 直接调用 `appWindow.close()`，没有任何保护。

### 实现方案

#### 方案 1：修改 WindowControls.tsx

```typescript
import { useEditorStore } from '@/stores/editor-store'
import { ConfirmDialog } from '@/components/common/ConfirmDialog'

const handleClose = async () => {
  const { isDirty } = useEditorStore.getState()

  if (isDirty) {
    // 显示确认对话框
    setShowCloseConfirm(true)
  } else {
    appWindow.close()
  }
}

// 对话框选项
// [保存并退出] → 执行保存 → 关闭
// [不保存退出] → 直接关闭
// [取消] → 关闭对话框
```

#### 方案 2：全局 Tauri close-requested 事件（推荐，更可靠）

新建 `src/hooks/use-close-protection.ts`：

```typescript
/**
 * @author Beta
 * @description 窗口关闭保护 — 拦截关闭事件，检查未保存修改
 */
import { useEffect } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEditorStore } from '@/stores/editor-store'

export function useCloseProtection() {
  useEffect(() => {
    const appWindow = getCurrentWindow()

    const unlisten = appWindow.onCloseRequested(async (event) => {
      const { isDirty } = useEditorStore.getState()

      if (isDirty) {
        // 阻止默认关闭
        event.preventDefault()

        // 触发保存
        const manualSave = (window as any).__manualSave
        if (manualSave) {
          try {
            await manualSave()
          } catch (error) {
            console.error('Failed to save before close:', error)
          }
        }

        // 保存完成后关闭
        await appWindow.close()
      }
      // 如果没有未保存修改，让默认关闭行为执行
    })

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])
}
```

在 `App.tsx` 中使用：

```typescript
// === Beta: Close Protection START ===
import { useCloseProtection } from '@/hooks/use-close-protection'

function App() {
  useCloseProtection()
  // ... 其余不变
}
// === Beta: Close Protection END ===
```

#### 同时添加 beforeunload 兜底

在 `App.tsx` 中：

```typescript
useEffect(() => {
  const handleBeforeUnload = (e: BeforeUnloadEvent) => {
    const { isDirty } = useEditorStore.getState()
    if (isDirty) {
      e.preventDefault()
      e.returnValue = ''
    }
  }
  window.addEventListener('beforeunload', handleBeforeUnload)
  return () => window.removeEventListener('beforeunload', handleBeforeUnload)
}, [])
```

### Tauri 配置

确保 `src-tauri/tauri.conf.json` 中窗口有 `closeOnRequest` 事件支持：

```json
{
  "app": {
    "windows": [{
      "closable": true
    }]
  }
}
```

### 自测项

```
☐ 有未保存修改时点击关闭按钮 → 自动保存后关闭
☐ 无修改时点击关闭 → 直接关闭
☐ 关闭后重新打开 → 内容已保存
☐ Alt+F4 → 同样触发保护
```

### 交付文件

```
src/hooks/use-close-protection.ts          ← 新建
src/components/layout/WindowControls.tsx   ← 修改（可选，如需对话框）
src/App.tsx                                ← 修改（添加 hook 和 beforeunload）
```

---

## B-6：章节切换保护

### 实现要求

修改 `src/components/layout/LeftPanel.tsx` 的 `handleChapterSelect`：

```typescript
const handleChapterSelect = async (chapterPath: string, chapterId: string, title?: string) => {
  if (!projectPath) return

  const { isDirty } = useEditorStore.getState()

  if (isDirty) {
    // 方案 A：自动保存后切换（推荐，更流畅）
    try {
      const manualSave = (window as any).__manualSave
      if (manualSave) await manualSave()
    } catch (error) {
      console.error('Failed to auto-save before switch:', error)
    }

    // 方案 B：弹出确认对话框（更安全但打断流程）
    // const confirmed = await showConfirmDialog(...)
    // if (!confirmed) return
  }

  try {
    const chapter = await readChapter(projectPath, chapterPath)
    setCurrentChapter(chapterId, chapterPath, title || chapter.title)
    setContent(chapter.content)
    setIsDirty(false)
  } catch (error) {
    console.error('Failed to read chapter:', error)
  }
}
```

### 自测项

```
☐ 正在编辑章节A，有未保存修改 → 切换到章节B → A 的修改已自动保存
☐ 切换后再切换回 A → 内容完整
```

### 交付文件

```
src/components/layout/LeftPanel.tsx  ← 修改（handleChapterSelect 函数）
```

---

## B-7：启动恢复上次文件

### 实现要求

在 `editor-store.ts` 中添加持久化字段：

```typescript
// 添加到 EditorState
lastOpenedProjectPath: string | null
lastOpenedChapterPath: string | null
lastOpenedChapterId: string | null
lastOpenedChapterTitle: string | null
```

使用 Zustand persist 中间件持久化。

在 `EditorPage` 挂载时，检查并恢复：

```typescript
useEffect(() => {
  const { lastOpenedChapterPath, lastOpenedChapterId, lastOpenedChapterTitle } = useEditorStore.getState()
  if (projectPath && lastOpenedChapterPath && !currentChapterId) {
    // 尝试恢复上次打开的章节
    handleChapterSelect(lastOpenedChapterPath, lastOpenedChapterId, lastOpenedChapterTitle)
  }
}, [projectPath])
```

### 自测项

```
☐ 打开章节A → 关闭应用 → 重新打开 → 自动恢复章节A
☐ 格式完整保留
☐ 如果上次的文件已删除 → 不崩溃，显示空编辑器
```

### 交付文件

```
src/stores/editor-store.ts              ← 修改（添加 lastOpened 字段）
src/components/layout/LeftPanel.tsx     ← 修改（保存 lastOpened）
src/components/editor/EditorPage.tsx    ← 修改（启动恢复逻辑）
```

---

## NovelEditor.tsx 集成指南

Beta 在 NovelEditor.tsx 中的修改范围：

```tsx
// === Beta: API Mount START ===
import { EditorAPI } from '@/lib/editor-api'

useEffect(() => {
  if (editor) {
    const api = new EditorAPI(editor)
    ;(window as any).editor = { api }
    // 发射 editor-ready 事件
    eventBus.emit(EVENTS.EDITOR_READY)
  }
  return () => {
    delete (window as any).editor
    eventBus.emit(EVENTS.EDITOR_DESTROYED)
  }
}, [editor])
// === Beta: API Mount END ===
```

**注意**：不要修改 NovelEditor.tsx 中 Beta 标记区域外的任何代码。
