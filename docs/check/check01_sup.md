# Phase 1 验收标准差距分析与补充清单

> 基于 check01.md 的 103 项验收标准，对 magic-novel 当前代码实现的逐项审查结果。
> 审查日期：2026-02-10

---

## 总体评估

```
分类                     测试项数    已实现    部分实现    未实现    通过率
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. 输入与编辑体验           17        10        3          4       ~59%
2. 格式化能力               14         8        1          5       ~57%
3. 查找与替换               10         7        1          2       ~70%
4. 文件管理                  9         4        2          3       ~44%
5. 界面与布局               12         5        2          5       ~42%
6. 快捷键体系                3         1        1          1       ~33%
7. 状态栏                    5         2        0          3       ~40%
8. 基础设施验证             19         6        3         10       ~32%
9. 性能指标                  7         0        0          7        0% (未测试)
10. 边界与稳定性             7         0        0          7        0% (未测试)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
合计                       103        43       13         47       ~42%
```

**结论：当前实现距离 Phase 1 验收标准有较大差距，尤其在以下必须全部通过的类别中：**
- **第8类（基础设施验证）**：仅约32%，缺少 Headless API 和原子化操作
- **第9类（性能指标）**：0%，完全没有性能测试/基准
- **第1类（输入体验）**：约59%，粘贴净化和部分 UUID 行为缺失

---

## 1. 输入与编辑体验 — 缺失项

### 已实现 ✅
- T-1.1.1 打开编辑器直接打字（TipTap 默认支持）
- T-1.1.2 连续打字流畅（TipTap StarterKit 基础保证）
- T-1.1.3 快速打字不丢字（TipTap 基础保证）
- T-1.1.5 回车键创建新段落 + UUID（UniqueIdExtension 实现）
- T-1.2.1~T-1.2.8 选择操作（TipTap/ProseMirror 原生支持）
- T-1.3.1 Ctrl+C/V 基础复制粘贴
- T-1.3.2 Ctrl+X 剪切
- T-1.4.1 Ctrl+Z 撤销
- T-1.4.2 Ctrl+Shift+Z 重做
- T-1.4.4 撤销/重做后光标位置正确

### 部分实现 ⚠️
- **T-1.1.4 中文输入法兼容** — TipTap 对中文有基本支持，但未做专项测试验证；输入法候选窗口位置可能有问题
- **T-1.1.6 Shift+回车创建软换行** — TipTap StarterKit 自带 hardBreak，但需验证不会产生新 UUID
- **T-1.4.3 撤销深度 ≥ 100 步** — TipTap 默认 history 插件有撤销栈，但未配置最小深度保证

### 未实现 ❌

#### ❌ T-1.1.7 空行行为验证
**问题**：未验证连续两次回车产生的空段落是否有正确的 UUID 和视觉间距。
**需要**：
- 确认空段落有 UUID
- 调整 CSS 确保段落间距明显（当前 `margin: 0.5em 0` 可能不够）

#### ❌ T-1.3.3 从外部粘贴纯文本 — 自动分段 + 新 UUID
**问题**：当前未实现粘贴拦截逻辑。从外部粘贴多段纯文本时：
- 未确保自动按换行符分段
- 未确保每段获得新 UUID（UniqueIdExtension 的 `parseHTML` 会给新的 `null` id 节点赋值，但粘贴流程未验证）
**需要**：添加 `handlePaste` 或 `transformPastedHTML` 钩子

#### ❌ T-1.3.4 从外部粘贴富文本 — 剥离所有外部样式
**问题**：**严重缺失**。当前未配置任何粘贴净化逻辑。从 Word/网页粘贴的内容可能带入 CSS、字体、颜色等信息，直接污染纯净 JSON。
**需要**：
- 实现 `transformPastedHTML` 或 `clipboardTextSerializer` 扩展
- 只保留 bold/italic 等白名单格式
- 剥离所有 style、class、color、font 等属性

