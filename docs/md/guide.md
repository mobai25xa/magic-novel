# Phase 1 并行开发契约

> 三位开发者的协作规范、文件归属、接口约定与集成策略。
> 生效日期：2026-02-10

---

## 一、角色定义

| 代号 | 角色 | 核心职责 | 详细任务 |
|------|------|----------|----------|
| **Alpha** | 编辑器核心引擎 | TipTap 扩展、编辑器底层行为 | UniqueID修复、粘贴净化、Slash Command、查找替换重构、编辑器快捷键 |
| **Beta** | API 与数据安全 | 对外 API 层、数据完整性 | Headless API、原子化操作、Markdown 导出、关闭保护、章节切换保护 |
| **Gamma** | 界面与质量保障 | UI 组件、视觉体验、测试 | 状态栏、全屏模式、排版优化、工具栏增强、性能测试、边界测试 |

---

## 二、文件归属（严格执行）

每位开发者只能修改自己负责的文件。如需修改他人文件，必须通过 **接口约定** 协商。

### Alpha 独占文件

```
src/components/editor/extensions/unique-id.ts       ← 重构
src/components/editor/extensions/paste-handler.ts    ← 新建
src/components/editor/extensions/slash-command.tsx   ← 新建
src/components/editor/extensions/keyboard-shortcuts.ts ← 新建
src/components/editor/FindReplacePanel.tsx           ← 重构
```

### Beta 独占文件

```
src/lib/editor-api.ts            ← 新建（Headless API 类）
src/lib/operations.ts            ← 新建（原子化操作模块）
src/lib/markdown-serializer.ts   ← 新建（JSON → Markdown 转换器）
src/hooks/use-close-protection.ts ← 新建
src/components/layout/WindowControls.tsx  ← 修改（添加关闭保护）
src/components/layout/LeftPanel.tsx       ← 修改（章节切换保护）
src-tauri/src/commands/chapter.rs         ← 修改（Markdown 双保存）
src-tauri/src/services/markdown.rs        ← 新建（Rust 端 Markdown 生成）
```

### Gamma 独占文件

```
src/components/editor/StatusBar.tsx      ← 新建
src/components/editor/FullscreenMode.tsx ← 新建
src/styles/editor.css                    ← 修改（排版优化）
src/components/editor/EditorToolbar.tsx  ← 修改（添加分割线和全屏按钮）
src/stores/layout-store.ts              ← 修改（添加全屏状态）
tests/                                   ← 新建整个目录
```

### 共享文件（修改需协调）

以下文件可能被多人需要修改，需通过约定的方式协调：

```
src/components/editor/NovelEditor.tsx  ← 集成点
  Alpha：注册新扩展（paste-handler, slash-command, keyboard-shortcuts）
  Beta：挂载 editor.api 到 window
  Gamma：添加 StatusBar 组件和全屏容器

src/components/editor/EditorPage.tsx   ← 集成点
  Beta：添加关闭保护 hook
  Gamma：添加全屏包裹层

src/App.tsx                            ← 集成点
  Beta：添加全局关闭保护
```

### 共享文件修改协议

1. **NovelEditor.tsx** 的修改通过"插槽"方式进行：
   - Alpha 在 `extensions` 数组中添加新扩展（不动其他代码）
   - Beta 在 `useEffect` 中添加 API 挂载（不动其他代码）
   - Gamma 在 JSX return 中添加 StatusBar 和全屏容器（不动其他代码）

2. 每人只在自己标记的区域内修改，用注释标记区域：
   ```tsx
   // === Alpha: Extensions Registration ===
   // === Beta: API Mount ===
   // === Gamma: Layout Wrapper ===
   ```

---

## 三、接口约定（API Contract）

### 3.1 Alpha 对外暴露

Alpha 必须导出以下内容供 Beta 和 Gamma 使用：

```typescript
// src/components/editor/extensions/unique-id.ts
// 导出 UniqueIdExtension（已有，修复后保持相同导出）

// src/components/editor/extensions/paste-handler.ts
export const PasteHandlerExtension: Extension
// 保证：粘贴后的节点只含 type/attrs/content/marks/text
// 保证：marks 只含 bold/italic/strike/highlight
// 保证：attrs 中无 style/class/color/font 相关属性

// src/components/editor/extensions/slash-command.tsx
export const SlashCommandExtension: Extension
// 保证：空行或行首 "/" 触发命令面板
// 保证：支持 /h1 /h2 /h3 /quote /divider /text

// src/components/editor/extensions/keyboard-shortcuts.ts
export const KeyboardShortcutsExtension: Extension
// 保证：Ctrl+1/2/3 → H1/H2/H3
// 保证：Ctrl+0 → 恢复正文
// 保证：Ctrl+H → 触发 find-replace-open 事件
// 保证：F11 → 触发 fullscreen-toggle 事件
```

