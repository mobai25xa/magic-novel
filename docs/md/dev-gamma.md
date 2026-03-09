# Developer Gamma 任务清单：界面与质量保障

> 负责人：Gamma
> 分支名：`feat/gamma-ui-quality`
> 核心文件：UI 组件、样式、测试

---

## 任务总览

```
编号    任务                         优先级   覆盖验收项                        预估复杂度
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
G-1    底部状态栏组件                P1      T-7.1~T-7.5                       ★★★★☆
G-2    编辑区域排版优化             P1      T-5.1.1~T-5.1.4                  ★★★☆☆
G-3    全屏/沉浸模式                P1      T-5.4.1~T-5.4.3                  ★★★★☆
G-4    工具栏增强                   P1      T-5.2.1（分割线+全屏按钮）       ★★☆☆☆
G-5    性能测试框架                 P0*     T-9.1~T-9.7                       ★★★★☆
G-6    边界情况测试清单             P0*     T-10.1~T-10.7                     ★★★☆☆
G-7    全量手动验收测试             P0*     check01.md 全部 103 项           ★★★★★
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

*P0* 标注：虽然是测试任务，但第 9/10 类必须全部通过才能交付。

**执行顺序：G-2 → G-1 → G-4 → G-3 → G-5 → G-6 → G-7**
（G-2 排版优化是最小改动量但影响最大的体验提升，优先完成）

---

## G-1：底部状态栏组件

### 问题分析

验收标准 T-7.1~T-7.5 要求编辑器底部有标准状态栏：

```
📄 3,542 字  │  ¶ 28 段  │  行 42, 列 15  │ 已保存
```

当前的 WritingStats 组件在左侧面板底部，是卡片式统计面板，不符合状态栏的紧凑格式。

### 实现要求

新建 `src/components/editor/StatusBar.tsx`：

```tsx
/**
 * @author Gamma
 * @date 2026-02-XX
 * @description 编辑器底部状态栏
 */
import { useEffect, useState, useCallback } from 'react'
import type { Editor } from '@tiptap/react'
import { useEditorStore } from '@/stores/editor-store'

interface StatusBarProps {
  editor: Editor | null
}

interface StatusInfo {
  wordCount: number
  paragraphCount: number
  line: number
  column: number
  selectionWordCount: number  // 选中字数，0表示无选区
}