#### ❌ T-1.3.5 粘贴后 JSON 纯净验证
**问题**：同上，依赖粘贴净化的实现。
**需要**：配合 T-1.3.4 解决

#### ❌ T-1.4.5 撤销删除段落 → UUID 恢复
**问题**：UniqueIdExtension 的 `parseHTML` 总是返回新 UUID（`parseHTML: () => uuidv4()`），撤销时可能导致恢复的段落获得新 UUID 而非原始 UUID。
**需要**：修改 UniqueIdExtension，让 `parseHTML` 优先读取已有的 `data-id` 属性

#### ❌ T-1.4.6 连续撤销多步中间状态正确
**问题**：依赖 T-1.4.5，UUID 恢复逻辑有缺陷会导致中间状态不正确

---

## 2. 格式化能力 — 缺失项

### 已实现 ✅
- T-2.1.1 粗体 Ctrl+B（StarterKit 内置）
- T-2.1.2 斜体 Ctrl+I（StarterKit 内置）
- T-2.1.3 标题 H1/H2/H3（工具栏实现，但缺少 Ctrl+1/2/3 快捷键）
- T-2.1.4 引用块（工具栏实现）
- T-2.1.6 删除线（工具栏实现）
- T-2.2.1~T-2.2.2 混合格式（TipTap 原生支持）
- T-2.2.4 连续切换格式（TipTap 原生支持）
- T-2.2.5 格式工具栏状态同步（`isActive` 检查已实现）

### 部分实现 ⚠️
- **T-2.1.3 标题快捷键** — 工具栏按钮有，但缺少 Ctrl+1/2/3 键盘快捷键

### 未实现 ❌

#### ❌ T-2.1.5 分割线（水平线）
**问题**：工具栏中没有分割线/水平线按钮。StarterKit 包含 `horizontalRule` 扩展但未暴露到 UI。
**需要**：
- 在 EditorToolbar 添加分割线按钮（`editor.chain().focus().setHorizontalRule().run()`）
- 验证 `---` 或 `***` 输入自动转换（StarterKit 的 inputRules 可能已支持）

#### ❌ T-2.2.3 标题段落中包含粗体/斜体验证
**需要**：验证测试

#### ❌ T-2.3.1~T-2.3.7 Slash Command 系统 — **完全缺失**
**问题**：**严重缺失**。没有任何 Slash Command 实现。搜索代码中无 `slash`、`SlashCommand` 相关代码。
**需要**：
- 安装 `@tiptap/suggestion` 或类似扩展
- 实现 "/" 触发的命令面板
- 支持命令列表：/h1, /h2, /h3, /quote, /divider, /text
- 支持过滤、上下键选择、回车确认、Esc 关闭
- 只在行首或空段落触发，段落中间不触发

---

## 3. 查找与替换 — 缺失项

### 已实现 ✅
- T-3.1 Ctrl+F 打开查找栏（已实现）
- T-3.2 搜索匹配高亮 + 计数显示（已实现 currentMatch/totalMatches）
- T-3.3 Enter/Shift+Enter 上下导航（已实现）
- T-3.4 替换 + 全部替换（已实现）
- T-3.5 大小写敏感开关（已实现 caseSensitive）
- T-3.7 Esc 关闭查找栏（已实现）
- T-3.8 / T-3.9 空文档/无结果不崩溃（已处理）

### 部分实现 ⚠️
- **T-3.2 匹配项高亮** — 当前只通过 `setTextSelection` 高亮当前匹配项，其他匹配项没有可见的高亮标记（如黄色背景）。验收要求"所有匹配项高亮标记"。

### 未实现 ❌

#### ❌ T-3.4 Ctrl+H 快捷键打开替换模式
**问题**：当前只有 Ctrl+F 打开查找面板，没有 Ctrl+H 直接打开替换模式的快捷键。
**需要**：在 NovelEditor 的 keydown 监听中添加 Ctrl+H 处理