**事件总线约定**（Alpha 发射，Beta/Gamma 监听）：

```typescript
// Alpha 通过 eventBus 发射以下自定义事件：
// （使用 src/lib/events.ts 的 eventBus）
EVENTS.FIND_REPLACE_OPEN    // Ctrl+H 触发
EVENTS.FULLSCREEN_TOGGLE    // F11 触发
```

### 3.2 Beta 对外暴露

Beta 必须导出以下内容供 Gamma 使用：

```typescript
// src/lib/editor-api.ts
export class EditorAPI {
  constructor(editor: Editor)

  // 读取操作
  getContent(format: 'json' | 'markdown' | 'text'): unknown | string
  getWordCount(): number
  getAllParagraphIds(): string[]
  getParagraphText(id: string): string | null
  getCursorPosition(): { paragraphId: string | null; offset: number }

  // 写入操作
  insertText(text: string): void
  replaceText(paragraphId: string, oldText: string, newText: string): boolean
  insertParagraphAfter(paragraphId: string, content: string): string  // 返回新 UUID
  deleteParagraph(paragraphId: string): boolean

  // 光标操作
  moveCursorToParagraph(paragraphId: string): void
  moveCursorToEnd(): void
}

// src/lib/operations.ts
// 独立函数，不依赖 React
export function operationSave(editor: Editor, projectPath: string, chapterPath: string): Promise<void>
export function operationFindText(editor: Editor, text: string): Array<{ paragraphId: string; offset: number }>
export function operationReplaceInParagraph(editor: Editor, id: string, old: string, replacement: string): boolean
export function operationGetMarkdown(editor: Editor): string
export function operationGetJSON(editor: Editor): object
export function operationGetWordCount(editor: Editor): number
```

**Gamma 可直接使用的接口**（用于状态栏）：

```typescript
// Beta 保证以下 store 字段始终可用：
// useEditorStore.getState().isDirty     → boolean
// useEditorStore.getState().isSaving    → boolean
// useEditorStore.getState().lastSavedAt → number | null

// Beta 在 window 上挂载：
// window.editor.api → EditorAPI 实例（editor 就绪后即可用）
```

### 3.3 Gamma 对外暴露

```typescript
// src/components/editor/StatusBar.tsx
export function StatusBar({ editor }: { editor: Editor | null }): JSX.Element
// 不对外暴露 API，纯展示组件

// src/stores/layout-store.ts
// Gamma 添加以下字段：
export interface LayoutState {
  // ... 现有字段 ...
  isFullscreen: boolean
  toggleFullscreen: () => void
}
```

---

## 四、分支策略

```
main
 ├── feat/alpha-editor-core     ← Alpha 的工作分支
 ├── feat/beta-api-safety       ← Beta 的工作分支
 └── feat/gamma-ui-quality      ← Gamma 的工作分支
```

### 规则

1. 每人在自己的分支上开发，不直接推送 main
2. 每完成一个任务单元，提交一次，commit message 格式：`[Alpha/Beta/Gamma] 任务描述`
3. 集成顺序：**Alpha 先合并 → Beta 再合并 → Gamma 最后合并**
   - 理由：Alpha 的扩展是 Beta API 的底层依赖，Beta 的 API 是 Gamma 状态栏的数据来源
4. 合并前必须通过所有自测项
5. 合并冲突由双方在线协商，以文件归属方为主

---

## 五、集成检查点

### 检查点 1：Alpha 完成后

```
验证项：
☐ UniqueIdExtension 修复 → 撤销删除段落后 UUID 不变
☐ 粘贴净化 → Word/网页内容粘贴后 JSON 无 style/class/color
☐ Slash Command → "/" 弹出命令面板，/h1 /h2 /h3 /quote /divider /text 可用
☐ 查找替换 → 位置跳转准确，所有匹配项有背景高亮
☐ 快捷键 → Ctrl+1/2/3/0/H、F11 事件正确发射
```

### 检查点 2：Beta 合并后

```
验证项：
☐ Console 执行 editor.api.getContent('json') → 返回纯净 JSON
☐ Console 执行 editor.api.getAllParagraphIds() → 返回 UUID 数组
☐ Console 执行 editor.api.insertText("test") → 编辑器实时更新
☐ API 操作后 Ctrl+Z 可撤销
☐ operations.getMarkdown() → 返回干净 Markdown
☐ 关闭窗口有未保存修改 → 弹出确认对话框
☐ 切换章节有未保存修改 → 弹出确认或自动保存
```