export function StatusBar({ editor }: StatusBarProps) {
  const { isDirty, isSaving, lastSavedAt } = useEditorStore()
  const [status, setStatus] = useState<StatusInfo>({
    wordCount: 0,
    paragraphCount: 0,
    line: 1,
    column: 1,
    selectionWordCount: 0,
  })

  // 计算状态信息
  const updateStatus = useCallback(() => {
    if (!editor) return

    // 字数（不含空白）
    const text = editor.getText()
    const wordCount = text.replace(/\s/g, '').length

    // 段落数（只统计非空段落）
    let paragraphCount = 0
    editor.state.doc.descendants((node) => {
      if (['paragraph', 'heading'].includes(node.type.name) && node.textContent.trim()) {
        paragraphCount++
      }
    })

    // 光标位置（行号/列号）
    const { from, to } = editor.state.selection
    const { line, column } = calculateLineColumn(editor, from)

    // 选中字数
    let selectionWordCount = 0
    if (from !== to) {
      const selectedText = editor.state.doc.textBetween(from, to, ' ')
      selectionWordCount = selectedText.replace(/\s/g, '').length
    }

    setStatus({ wordCount, paragraphCount, line, column, selectionWordCount })
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
    if (isSaving) return '保存中...'
    if (isDirty) return '未保存'
    if (lastSavedAt) {
      return '已保存'
    }
    return '已保存'
  }

  // 保存状态图标颜色
  const getSaveStatusColor = (): string => {
    if (isSaving) return 'text-yellow-500'
    if (isDirty) return 'text-orange-500'
    return 'text-muted-foreground'
  }

  if (!editor) return null

  return (
    <div className="h-6 flex items-center px-3 border-t border-border bg-card text-xs text-muted-foreground select-none shrink-0">
      {/* 字数 */}
      <span className="flex items-center gap-1">
        {status.selectionWordCount > 0 ? (
          <span>已选 {status.selectionWordCount} 字 / 共 {status.wordCount} 字</span>
        ) : (
          <span>{status.wordCount.toLocaleString()} 字</span>
        )}
      </span>

      <Separator />

      {/* 段落数 */}
      <span>{status.paragraphCount} 段</span>

      <Separator />

      {/* 光标位置 */}
      <span>行 {status.line}, 列 {status.column}</span>

      {/* 右侧对齐 */}
      <div className="flex-1" />

      {/* 保存状态 */}
      <span className={getSaveStatusColor()}>
        {getSaveStatus()}
      </span>
    </div>
  )
}

// 分隔符组件
function Separator() {
  return <span className="mx-2 text-border">│</span>
}

/**
 * 计算光标所在的行号和列号
 */
function calculateLineColumn(editor: Editor, pos: number): { line: number; column: number } {
  let line = 1
  let column = 1
  let currentPos = 0

  editor.state.doc.descendants((node, nodePos) => {
    if (nodePos >= pos) return false  // 已经过了光标位置

    if (['paragraph', 'heading', 'blockquote'].includes(node.type.name)) {
      if (nodePos + node.nodeSize <= pos) {
        // 整个节点在光标之前 → 计为一行
        line++
      } else if (nodePos < pos) {
        // 光标在此节点内部
        // 计算节点内部的列偏移
        const textBefore = node.textBetween(0, pos - nodePos - 1, undefined, ' ')
        column = textBefore.length + 1

        // 检查节点内是否有软换行 (hardBreak)
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
```

### 在 NovelEditor.tsx 中集成

在 Gamma 标记区域内：

```tsx
// === Gamma: Layout Wrapper START ===
import { StatusBar } from './StatusBar'

// 在 return 的 JSX 中，EditorContent 下方添加：
return (
  <div className="flex h-full flex-col">
    <EditorToolbar ... />
    <FindReplacePanel ... />
    <div className="flex-1 relative">
      <div ref={scrollContainerRef} className="h-full overflow-auto p-4 editor-scroll-container">
        <EditorContent editor={editor} className="h-full" />
      </div>
      <ScrollButtons containerRef={scrollContainerRef} />
    </div>
    <StatusBar editor={editor} />  {/* Gamma 添加 */}
  </div>
)
// === Gamma: Layout Wrapper END ===
```

### 自测项

```
☐ 编辑器底部出现状态栏，一行高度
☐ 显示字数（实时更新）
☐ 显示段落数（只统计非空段落）
☐ 显示行号和列号（随光标移动实时更新）
☐ 显示保存状态："已保存" / "未保存" / "保存中..."
☐ 选中文字 → 显示"已选 X 字 / 共 Y 字"
☐ 取消选区 → 恢复为"X 字"
☐ 暗色主题下颜色适配
☐ 状态栏不过度占用垂直空间（24px 高度）
```

### 交付文件

```
src/components/editor/StatusBar.tsx  ← 新建
```

---

## G-2：编辑区域排版优化

### 问题分析

当前编辑器的排版问题：
- 编辑器 `flex-1` 全宽展开，没有居中的内容区
- 使用 Tailwind `prose prose-sm`，字号偏小
- 行高由 prose 默认控制，可能不适合中文长文
- 字色未优化（可能是纯黑）
- 没有中文友好的字体设置

### 实现要求

修改 `src/styles/editor.css`：

```css
/* ====== Gamma: 编辑区域排版优化 START ====== */

/* 编辑器内容区域居中 */
.editor-scroll-container {
  display: flex;
  justify-content: center;
}

.ProseMirror {
  outline: none;
  min-height: 100%;
  max-width: 750px;          /* 内容区宽度：650-800px */
  width: 100%;
  margin: 0 auto;
  padding: 2rem 1.5rem;      /* 内边距 */
}

/* 中文友好字体 */
.ProseMirror {
  font-family:
    "PingFang SC",           /* macOS */
    "Microsoft YaHei",       /* Windows */
    "Noto Sans SC",          /* Linux/通用 */
    "Hiragino Sans GB",
    "WenQuanYi Micro Hei",
    -apple-system,
    BlinkMacSystemFont,
    sans-serif;
  font-size: 16px;           /* 正文字号：16-18px */
  line-height: 1.8;          /* 行高：1.75-2.0 */
  color: #1a1a1a;            /* 非纯黑字色 */
}

/* 暗色主题字色 */
.dark .ProseMirror {
  color: #e0e0e0;            /* 非纯白 */
}

/* 段落间距 */
.ProseMirror p {
  margin: 0.75em 0;          /* 段落间距明显大于行间距 */
}

/* 首行缩进 */
.ProseMirror.first-line-indent p {
  text-indent: 2em;
}

/* 标题样式 */
.ProseMirror h1 {
  font-size: 1.75em;
  font-weight: 700;
  margin-top: 1.5em;
  margin-bottom: 0.75em;
  line-height: 1.3;
}

.ProseMirror h2 {
  font-size: 1.4em;
  font-weight: 600;
  margin-top: 1.25em;
  margin-bottom: 0.5em;
  line-height: 1.4;
}

.ProseMirror h3 {
  font-size: 1.15em;
  font-weight: 600;
  margin-top: 1em;
  margin-bottom: 0.5em;
  line-height: 1.4;
}

/* 引用块 */
.ProseMirror blockquote {
  border-left: 3px solid var(--border);
  padding-left: 1em;
  margin: 1em 0;
  color: var(--muted-foreground, #666);
  font-style: italic;
}

/* 分割线 */
.ProseMirror hr {
  border: none;
  border-top: 1px solid var(--border, #e0e0e0);
  margin: 2em auto;
  width: 60%;
}

/* 高亮标记 */
.ProseMirror mark {
  background-color: rgba(254, 240, 138, 0.7);
  padding: 0.1em 0;
}

/* 空段落确保可见 */
.ProseMirror p:empty::before {
  content: '\00a0';
}

/* ====== Gamma: 编辑区域排版优化 END ====== */
```

同时修改 `NovelEditor.tsx` 中 editorProps 的 class（移除 `prose-sm`）：

```typescript
editorProps: {
  attributes: {
    class: `novel-editor-content max-w-none focus:outline-none min-h-full${firstLineIndent ? ' first-line-indent' : ''}`,
    // 移除 prose prose-sm，改用 editor.css 中的自定义样式
  },
},
```

### 自测项

```
☐ 编辑区域居中，两侧有留白（非编辑区域显示背景色）
☐ 内容区宽度约 750px
☐ 字号 16px，行高 1.8
☐ 字色为 #1a1a1a（非纯黑）
☐ 暗色模式字色为 #e0e0e0（非纯白）
☐ 段落间距明显大于行间距
☐ H1 > H2 > H3 视觉层级清晰
☐ 引用块有左边框和缩进
☐ 分割线居中显示
☐ 持续写作 30 分钟主观不感到视觉疲劳
```

### 交付文件

```
src/styles/editor.css  ← 重构
```

---

## G-3：全屏/沉浸模式

### 实现要求

#### 1. 在 layout-store.ts 中添加全屏状态

```typescript
// 添加到 LayoutState
isFullscreen: boolean
toggleFullscreen: () => void
```

```typescript
// 在 create 中实现
isFullscreen: false,
toggleFullscreen: () => set((state) => ({ isFullscreen: !state.isFullscreen })),
```

#### 2. 新建全屏模式组件

新建 `src/components/editor/FullscreenMode.tsx`：

```tsx
/**
 * @author Gamma
 * @date 2026-02-XX
 * @description 全屏/沉浸写作模式
 */
import { useEffect } from 'react'
import { useLayoutStore } from '@/stores/layout-store'
import { eventBus, EVENTS } from '@/lib/events'

interface FullscreenModeProps {
  children: React.ReactNode
}

export function FullscreenMode({ children }: FullscreenModeProps) {
  const { isFullscreen, toggleFullscreen } = useLayoutStore()

  // 监听 F11 事件（由 Alpha 的快捷键扩展发射）
  useEffect(() => {
    const handleToggle = () => toggleFullscreen()
    eventBus.on(EVENTS.FULLSCREEN_TOGGLE, handleToggle)
    return () => eventBus.off(EVENTS.FULLSCREEN_TOGGLE, handleToggle)
  }, [toggleFullscreen])

  // 监听 Esc 退出全屏
  useEffect(() => {
    if (!isFullscreen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        toggleFullscreen()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isFullscreen, toggleFullscreen])

  if (!isFullscreen) {
    return <>{children}</>
  }

  return (
    <div className="fixed inset-0 z-50 bg-background flex flex-col">
      {/* 全屏模式：只显示编辑区域 */}
      {/* 鼠标移到顶部时短暂显示工具栏（可选增强） */}
      <div className="flex-1 overflow-hidden">
        {children}
      </div>
    </div>
  )
}
```

#### 3. 在 EditorPage.tsx 中集成

```tsx
// Gamma 在 EditorPage 中包裹编辑器
import { FullscreenMode } from '../editor/FullscreenMode'

// 在 EditorPanel 外层包裹
{/* Center Editor Panel */}
<FullscreenMode>
  <EditorPanel />
</FullscreenMode>
```

#### 4. 全屏模式下的 NovelEditor 行为

在 `NovelEditor.tsx` 中，全屏模式下隐藏工具栏：

```tsx
const { isFullscreen } = useLayoutStore()

return (
  <div className="flex h-full flex-col">
    {!isFullscreen && <EditorToolbar ... />}
    {!isFullscreen && <FindReplacePanel ... />}
    <div className="flex-1 relative">
      ...
    </div>
    {!isFullscreen && <StatusBar editor={editor} />}
  </div>
)
```

**注意**：全屏模式下快捷键仍有效（Ctrl+S、Ctrl+F、Ctrl+Z 等），因为它们在 editor 层面注册，不依赖工具栏可见性。

当 Ctrl+F 在全屏模式下触发时，FindReplacePanel 需要临时显示。实现方式：

```tsx
// 全屏模式下查找替换仍然可用
{(showFindReplace) && <FindReplacePanel ... />}
```

### 自测项

```
☐ F11 → 进入全屏：工具栏隐藏、标题栏隐藏、状态栏隐藏
☐ 全屏模式只剩编辑区域
☐ Esc → 退出全屏，恢复正常布局
☐ 全屏下 Ctrl+S → 保存成功
☐ 全屏下 Ctrl+F → 查找栏临时出现
☐ 全屏下 Ctrl+Z → 撤销正常工作
☐ 全屏下所有快捷键仍有效
☐ 暗色主题下全屏背景色正确
```

### 交付文件

```
src/components/editor/FullscreenMode.tsx  ← 新建
src/stores/layout-store.ts               ← 修改（添加 isFullscreen）
src/components/editor/EditorPage.tsx      ← 修改（添加 FullscreenMode 包裹）
```

---

## G-4：工具栏增强

### 实现要求

修改 `src/components/editor/EditorToolbar.tsx`，添加两个按钮：

#### 1. 分割线按钮

在"引用"按钮后面添加：

```tsx
<ToolbarButton
  onClick={() => editor.chain().focus().setHorizontalRule().run()}
  title="分割线"
>
  <Minus className="h-4 w-4" />
</ToolbarButton>
```

#### 2. 全屏按钮

在最后一个分隔符后面添加：

```tsx
import { Maximize, Minimize } from 'lucide-react'
import { useLayoutStore } from '@/stores/layout-store'

// 在组件内部
const { isFullscreen, toggleFullscreen } = useLayoutStore()

// 在 JSX 中
<div className="w-px h-6 bg-border mx-1" />

<ToolbarButton
  onClick={toggleFullscreen}
  title={isFullscreen ? "退出全屏 (Esc)" : "全屏 (F11)"}
>
  {isFullscreen ? (
    <Minimize className="h-4 w-4" />
  ) : (
    <Maximize className="h-4 w-4" />
  )}
</ToolbarButton>
```

#### 3. Tooltip 显示快捷键

更新现有按钮的 tooltip，补全快捷键信息：

```tsx
// 标题按钮
title="标题 1 (Ctrl+1)"
title="标题 2 (Ctrl+2)"
title="标题 3 (Ctrl+3)"

// 工具栏布局最终结果
// [撤销] [重做] │ [H1] [H2] [H3] [引用] [分割线] │ [B] [I] [S] [高亮] │ [查找] │ [全屏]
```

### 自测项

```
☐ 分割线按钮 → 点击插入水平线
☐ 全屏按钮 → 点击进入全屏
☐ 全屏模式下图标变为"退出全屏"
☐ 所有按钮 tooltip 显示名称 + 快捷键
☐ 光标在 H2 中 → H2 按钮高亮
☐ 工具栏单行排列，不超出屏幕
```

### 交付文件

```
src/components/editor/EditorToolbar.tsx  ← 修改
```

---

## G-5：性能测试框架

### 实现要求

新建 `tests/` 目录，创建性能测试脚本和文档。

#### 1. 测试数据生成器

新建 `tests/generate-test-data.ts`：

```typescript
/**
 * @author Gamma
 * @description 生成不同规模的测试文档
 * 使用方法：在 Console 中粘贴运行
 */

// 生成指定字数的 TipTap JSON 文档
function generateTestDoc(wordCount: number): object {
  const paragraphs = []
  const wordsPerParagraph = 100  // 每段约100字
  const paragraphCount = Math.ceil(wordCount / wordsPerParagraph)

  for (let i = 0; i < paragraphCount; i++) {
    const text = generateChineseText(wordsPerParagraph)
    paragraphs.push({
      type: 'paragraph',
      attrs: { id: crypto.randomUUID() },
      content: [{ type: 'text', text }],
    })
  }

  return {
    type: 'doc',
    content: paragraphs,
  }
}

function generateChineseText(count: number): string {
  const sample = '这是一段用于性能测试的中文文本内容。在这个美丽的故事中，主人公经历了许多精彩的冒险。'
  let result = ''
  while (result.length < count) {
    result += sample
  }
  return result.slice(0, count)
}

// 使用方式：
// const doc = generateTestDoc(50000)  // 5万字文档
// editor.api.getContent('json') 保存当前内容
// 然后 editor 实例的 setContent(doc) 加载测试文档
```

#### 2. 性能测试清单

新建 `tests/performance-tests.md`：

```markdown
# 性能测试清单

## T-9.1 输入延迟
- [ ] 测试方法：打开 DevTools > Performance > 录制
- [ ] 快速打字 30 秒，观察 Long Task 标记
- [ ] 主观感受：与记事本对比无明显差异
- [ ] 结果：____ms

## T-9.2 打开文件速度
- [ ] 1,000 字文档：console.time('open') → 加载 → console.timeEnd('open')
      结果：____ms（要求 < 100ms）
- [ ] 5,000 字文档：结果：____ms（要求 < 200ms）
- [ ] 20,000 字文档：结果：____ms（要求 < 500ms）
- [ ] 50,000 字文档：结果：____ms（要求 < 1000ms）

## T-9.3 保存速度
- [ ] 测试方法：在 use-auto-save.ts 的 performSave 中添加计时
      console.time('save') / console.timeEnd('save')
- [ ] 任意大小文档保存：结果：____ms（要求 < 200ms）

## T-9.4 滚动流畅度
- [ ] 测试方法：5万字文档 + DevTools > Rendering > Frame Rendering Stats
- [ ] 快速滚动时帧率：____fps（要求 ≥ 30fps）

## T-9.5 内存占用
- [ ] 测试方法：DevTools > Memory > Heap Snapshot
- [ ] 5,000 字文档：____MB（要求 < 200MB）
- [ ] 50,000 字文档：____MB（要求 < 300MB）
- [ ] 写作 1 小时后内存：____MB（要求不持续增长）

## T-9.6 应用启动时间
- [ ] 冷启动：____秒（要求 < 3秒）
- [ ] 热启动：____秒（要求 < 2秒）

## T-9.7 连续写作稳定性
- [ ] 连续写作 2 小时后输入仍流畅：☐ 是 / ☐ 否
- [ ] 内存无显著增长：☐ 是 / ☐ 否
- [ ] 未崩溃：☐ 是 / ☐ 否
```

#### 3. 自动化性能测试脚本

新建 `tests/perf-benchmark.js`（在 Console 中运行）：

```javascript
/**
 * 性能基准测试脚本
 * 在应用的 DevTools Console 中粘贴运行
 */
async function runBenchmarks() {
  const results = {}

  // T-9.2 打开速度
  const sizes = [1000, 5000, 20000, 50000]
  for (const size of sizes) {
    const doc = generateTestDoc(size)
    const start = performance.now()
    // 假设 editor 实例可用
    window.__tiptapEditor?.commands.setContent(doc)
    const end = performance.now()
    results[`open_${size}`] = `${(end - start).toFixed(1)}ms`
    await sleep(500)
  }

  // T-9.3 保存速度
  const saveStart = performance.now()
  await window.__manualSave?.()
  const saveEnd = performance.now()
  results['save'] = `${(saveEnd - saveStart).toFixed(1)}ms`

  // T-9.5 内存
  if (performance.memory) {
    results['memory_mb'] = `${(performance.memory.usedJSHeapSize / 1024 / 1024).toFixed(1)}MB`
  }

  console.table(results)
  return results
}

function sleep(ms) { return new Promise(r => setTimeout(r, ms)) }

function generateTestDoc(wordCount) {
  const paragraphs = []
  const sample = '这是一段用于性能测试的中文文本内容。在这个美丽的故事中主人公经历了许多精彩的冒险和奇遇。'
  const wordsPerParagraph = 100
  const count = Math.ceil(wordCount / wordsPerParagraph)
  for (let i = 0; i < count; i++) {
    let text = ''
    while (text.length < wordsPerParagraph) text += sample
    text = text.slice(0, wordsPerParagraph)
    paragraphs.push({
      type: 'paragraph',
      attrs: { id: crypto.randomUUID() },
      content: [{ type: 'text', text }],
    })
  }
  return { type: 'doc', content: paragraphs }
}

// 运行
runBenchmarks()
```

### 交付文件

```
tests/generate-test-data.ts    ← 新建
tests/performance-tests.md     ← 新建
tests/perf-benchmark.js        ← 新建
```

---

## G-6：边界情况测试清单

### 实现要求

新建 `tests/edge-case-tests.md`：

```markdown
# 边界情况测试清单

## T-10.1 空文档操作
- [ ] 空文档 Ctrl+A → 不崩溃
- [ ] 空文档 Ctrl+F → 显示"无结果"，不崩溃
- [ ] 空文档 Ctrl+S → 正常保存
- [ ] 空文档 getWordCount() → 返回 0
- [ ] 空文档 getAllParagraphIds() → 返回空数组或包含初始段落 ID

## T-10.2 极长段落
- [ ] 单段落 5000 字 → 编辑器不卡顿
- [ ] 在 5000 字段落中间插入文字 → 不卡顿
- [ ] 5000 字段落格式化（全选加粗） → 不卡顿

## T-10.3 极多段落
- [ ] 1000 个短段落（每段一句话） → 不卡顿
- [ ] 所有 1000 个段落 ID 唯一（Console 验证）
- [ ] 滚动不卡顿

## T-10.4 快速操作
- [ ] 快速连续按 Enter 100 次 → 不崩溃
- [ ] 快速连续 Ctrl+Z 50 次 → 不崩溃
- [ ] 快速连续 Ctrl+B 20 次 → 格式状态正确
- [ ] 快速连续 "/" 打开和关闭 Slash Command 10 次 → 不崩溃

## T-10.5 特殊字符
- [ ] Emoji 😀🎉📝 → 正确显示
- [ ] Emoji 保存后重新打开 → 仍正确显示
- [ ] 特殊标点 —— "" '' …… → 正确处理
- [ ] 中英文混排 → 正常显示和编辑
- [ ] 全角/半角标点 → 正常

## T-10.6 超大文件
- [ ] 10 万字文件 → 能打开（可以慢但不崩溃）
- [ ] 如果性能不佳 → 是否有友好提示？

## T-10.7 并发操作安全
- [ ] 自动保存执行中按 Ctrl+S → 不冲突
- [ ] 保存过程中继续打字 → 打字不被阻塞
- [ ] 自动保存和切换章节同时发生 → 不丢失数据
```

每项测试执行后，记录：
- 通过/不通过
- 如不通过，具体表现（崩溃/卡顿/数据错误）
- 截图或录屏（如有必要）

### 交付文件

```
tests/edge-case-tests.md  ← 新建
```

---

## G-7：全量手动验收测试

### 实现要求

**在 Alpha 和 Beta 都合并后执行。**

新建 `tests/full-acceptance-test.md`，逐项对照 `check01.md` 的 103 项标准进行测试。

格式：

```markdown
# Phase 1 全量验收测试报告

> 测试日期：2026-02-XX
> 测试人：Gamma
> 应用版本：X.X.X

## 1. 输入与编辑体验

| 编号 | 测试项 | 状态 | 备注 |
|------|--------|------|------|
| T-1.1.1 | 打开编辑器直接打字 | ✅/❌ | |
| T-1.1.2 | 连续打字2000字无卡顿 | ✅/❌ | |
| ... | ... | ... | ... |

## 2. 格式化能力
...

## 总结
- 通过项数：XX / 103
- 未通过项数：XX
- 阻断问题列表：
  1. ...
  2. ...
```

### 交付文件

```
tests/full-acceptance-test.md  ← 新建（三方合并后填写）
```

---

## NovelEditor.tsx 集成指南

Gamma 在 NovelEditor.tsx 中的修改范围：

```tsx
// === Gamma: Layout Wrapper START ===
import { StatusBar } from './StatusBar'
import { useLayoutStore } from '@/stores/layout-store'

// 在组件内部
const { isFullscreen } = useLayoutStore()

// 在 return JSX 中
return (
  <div className="flex h-full flex-col">
    {!isFullscreen && (
      <EditorToolbar editor={editor} onToggleFindReplace={() => setShowFindReplace(!showFindReplace)} />
    )}
    {showFindReplace && (
      <FindReplacePanel editor={editor} isOpen={showFindReplace} onClose={() => setShowFindReplace(false)} />
    )}
    <div className="flex-1 relative">
      <div ref={scrollContainerRef} className="h-full overflow-auto p-4 editor-scroll-container">
        <EditorContent editor={editor} className="h-full" />
      </div>
      <ScrollButtons containerRef={scrollContainerRef} />
    </div>
    {!isFullscreen && <StatusBar editor={editor} />}
  </div>
)
// === Gamma: Layout Wrapper END ===
```

**注意**：
- 不修改 Alpha 的 extensions 注册区域
- 不修改 Beta 的 API 挂载区域
- 只在 Gamma 标记区域内操作
- 对 `editorProps.attributes.class` 的修改需要和 Alpha 协调（Alpha 可能也改了首行缩进类名）