#### ❌ T-3.6 正则表达式搜索
**问题**：当前使用 `indexOf` 做纯字符串搜索，不支持正则。
**需要**：添加正则开关，使用 `RegExp` 替代 `indexOf`

#### ❌ T-3.10 全部替换后可整体撤销
**问题**：当前 `handleReplaceAll` 在一个 transaction 中执行所有替换，理论上可以一次撤销，但需要验证。
**需要**：测试验证

#### ❌ T-3.2 搜索匹配位置计算 Bug
**问题**：FindReplacePanel 中使用 `doc.textContent` 获取纯文本，然后 `foundIndex + 1` 计算 ProseMirror 位置。但 ProseMirror 的位置体系中，节点之间有额外的位置偏移（如段落节点的开始/结束标记各占1个位置），纯文本 index 不等于 ProseMirror position。这会导致搜索跳转到错误位置。
**需要**：使用 ProseMirror 的 `doc.descendants` 遍历或 `TextSelection.findFrom` 来正确计算位置

---

## 4. 文件管理 — 缺失项

### 已实现 ✅
- T-4.1.1 Ctrl+S 手动保存（use-auto-save.ts 实现）
- T-4.1.2 自动保存（2秒 debounce，已实现）
- T-4.1.3 修改标记（isDirty 状态 + TopBar 显示"未保存"/"保存中..."）
- T-4.3.1 新建文件（通过 LeftPanel 的创建章节功能）

### 部分实现 ⚠️
- **T-4.1.4 保存产物验证** — 保存为 JSON 文件，但**没有同时保存 .md 文件**。验收要求同时存在 `.tiptap.json` 和 `.md` 文件。当前保存为 `{chapter_id}.json`（包含 Chapter 结构体，内嵌 content 字段）
- **T-4.2.1 启动恢复** — 无自动恢复上次打开的文件功能（EditorPanel 启动时显示"选择一个章节开始编辑"）

### 未实现 ❌

#### ❌ T-4.1.4 双格式保存（JSON + Markdown）
**问题**：**严重缺失**。当前只保存为 JSON。验收要求同时保存 `.tiptap.json`（编辑器格式）和 `.md`（Markdown 格式）。
**需要**：
- 在 save_chapter 后端命令中，额外生成并保存 Markdown 文件
- 或在前端保存时同时调用 `editor.storage.markdown` 获取 MD 并保存

#### ❌ T-4.2.2 菜单/按钮打开文件 + 文件选择对话框
**问题**：当前打开文件是通过左侧目录树选择章节，没有独立的"打开文件"对话框来选择 `.tiptap.json` 文件。
**需要**：这是单文件阶段的验收要求，当前的项目管理模式实际上已超越了这个要求，但缺少直接打开文件的能力。

#### ❌ T-4.2.3 打开时未保存提示
**问题**：切换章节时没有检查当前文件是否有未保存修改并弹出确认对话框。虽然有自动保存（2秒），但如果用户在2秒内切换章节，修改可能丢失。
**需要**：在 `handleChapterSelect` 中检查 `isDirty`，如果为 true 则弹出确认

#### ❌ T-4.3.2 新建时未保存修改提示
**问题**：同 T-4.2.3

#### ❌ T-4.3.3 新建文件首次保存弹出"另存为"
**问题**：当前新建章节时直接保存到项目目录，没有"另存为"功能

#### ❌ T-4.4.1 关闭窗口时未保存确认
**问题**：**严重缺失**。WindowControls.tsx 中 `handleClose` 直接调用 `appWindow.close()`，没有检查是否有未保存修改，也没有弹出确认对话框。
**需要**：
- 在关闭前检查 `isDirty` 状态
- 弹出"保存并退出 / 不保存退出 / 取消"确认对话框
- 或注册 Tauri 的 `close-requested` 事件拦截