### 检查点 3：Gamma 合并后（最终集成验收）

```
验证项：
☐ 状态栏实时显示字数、段落数、行列号、保存状态
☐ 选中文字 → 状态栏显示"已选 X 字 / 共 Y 字"
☐ F11 进入全屏 → 工具栏/标题栏/状态栏隐藏
☐ 全屏下 Ctrl+S/Ctrl+F 仍可用
☐ 编辑区域居中，内容宽度 ~750px
☐ 字体 16px，行高 1.8，字色非纯黑
☐ 工具栏有分割线和全屏按钮
```

---

## 六、通信协议

### 日常沟通

- 每人开始新任务前，在群组中发消息：`[Alpha] 开始：XXX任务`
- 每人完成任务后，发消息：`[Alpha] 完成：XXX任务，可验证`
- 遇到阻塞立即通知：`[Alpha] 阻塞：需要 Beta 提供 XXX`

### 接口变更

- 如需变更已约定的接口（第三节），必须发起 **接口变更请求**：
  ```
  [ICR] 发起人：Alpha
  变更内容：EditorAPI.getContent 返回类型从 unknown 改为 JSONContent
  影响范围：Beta 的实现、Gamma 的消费
  理由：类型安全
  ```
- 所有相关方同意后方可变更
- 变更后更新本文档（guide.md）

---

## 七、编码规范

### 统一规范

1. 所有新文件使用 TypeScript（`.ts` / `.tsx`）
2. 不引入新的 npm 依赖前需在群组中讨论
   - 已批准的新依赖：`@tiptap/suggestion`（Alpha 用于 Slash Command）
   - 已批准的新依赖：`tiptap-markdown`（Beta 用于 Markdown 导出）——如果需要的话，也可自行实现序列化器
3. 组件使用函数组件 + hooks
4. 状态管理统一使用 Zustand
5. 事件通信使用 `src/lib/events.ts` 的 `eventBus`
6. 样式使用 Tailwind CSS，编辑器内部样式写在 `src/styles/editor.css`
7. 不使用 `any` 类型（除非确实无法避免，需注释原因）
8. 新建文件需在文件头注释作者和日期：
   ```typescript
   /**
    * @author Alpha
    * @date 2026-02-XX
    * @description 粘贴净化扩展
    */
   ```

### 测试要求

- Alpha：每个扩展附带至少 3 个手动测试用例（写在代码注释或单独 .md 中）
- Beta：每个 API 方法附带 Console 可执行的测试脚本
- Gamma：提供完整的手动测试清单 + 自动化性能测试脚本

---

## 八、交付标准

### 每人的任务完成标准

- 代码可正常编译（`pnpm build` 不报错）
- 新增代码无 TypeScript 错误
- 手动测试全部通过
- 对应 check01.md 的验收项标记为 ✅

### Phase 1 最终交付标准

回到 check01.md 的定义：

> "一个作家拿到它，在没有任何AI功能的情况下，愿意用它来写小说的前三章。"

三人合并后，由一位非开发者进行 30 分钟实际写作测试，无严重抱怨即为通过。

---

## 九、风险与应急

| 风险 | 影响 | 应急方案 |
|------|------|----------|
| Alpha 的扩展导致编辑器崩溃 | 全局 | Alpha 的所有扩展必须可独立禁用（通过 extensions 数组注释掉） |
| Beta 的 API 变更导致 Gamma 状态栏失效 | Gamma | Beta 提供 mock 数据函数供 Gamma 开发时使用 |
| 新依赖安装失败或版本冲突 | 全局 | 优先使用自行实现替代第三方库 |
| Rust 后端编译失败 | Beta | Beta 的 Markdown 保存可先在前端实现，后端作为增强 |

---

## 十、事件常量扩展

Alpha 需在 `src/lib/events.ts` 中添加以下常量（或通知 Beta 添加）：

```typescript
// 在 EVENTS 对象中添加：
FIND_REPLACE_OPEN: 'find-replace-open',
FIND_REPLACE_CLOSE: 'find-replace-close',
FULLSCREEN_TOGGLE: 'fullscreen-toggle',
EDITOR_READY: 'editor-ready',         // editor 实例就绪
EDITOR_DESTROYED: 'editor-destroyed', // editor 实例销毁
```

这些事件是三人协作的关键通信通道。

---

*本契约经三位开发者确认后生效。如有分歧，以本文档为准。*