#### ❌ T-4.4.2 系统强制关闭保护
**问题**：没有注册 `beforeunload` 或 Tauri 的窗口关闭事件来触发自动保存
**需要**：在应用中添加 `beforeunload` 事件处理

---

## 5. 界面与布局 — 缺失项

### 已实现 ✅
- T-5.2.1 工具栏包含必要按钮（粗体、斜体、删除线、H1/H2/H3、引用、撤销/重做）
- T-5.2.2 工具栏按钮有 tooltip（已实现，如"粗体 (Ctrl+B)"）
- T-5.2.3 工具栏状态同步（`isActive` 检测已实现）
- T-5.3.1~T-5.3.3 亮色/暗色主题 + 切换（useTheme hook + SettingsDialog 实现）
- T-5.3.4 跟随系统主题（theme='system' 模式已实现）

### 部分实现 ⚠️
- **T-5.1.1 编辑区域居中 + 两侧留白** — EditorPanel 是 `flex-1` 全宽展开，没有设置固定内容区宽度（650-800px）和两侧留白
- **T-5.1.2 行高舒适** — editor.css 中行高由 Tailwind `prose` 类控制，但未显式设置 1.75-2.0 的行高

### 未实现 ❌

#### ❌ T-5.1.1 编辑区域居中显示 + A4 纸面感
**问题**：编辑器占满中间面板全部宽度，没有居中的内容区（650-800px），也没有两侧灰色留白。
**需要**：
- 在 editor.css 或 EditorContent 外层添加 `max-width: 750px; margin: 0 auto;` 样式
- 两侧添加淡色背景

#### ❌ T-5.1.3 字体设置
**问题**：没有显式设置适合中文阅读的字体和字号。编辑器使用 Tailwind `prose prose-sm` 类，默认字号较小。验收要求正文 16-18px，字色 #333 或 #1a1a1a。
**需要**：
- 在 editor.css 中设置字体族（如"思源宋体"、"PingFang SC"等）
- 设置字号 16-18px
- 设置字色为非纯黑

#### ❌ T-5.2.1 工具栏缺少分割线按钮和全屏按钮
**问题**：工具栏缺少"分割线 ──"按钮和"全屏 ⛶"按钮

#### ❌ T-5.4.1~T-5.4.3 全屏/沉浸模式 — **完全缺失**
**问题**：**严重缺失**。没有任何全屏/沉浸模式的实现。
**需要**：
- 实现 F11 或 Ctrl+Shift+F 进入全屏模式
- 隐藏工具栏、标题栏、状态栏
- Esc 退出全屏
- 全屏模式下快捷键仍有效

---

## 6. 快捷键体系 — 缺失项

### 已实现 ✅
- 基础编辑快捷键：Ctrl+Z、Ctrl+Shift+Z、Ctrl+A、Ctrl+C/X/V（TipTap 内置）
- 格式快捷键：Ctrl+B、Ctrl+I（TipTap 内置）
- Ctrl+S 保存、Ctrl+F 查找（已实现）

### 部分实现 ⚠️
- **T-6.2 快捷键不与系统/Tauri 冲突** — 需要测试验证

### 未实现 ❌

#### ❌ 大量快捷键缺失
以下快捷键均未实现：
```
Ctrl+1       H1 标题
Ctrl+2       H2 标题
Ctrl+3       H3 标题
Ctrl+0       恢复为正文段落
Ctrl+H       替换
Ctrl+N       新建（可选）
Ctrl+O       打开（可选）
F11          全屏/退出全屏
Ctrl+加号    放大字号（可选）
Ctrl+减号    缩小字号（可选）
Esc          关闭查找栏（已实现但需验证焦点恢复）
```

**需要**：在 NovelEditor 或全局 keydown 处理中添加这些快捷键

---

## 7. 状态栏 — 缺失项

### 已实现 ✅
- T-7.2 字数统计（WritingStats 组件，实时更新）
- T-7.3 段落数统计（WritingStats 组件）

### 未实现 ❌

#### ❌ T-7.1 编辑器底部状态栏
**问题**：**严重缺失**。当前的 WritingStats 是放在左侧面板底部，不是编辑器底部的状态栏。验收要求的格式是：
```
📄 3,542 字  │  ¶ 28 段  │  行 42, 列 15  │ 已保存
```
这个标准状态栏完全不存在。
**需要**：
- 在编辑器底部添加独立的状态栏组件
- 显示字数、段落数、光标位置（行/列）、保存状态

#### ❌ T-7.2 选中文字时显示"已选 X 字 / 共 Y 字"
**问题**：当前 WritingStats 不会根据选区变化显示选中字数
**需要**：监听编辑器 `selectionUpdate` 事件，计算选区字数

#### ❌ T-7.4 光标位置（行号/列号）
**问题**：完全未实现
**需要**：
- 从 editor.state.selection 获取当前位置
- 计算行号和列号
- 实时更新显示

#### ❌ T-7.5 保存状态显示
**问题**：TopBar 有显示"未保存"/"保存中..."，但不在状态栏位置，且缺少"已保存"状态显示。
**需要**：移到底部状态栏统一显示

---

## 8. 基础设施验证 — 缺失项（★ 必须全部通过）

### 8.1 稳定 ID 验证

#### 已实现 ✅
- T-8.1.1 段落有唯一 UUID（UniqueIdExtension 实现）
- T-8.1.2 插入新段落时原段落 UUID 不变（appendTransaction 只给 null id 赋值）
- T-8.1.3 删除段落时其他段落 UUID 不变

#### 部分实现 ⚠️
- **T-8.1.9 保存→关闭→重新打开后 UUID 一致** — JSON 保存了 id 属性，但 `parseHTML: () => uuidv4()` 在从 HTML 加载时总是生成新 ID。如果通过 JSON (`setContent`) 加载则应该保留 ID，需要验证。

#### 未实现 ❌

##### ❌ T-8.1.4 剪切→粘贴保留原 UUID
**问题**：UniqueIdExtension 的 `parseHTML` 始终返回 `uuidv4()`，粘贴时无论是剪切还是复制都会生成新 UUID。验收要求剪切粘贴保留原 UUID。
**需要**：区分剪切和复制的粘贴行为，或修改粘贴逻辑

##### ❌ T-8.1.5 复制→粘贴产生新 UUID
**问题**：由于 `parseHTML` 总是生成新 UUID，复制粘贴确实会产生新 UUID，但这是巧合而非设计。需要明确验证。

##### ❌ T-8.1.6 撤销删除后 UUID 恢复
**问题**：**关键缺陷**。UniqueIdExtension 的 appendTransaction 会检查所有无 id 的节点并赋新 UUID，但 TipTap 的 undo 操作会恢复节点的 attrs（包括 id），不经过 `parseHTML`。理论上撤销应该保留原 UUID，但 appendTransaction 可能会在 undo 后再次触发并覆盖。需要验证。
**需要**：测试并确保 appendTransaction 不会覆盖 undo 恢复的 UUID

##### ❌ T-8.1.7 跨段落删除后合并段落的 UUID
**问题**：未测试。选中跨3段文字删除后，合并的段落 UUID 应为第一个段落的 UUID。
**需要**：测试验证

##### ❌ T-8.1.8 从外部粘贴文字的 UUID
**问题**：依赖粘贴净化实现

##### ❌ T-8.1.10 拆分段落的 UUID 行为
**问题**：在段落中间按回车，前半段应保留原 UUID，后半段获得新 UUID。当前 appendTransaction 会给无 id 的新节点赋 UUID，但需要验证前半段是否保留原 UUID。
**需要**：测试验证

### 8.2 Headless API — **完全缺失** ❌

**问题**：**这是最严重的缺失之一**。验收标准要求一整套可在 Console 中调用的 API：

```javascript
// 以下 API 全部不存在：
editor.api.getContent('json')
editor.api.getContent('markdown')
editor.api.getContent('text')
editor.api.getWordCount()
editor.api.getAllParagraphIds()
editor.api.getParagraphText('某ID')
editor.api.getCursorPosition()
editor.api.insertText("测试文字")
editor.api.replaceText("某段落ID", "旧文字", "新文字")
editor.api.insertParagraphAfter("某段落ID", "新段落内容")
editor.api.deleteParagraph("某段落ID")
editor.api.moveCursorToParagraph("某段落ID")
editor.api.moveCursorToEnd()
```

当前只有 `window.__manualSave` 暴露到全局。

**需要**：
- 创建 `EditorAPI` 类，封装所有读取/写入/光标操作
- 将实例挂载到 `window.editor = { api: new EditorAPI(editor) }`
- 确保 API 操作和手动操作共享同一个 undo 栈
- 通过 Console 操作后 Ctrl+Z 可撤销

### 8.3 纯净 JSON 验证

#### 部分实现 ⚠️
- 正常打字产生的 JSON 应该是纯净的（TipTap StarterKit 不会引入 CSS）

#### 未实现 ❌

##### ❌ T-8.3.2 / T-8.3.3 从外部粘贴后 JSON 纯净
**问题**：没有粘贴净化，外部粘贴内容可能污染 JSON
**需要**：实现粘贴净化（同 T-1.3.4）

##### ❌ T-8.3.4 JSON 结构合规性
**问题**：UniqueIdExtension 的 `parseHTML` 总是调用 `uuidv4()`，即使传入的 HTML 有 `data-id`，也会忽略。这意味着从 HTML 解析时 id 总是新的。
**需要**：修改 `parseHTML` 实现

##### ❌ T-8.3.5 JSON → Markdown 导出检查
**问题**：没有 `getContent('markdown')` API。TipTap 需要安装 `@tiptap/extension-markdown` 或手动实现 JSON→Markdown 转换。
**需要**：安装 markdown 扩展或实现转换器

### 8.4 原子化操作 — **完全缺失** ❌

**问题**：验收要求以下操作是独立函数，可在 Console 调用且不依赖 React 状态：

```javascript
operations.save()
operations.findText("张伟")
operations.replaceInParagraph(id, old, new)
operations.getMarkdown()
operations.getJSON()
operations.getWordCount()
```

当前所有操作都嵌在 React 组件或 hooks 中（如 `useAutoSave`、`FindReplacePanel`），不是独立函数。

**需要**：
- 创建独立的 `operations` 模块，不依赖 React 状态
- 所有操作函数接收 editor 实例作为参数
- 确保三种调用方式（UI按钮、Console、测试）结果一致

---

## 9. 性能指标 — **完全未测试** ❌

以下所有项目需要进行实际测试和基准测量：

| 测试项 | 状态 | 说明 |
|--------|------|------|
| T-9.1 输入延迟 < 16ms | ❓ | 需要高帧率录屏测试 |
| T-9.2 打开文件速度 | ❓ | 需要分级测试（1K/5K/20K/50K字） |
| T-9.3 保存速度 < 200ms | ❓ | 需要计时测试 |
| T-9.4 滚动流畅度 ≥ 30fps | ❓ | 需要大文档滚动测试 |
| T-9.5 内存占用 | ❓ | 需要 DevTools 监控 |
| T-9.6 应用启动时间 < 3秒 | ❓ | 需要计时测试 |
| T-9.7 连续写作2小时稳定性 | ❓ | 需要长时间测试 |

**需要**：
- 建立性能测试脚本或手动测试流程
- 记录各项基准数据
- 对不达标项进行优化

---

## 10. 边界情况与稳定性 — **完全未测试** ❌

以下所有项目需要进行实际测试：

| 测试项 | 状态 | 说明 |
|--------|------|------|
| T-10.1 空文档操作 | ❓ | 需要测试各操作不崩溃 |
| T-10.2 极长段落（5000字） | ❓ | 需要性能测试 |
| T-10.3 极多段落（1000段） | ❓ | 需要性能测试 + UUID唯一性验证 |
| T-10.4 快速操作 | ❓ | 需要压力测试 |
| T-10.5 特殊字符（Emoji等） | ❓ | 需要测试各种字符的显示/保存/恢复 |
| T-10.6 超大文件（10万字） | ❓ | 需要测试不崩溃 |
| T-10.7 并发操作安全 | ❓ | 需要测试自动保存与手动保存不冲突 |

---

## 优先级排序：需要补充的功能

### P0 — 阻断验收（必须全部通过的类别中的缺失）

1. **粘贴净化机制** — 影响 T-1.3.4, T-1.3.5, T-8.3.2, T-8.3.3
   - 实现 `transformPastedHTML` 扩展
   - 白名单过滤：只保留 bold/italic/strike
   - 剥离所有 style/class/color/font 属性

2. **Headless API（editor.api）** — 影响 T-8.2.1~T-8.2.3 全部19项中的核心
   - 创建 EditorAPI 类
   - 实现所有读取/写入/光标操作
   - 挂载到 window.editor.api

3. **原子化操作模块** — 影响 T-8.4.1~T-8.4.3
   - 创建独立的 operations 模块
   - 不依赖 React 状态

4. **UniqueIdExtension 修复** — 影响 T-8.1.4~T-8.1.10
   - `parseHTML` 应优先读取 `data-id` 而非总是生成新 UUID
   - 验证各种操作场景下的 UUID 行为

5. **关闭窗口保护** — 影响 T-4.4.1, T-4.4.2
   - 拦截窗口关闭事件
   - 检查 isDirty 并弹出确认

6. **Find/Replace 位置计算修复** — 影响 T-3.2
   - 使用 ProseMirror 位置体系替代纯文本 index

### P1 — 重要缺失

7. **底部状态栏** — 影响 T-7.1, T-7.4, T-7.5
   - 字数/段落数/光标位置/保存状态

8. **全屏/沉浸模式** — 影响 T-5.4.1~T-5.4.3
   - F11 切换全屏
   - 隐藏 UI 元素

9. **Slash Command 系统** — 影响 T-2.3.1~T-2.3.7
   - "/" 触发命令面板

10. **快捷键补全** — 影响 T-6.1
    - Ctrl+1/2/3/0 标题切换
    - Ctrl+H 替换
    - F11 全屏

11. **编辑区域居中 + 排版优化** — 影响 T-5.1.1~T-5.1.3
    - 内容区 650-800px
    - 行高 1.75-2.0
    - 中文友好字体

12. **双格式保存（JSON + Markdown）** — 影响 T-4.1.4

### P2 — 需要验证/测试

13. **切换章节时未保存提示** — T-4.2.3, T-4.3.2
14. **选中字数显示** — T-7.2
15. **分割线按钮** — T-2.1.5
16. **匹配项全部高亮**（非仅当前项）— T-3.2
17. **Ctrl+H 直接打开替换** — T-3.4
18. **正则搜索** — T-3.6（可选但推荐）
19. **性能测试** — 第9类全部
20. **边界测试** — 第10类全部

---

## 进入 Phase 2 的前置条件检查

```
☐ 编辑器的数据层接口稳定          → ❌ JSON schema 未正式定义，无 .tiptap.json 格式
☐ Headless API 函数签名稳定        → ❌ API 完全不存在
☐ 稳定 ID 机制全场景验证通过       → ❌ 多个场景未验证/有已知缺陷
☐ 纯净 JSON 全输入来源验证通过     → ❌ 粘贴净化未实现
☐ 性能在 5 万字级别可接受          → ❌ 未测试
```

**结论：当前距离 Phase 2 的地基要求仍有较大差距，需优先解决 P0 项目。**
