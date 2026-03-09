# Phase 1（Foundation）需求与技术分析（analysis01）

> 目标：构建一个**本地优先**、并且"架构为 AI 准备"的小说写作软件内核。
> 
> 本文档是 Phase 1 的**可执行规范**（contract）：目录结构、数据结构、导入导出、版本迁移、以及前后端接口都以此为准。

---

## 1. 项目概览

### 1.1 产品形态（UI 信息架构）

- **顶部栏**：包含设置按钮（至少：作品库路径、偏好设置）。
- **三栏布局**：
  - **左栏**：作品目录树（卷/章）
    - 目录树来自 `content/` 真实文件系统：**卷 = 文件夹**、**章 = JSON 文件**。
    - 排序规则：
      - 同一层级内均按文件名排序（ASCII/Unicode 字符序；建议用 `001_` 前缀确保顺序稳定）。
      - **展示顺序**：同一目录下，先显示"卷（文件夹）"，再显示"无卷章节（直接位于 `content/` 根目录或某目录下的章节文件）"。
    - **左下信息面板**：展示当前选中章节（chapter JSON）的元信息（字数、目标字数、进度/状态等）。
  - **中栏**：正文编辑器（Tiptap/ProseMirror），打开一个章节进行编辑。
  - **右栏**：AI 写小说助手（Phase 1 可以先搭好数据与接口、UI 占位；但必须支持后续以 block-level patch 写入/回放）。

### 1.2 Phase 1 范围

Phase 1 必须完成：
- 作品创建/打开（用户选择作品库路径，之后在该路径下创建多个作品）。
- 作品文件系统规范（`magic_novel/` + `content/`）。
- 章节编辑与持久化（chapter JSON，原子写入）。
- Block Identity（稳定 block id，复制粘贴防冲突）。
- 导入（txt/md/docx）到：
  - `magic_novel/`（提示词/设定/资料资产 → JSON 资产树）
  - `content/`（正文初稿 → 章/卷 → chapter JSON）
- 导出（txt/md/docx）支持：
  - 整本书单文件
  - 按卷/章多文件（保持目录层级）
- Schema 版本控制与迁移（整数递增、链式迁移、备份）。
- AI 变更记录底座（proposal + history + block-level patch）。

### 1.3 目录树交互规范（Phase 1）

```ts
interface TreeOperations {
  createVolume: (parentPath: string, title: string) => Promise<string>;
  createChapter: (volumePath: string, title: string) => Promise<string>;
  renameNode: (path: string, newName: string) => Promise<void>;
  deleteNode: (path: string) => Promise<void>;
}
```

> AI 友好考量：AI Agent 可能需要 `createChapter` 来自动创建新章节，未来可扩展 `batchCreateChapters`（大纲生成时批量创建）。

---

## 2. 技术栈（Tech Stack）

- Core: **Tauri v2 (Rust)**
- Frontend: **React 18+**, **TypeScript**
- Build Tool: **Vite**
- UI Framework: **Tailwind CSS + shadcn/ui**
- State Management: **Zustand**
- Editor Engine: **Tiptap (ProseMirror)**
- Data Sync: **tauri-specta**（Rust/TS 类型同步）
- Schema Validation: **Zod**（前端/导入时校验）

### 2.1 UUID 生成策略

| 场景 | 生成位置 | 理由 |
|------|----------|------|
| `project_id`, `volume_id`, `chapter_id` | **Rust 后端** | 保证唯一性，创建时生成 |
| `block.attrs.id`（人工输入） | **前端 UniqueIdExtension** | 实时插入，后端无感知 |
| `block.attrs.id`（AI 生成） | **Rust 后端** | AI proposal 中预分配，便于 patch 引用 |
| `proposal_id`, `event_id` | **Rust 后端** | 后端创建 |

建议库：
- Rust: `uuid` crate（v4）
- TS: `uuid` 或 `nanoid`（nanoid 更短，21 字符）

---

## 3. 文件系统规范（File System Contract）

### 3.1 作品库路径（Library Root）

- 应用有一个全局配置：`library_root`（用户选择的作品库根路径）。
- 创建作品时：在 `library_root` 下创建 `/<ProjectFolderName>`。
- 打开作品时：用户选择某个作品根目录（通常位于 `library_root` 下）。

> 兼容建议：允许打开不在 `library_root` 下的作品（"临时打开/导入"），但 Phase 1 可先不做 UI，只保留后端能力。

### 3.2 作品目录结构（强约束）

```text
/MyNovelProject
├── magic_novel/              # 元数据中枢（必须）
│   ├── project.json          # 项目级元数据（必须）
│   ├── lore/                 # 角色/世界观/规则/资料（JSON 资产）
│   ├── prompts/              # 提示词资产（JSON 资产）
│   ├── ai/
│   │   └── proposals/        # AI 生成的候选稿（proposal）
│   ├── history/
│   │   └── chapters/         # 章节变更历史（jsonl）
│   └── backups/              # 迁移备份
└── content/                  # 正文（必须）
    ├── 卷一/
    │   ├── _volume.json      # 卷元数据（必须）
    │   ├── 001_开端.json     # 章（JSON）
    │   └── 002_冲突.json
    └── 卷二/
        ├── _volume.json
        └── 001_转折.json
```

### 3.3 路径与引用约定

- 前后端协议中，涉及作品内文件路径时，统一使用**相对作品根目录的相对路径**。
  - 例：`content/卷一/001_开端.json`
- 任何跨文件引用（未来可能出现）优先使用稳定 `id`（chapter_id / volume_id / lore_item_id），避免依赖可变文件名。

---

## 4. 数据结构规范（Data Schema, JSON）

> 原则：所有 JSON 都必须可被 Rust struct（serde）与 TS type（specta）描述，并可被 Zod 校验。

### 4.1 Schema Version（数据结构版本）

- `schema_version` 使用**整数递增**：1, 2, 3 ...
- 版本字段存在于：
  - `magic_novel/project.json`（必须）
  - `content/**/_volume.json`（必须）
  - `content/**/*.json`（chapter，必须）
  - `magic_novel/lore/*.json`、`magic_novel/prompts/*.json`（建议包含）

若缺失 `schema_version`：视为 `1`。

### 4.2 项目元数据：`magic_novel/project.json`

最小字段（Phase 1 必须支持）：

```ts
interface ProjectMetadata {
  schema_version: number;

  project_id: string;      // UUID
  name: string;
  author: string;
  description?: string;

  created_at: number;      // unix ms 或 s：需统一（建议 ms）
  updated_at: number;

  app_min_version?: string; // 可选：防止旧软件打开新数据
  last_opened_at?: number;
}
```

### 4.3 卷元数据：`content/**/_volume.json`（必须）

```ts
interface VolumeMetadata {
  schema_version: number;

  volume_id: string;  // UUID
  title: string;
  summary?: string;

  created_at: number;
  updated_at: number;
}
```

> 卷的排序：由卷文件夹名排序决定（例如 `01_卷一`）。不额外提供 `order` 字段。

### 4.4 章节文件：`content/**/*.json`（Chapter）

#### 4.4.1 Chapter 结构

```ts
type ChapterStatus = "draft" | "revised" | "final";

interface Chapter {
  schema_version: number;

  id: string;           // UUID（稳定）
  title: string;

  // 正文（Tiptap JSON Root）
  content: TiptapDoc;   // 见 5.3 节完整定义

  // 章内信息面板字段
  counts: {
    text_length_no_whitespace: number;
    word_count?: number;
    algorithm_version: number;
    last_calculated_at: number;
  };

  target_words?: number;
  status?: ChapterStatus;
  summary?: string;
  tags?: string[];

  // 光标位置（可选，用于恢复上次编辑位置）
  last_cursor_position?: number;

  created_at: number;
  updated_at: number;
}
```

#### 4.4.2 字数统计（Phase 1 口径，已确认）

- 字数统计以 **可见纯文本** 为基础（不含 markup）。
- Phase 1 主指标：`counts.text_length_no_whitespace`
  - 将正文转换为纯文本 `plainText`
  - 规范化换行（例如 `\r\n` → `\n`）
  - 移除所有 Unicode 空白字符（空格/换行/制表等）
  - 统计剩余字符数
- 可选指标：`counts.word_count`
  - 统计纯文本中的英文/数字 token 数（用于英文场景）；Phase 1 可先不在 UI 展示。
- 每次保存或导入时更新 `counts.last_calculated_at`。
- 若未来调整统计口径，递增 `counts.algorithm_version`，避免旧章节统计结果"悄悄变化"。

### 4.5 Lore/Prompts 资产（导入后存为 JSON 条目树）

你已确认：导入 `md/docx/txt` 后，解析 **md 标题层级 / Word heading** 为条目树。

#### 4.5.1 通用条目树结构（建议统一 lore 与 prompts）

```ts
type AssetKind = "lore" | "prompt";

interface AssetTree {
  schema_version: number;
  id: string;                 // UUID
  kind: AssetKind;
  title: string;
  source?: {
    original_filename?: string;
    imported_at: number;
    importer: "txt" | "md" | "docx";
  };
  root: AssetNode;
}

interface AssetNode {
  node_id: string;           // UUID
  title: string;
  level: number;             // heading level（1..N），txt 可用 0
  content: string;           // 该节点正文（纯文本，去除格式）
  children: AssetNode[];
  tags?: string[];
}
```

- 存储建议：
  - `magic_novel/lore/<asset_id>.json`
  - `magic_novel/prompts/<asset_id>.json`

> 说明：Phase 1 先把内容存为纯文本 `content`，便于直接作为 LLM 上下文拼接。后续可扩展为富文本/引用。

---

## 5. 编辑器规范（Tiptap / ProseMirror Contract）

### 5.1 Tiptap 扩展配置清单（Phase 1）

```ts
import { Editor } from '@tiptap/react';
import Document from '@tiptap/extension-document';
import Text from '@tiptap/extension-text';
import Paragraph from '@tiptap/extension-paragraph';
import Heading from '@tiptap/extension-heading';
import Blockquote from '@tiptap/extension-blockquote';
import HardBreak from '@tiptap/extension-hard-break';
import Bold from '@tiptap/extension-bold';
import Italic from '@tiptap/extension-italic';
import Strike from '@tiptap/extension-strike';
import Highlight from '@tiptap/extension-highlight';
import History from '@tiptap/extension-history';
import { UniqueIdExtension } from './extensions/unique-id';

const extensions = [
  // Nodes
  Document,
  Text,
  Paragraph.configure({ HTMLAttributes: { class: 'novel-paragraph' } }),
  Heading.configure({ levels: [1, 2, 3] }),
  Blockquote,
  HardBreak,
  
  // Marks
  Bold,
  Italic,
  Strike,
  Highlight.configure({ multicolor: true }), // AI diff 用不同颜色：新增(绿)/修改(黄)/删除(红)
  
  // 核心：Block Identity
  UniqueIdExtension,
  
  // 历史（撤销/重做）
  History,
];
```

> AI 友好考量：`Highlight` 支持多色，便于区分 AI 生成内容的状态。后续可加 `AiGeneratedMark` 标记 AI 生成的文本。

### 5.2 Block Identity（稳定 block id）

必须实现自定义 `UniqueIdExtension`：
- 对指定 block nodes 自动注入 `attrs.id`（UUID）。
- Phase 1 最小覆盖节点：
  - `paragraph`
  - `heading`
  - `blockquote`
- 建议扩展（后续）：`listItem`、`codeBlock`、`tableRow` 等。

复制粘贴规则（验收点）：
- 从同一文档或外部粘贴进来，如果粘贴片段中带有 `attrs.id`：必须**重写为新 UUID**。
- 保存/重新打开后：`attrs.id` 必须保持不变。

DOM 映射约定：
- 存储层使用 `attrs.id`。
- 渲染到 DOM 可使用 `data-id`（实现层细节），但以 `attrs.id` 为真。

#### 5.2.1 UniqueIdExtension 实现参考

```ts
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { v4 as uuidv4 } from 'uuid';

const BLOCK_TYPES = ['paragraph', 'heading', 'blockquote'];

export const UniqueIdExtension = Extension.create({
  name: 'uniqueId',

  addGlobalAttributes() {
    return BLOCK_TYPES.map(type => ({
      types: [type],
      attributes: {
        id: {
          default: null,
          parseHTML: () => uuidv4(), // 粘贴时总是生成新 ID（防冲突）
          renderHTML: (attrs) => ({ 'data-id': attrs.id }),
        },
      },
    }));
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('uniqueId'),
        appendTransaction: (transactions, oldState, newState) => {
          // 为缺少 id 的 block 补充 UUID
          let tr = newState.tr;
          let modified = false;
          newState.doc.descendants((node, pos) => {
            if (BLOCK_TYPES.includes(node.type.name) && !node.attrs.id) {
              tr.setNodeMarkup(pos, undefined, { ...node.attrs, id: uuidv4() });
              modified = true;
            }
          });
          return modified ? tr : null;
        },
      }),
    ];
  },
});
```

> AI 友好考量：AI 生成的 Tiptap JSON 可以不带 id，插入后自动补充；或由后端预分配 UUID（推荐，便于 proposal 中提前引用）。

### 5.3 Tiptap JSON 结构规范

`Chapter.content` 字段的完整类型定义：

```ts
interface TiptapDoc {
  type: 'doc';
  content: TiptapBlock[];
}

type TiptapBlock =
  | {
      type: 'paragraph';
      attrs: { id: string };
      content?: TiptapInline[];
    }
  | {
      type: 'heading';
      attrs: { id: string; level: 1 | 2 | 3 };
      content?: TiptapInline[];
    }
  | {
      type: 'blockquote';
      attrs: { id: string };
      content?: TiptapBlock[]; // blockquote 可嵌套其他 block
    };

type TiptapInline =
  | { type: 'text'; text: string; marks?: TiptapMark[] }
  | { type: 'hardBreak' };

type TiptapMark =
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'strike' }
  | { type: 'highlight'; attrs?: { color: string } };
```

示例：

```json
{
  "type": "doc",
  "content": [
    {
      "type": "heading",
      "attrs": { "id": "b1a2c3d4-...", "level": 2 },
      "content": [{ "type": "text", "text": "第一章 起源" }]
    },
    {
      "type": "paragraph",
      "attrs": { "id": "e5f6g7h8-..." },
      "content": [
        { "type": "text", "text": "这是一个" },
        { "type": "text", "text": "重要的", "marks": [{ "type": "bold" }] },
        { "type": "text", "text": "开始。" }
      ]
    }
  ]
}
```

> AI 友好考量：AI 输出可直接是 `TiptapBlock[]`，由前端包装为 `{ type: 'doc', content: [...] }`。结构严格定义后，可用 Zod 校验 AI 输出是否合法。

### 5.4 Strict Schema（限制可用 marks/nodes）

- 禁用 TextStyle 等允许任意内联 CSS 的扩展。
- Phase 1 允许 marks：
  - Bold
  - Italic
  - Strike
  - Highlight（用于 diff/审阅，支持多色）

> 是否需要 Link：目前未列入 Phase 1，后续可讨论。

### 5.5 自动保存策略

```ts
interface AutoSaveConfig {
  debounceMs: number;           // 人工编辑：停止输入后延迟保存
  aiDebounceMs: number;         // AI 写入：更短延迟
  maxIntervalMs: number;        // 强制保存间隔（防止长时间未保存）
  saveOnBlur: boolean;          // 编辑器失焦时立即保存
  saveBeforeAiApply: boolean;   // AI 修改前先保存当前状态（便于回滚）
}

// Phase 1 默认配置
const AUTO_SAVE_CONFIG: AutoSaveConfig = {
  debounceMs: 2000,
  aiDebounceMs: 500,
  maxIntervalMs: 30000,
  saveOnBlur: true,
  saveBeforeAiApply: true,
};
```

实现要点：
- 使用 `editor.on('update', callback)` 监听内容变更
- 配合 `lodash.debounce` 或 `use-debounce` 实现防抖
- 保存时更新 `Chapter.updated_at` 和 `Chapter.counts`

> AI 友好考量：`saveBeforeAiApply: true` 确保 AI 应用 patch 前有 before 快照，便于回滚和 history 记录。

### 5.6 光标位置与选区行为

```ts
interface CursorBehavior {
  onChapterOpen: 'start' | 'end' | 'restore_last';
  afterAiInsert: 'end_of_inserted' | 'stay' | 'start_of_inserted';
  afterAiReplace: 'end_of_replaced' | 'select_replaced';
}

// Phase 1 默认配置
const DEFAULT_CURSOR_BEHAVIOR: CursorBehavior = {
  onChapterOpen: 'restore_last',      // 恢复上次编辑位置
  afterAiInsert: 'end_of_inserted',   // 光标移到新内容末尾
  afterAiReplace: 'select_replaced',  // 选中被替换内容，便于审阅
};
```

光标位置持久化：
- 在 `Chapter` 中增加 `last_cursor_position?: number`（存储 ProseMirror position）
- 或存储在前端 localStorage（key: `cursor_{chapter_id}`）

> AI 友好考量：`select_replaced` 让用户一眼看到 AI 改了什么，配合 Highlight 高亮效果更佳。

### 5.7 字数统计实现

```ts
function extractPlainText(doc: TiptapDoc): string {
  const texts: string[] = [];
  
  function walk(node: any) {
    if (node.type === 'text') {
      texts.push(node.text);
    } else if (node.content) {
      node.content.forEach(walk);
    }
  }
  
  walk(doc);
  return texts.join('');
}

function countCharsNoWhitespace(plainText: string): number {
  // 规范化换行
  const normalized = plainText.replace(/\r\n/g, '\n');
  // 移除所有 Unicode 空白字符
  return normalized.replace(/\s/gu, '').length;
}

// 保存时调用
function updateChapterCounts(chapter: Chapter, doc: TiptapDoc): void {
  const plainText = extractPlainText(doc);
  chapter.counts.text_length_no_whitespace = countCharsNoWhitespace(plainText);
  chapter.counts.last_calculated_at = Date.now();
}
```

> AI 友好考量：AI Agent 可查询当前字数决定生成策略（如"还差 2000 字完成本章"）。建议将统计函数抽为通用工具，前后端都可用。

---

## 6. AI 内容的版本控制：proposal + history + block-level patch

你已确认：需要 **block-level patch**（更强 diff/回放）。

### 6.1 Proposal（AI 候选稿，不直接改正文）

- 路径：`magic_novel/ai/proposals/{proposal_id}.json`

```ts
type ProposalStatus = "generated" | "accepted" | "partially_accepted" | "rejected";

interface AiProposal {
  schema_version: number;

  proposal_id: string;     // UUID
  created_at: number;

  project_id: string;
  chapter_id: string;
  chapter_path: string;    // 相对路径（用于快速定位）

  target: {
    type: "cursor" | "block";
    block_id?: string;
    position?: "before" | "after" | "replace";
  };

  prompt: string;
  context_refs: {
    lore_asset_ids?: string[];
    prompt_asset_ids?: string[];
    node_ids?: string[];   // 可选：精确引用条目节点
  };

  model: {
    provider?: string;
    name: string;
    temperature?: number;
    top_p?: number;
  };

  output: {
    format: "text" | "tiptap_json";
    text?: string;
    tiptap_json?: unknown;
  };

  status: ProposalStatus;
}
```

### 6.2 History（章节变更历史，支持回放/撤销）

- 路径：`magic_novel/history/chapters/{chapter_id}.jsonl`
- 格式：JSON Lines（每行一个事件，append-only）

#### 6.2.1 事件结构（建议）

```ts
type Actor = "human" | "ai";

type PatchOp =
  | {
      op: "insert_blocks";
      after_block_id: string | null; // null 表示插入到文档开头
      blocks: unknown[];             // tiptap block nodes（必须包含 attrs.id）
    }
  | {
      op: "update_block";
      block_id: string;
      before: unknown; // 该 block 的旧 JSON
      after: unknown;  // 该 block 的新 JSON
    }
  | {
      op: "delete_blocks";
      block_ids: string[];
    };

interface ChapterHistoryEvent {
  schema_version: number;

  event_id: string;        // UUID
  created_at: number;

  actor: Actor;
  source_proposal_id?: string;

  before_hash: string;
  after_hash: string;

  summary?: string;
  patch: PatchOp[];
}
```

#### 6.2.2 Patch MVP 范围（你已确认）

- Phase 1 patch 操作集：**insert / update / delete**。
- `move` 等复杂操作后置。

---

## 7. 导入（Import）规范

你已确认：A（提示词/设定）和 B（正文初稿）都做；并且目标是把 txt/md/docx 内容转换为 JSON 存储。

### 7.1 导入类型

- `import_asset`：导入为 `magic_novel/` 的 lore/prompt 资产（条目树）。
- `import_manuscript`：导入为 `content/` 的卷/章与 chapter JSON。

### 7.2 资产导入（txt/md/docx → AssetTree JSON）

#### md 解析规则
- 使用 Markdown 标题层级形成树：
  - `#` => level 1 节点
  - `##` => level 2 节点
  - ...
- 每个标题下的段落文本归属到该标题节点的 `content`。
- 无标题内容：挂载到 root（level 0）或最近上级标题。

#### docx 解析规则
- 使用 Word Heading 样式（Heading 1/2/3...）形成树。
- 段落文本归属同上。

#### txt 解析规则
- 默认视为单节点（root 的 `content`）。

### 7.3 正文导入（txt/md/docx → content/ 卷/章）

你已确认：使用 Markdown/Heading 识别卷/章：
- `#` 识别为 **卷**
- `##` 识别为 **章**

2. 对每个卷创建目录：`content/<卷文件夹名>/` 并生成 `_volume.json`。
3. 对每个章生成 chapter JSON：
   - chapter 内部生成稳定 `id`（UUID）。
   - 将正文转换为 Tiptap JSON，并为 block nodes 注入 `attrs.id`。
4. 计算并写入章内计数信息 `counts.text_length_no_whitespace`（以及可选的 `counts.word_count`）。

- 若输入 manuscript 既没有 `#` 也没有 `##`：导入为 `content/001_导入内容.json`（文件名可调整），作为单章。

---

## 8. 导出（Export）规范


### 8.1 导出模式

- `export_tree_multi`：按卷/章多文件导出，保持目录结构。

### 8.2 输出格式

- `docx`：卷用 Heading 1、章用 Heading 2；正文段落与基础 marks 映射（Bold/Italic/Strike/Highlight）。

### 8.3 多文件导出命名


---

## 9. Schema 迁移（Versioning & Migration）

### 9.1 迁移触发

1. 读取 `magic_novel/project.json.schema_version`。
2. 若小于当前 `LATEST_SCHEMA_VERSION`：提示用户升级。
3. 用户确认后执行链式迁移：`v1 -> v2 -> ... -> latest`。

### 9.2 迁移安全策略

- 迁移前备份：`magic_novel/backups/<timestamp>/`（建议备份 `project.json`、所有 `_volume.json`、所有 chapter、lore/prompt 资产）。
- 所有写入必须原子化：写 `*.tmp` → rename 覆盖。

---

## 10. Observability（日志与诊断）


### 10.1 Rust 侧日志（tracing）

- Rust 后端统一使用 `tracing` 体系（`tracing`, `tracing-subscriber`）。
- 日志级别：`ERROR/WARN/INFO/DEBUG/TRACE`。
  - `open_project`（project_root）
  - `scan_content_tree`（content_root）
  - `read_chapter`（path, chapter_id）
  - `save_chapter`（path, chapter_id, bytes_written）
  - `import_asset`（input_path, kind）
  - `import_manuscript`（input_path）
  - `export_book_single`（format, output_path）
  - `export_tree_multi`（format, output_dir）
  - `migrate_schema`（from_version, to_version, backup_dir）
  - `save_ai_proposal`（proposal_id, chapter_id）
  - `append_chapter_history_event`（event_id, chapter_id, actor）

### 10.2 日志落地位置与滚动

- 默认将日志写入**应用数据目录**（由 Tauri 提供的 app data dir），例如：
  - `<app_data_dir>/logs/app-YYYY-MM-DD.log`

### 10.3 隐私与内容脱敏

- 允许记录：文件路径、ID、长度、hash（如 before/after hash）、错误码、耗时等。

### 10.4 前端日志与命令调用

  - 必须展示用户可理解的提示（依据 `AppError.code`）

---

## 11. Error Model（错误类型规范）


### 11.1 禁止使用 `Result<_, String>` 作为协议

- 所有对前端暴露的命令统一返回：`Result<T, AppError>`。
- `AppError` 必须通过 tauri-specta 导出到 TypeScript。

### 11.2 错误码（ErrorCode）


```ts
type ErrorCode =
  | "INVALID_ARGUMENT"
  | "NOT_FOUND"
  | "PERMISSION_DENIED"
  | "IO_ERROR"
  | "JSON_PARSE_ERROR"
  | "SCHEMA_VALIDATION_ERROR"
  | "SCHEMA_VERSION_UNSUPPORTED"
  | "MIGRATION_REQUIRED"
  | "MIGRATION_FAILED"
  | "IMPORT_PARSE_FAILED"
  | "EXPORT_FAILED"
  | "CONFLICT"
  | "INTERNAL";
```

### 11.3 AppError 结构（跨端一致）

```ts
interface AppError {
  code: ErrorCode;

  message: string;

  details?: unknown;

  recoverable?: boolean;
}
```


- `SCHEMA_VALIDATION_ERROR` / `JSON_PARSE_ERROR`：提示"文件损坏或格式不支持"，并引导查看日志。
- `PERMISSION_DENIED`：提示检查目录权限或更换作品库路径。

---

## 12. 前后端接口（Rust Commands）与类型同步


### 12.1 作品与目录扫描

- `set_library_root(path: String) -> Result<(), AppError>`
- `create_project(library_root: String, project_folder_name: String, name: String, author: String) -> Result<String, AppError>`
- `open_project(project_root: String) -> Result<ProjectSnapshot, AppError>`

#### FileNode（必须定义）

```ts
type FileNode =
  | {
      kind: "dir";
      name: string;
      path: string;      // 相对路径
      children: FileNode[];
    }
  | {
      kind: "chapter";
      name: string;      // 文件名（含序号前缀）
      path: string;
      chapter_id: string;
      title: string;
      // 主显示：去空白字符数（counts.text_length_no_whitespace）
      text_length_no_whitespace: number;
      status?: string;
      updated_at: number;
    };

interface ProjectSnapshot {
  project: ProjectMetadata;
  tree: FileNode[]; // content/ 下的树（或直接返回 content 根节点）
}
```


- `read_chapter(path: String) -> Result<Chapter, AppError>`
- `save_chapter(path: String, data: Chapter) -> Result<(), AppError>`
  - 保存时：更新 `updated_at`，并重算 `counts.text_length_no_whitespace`（主字数口径）。


- `create_volume(parent_dir: String, folder_name: String, title: String) -> Result<String, AppError>`
- `create_chapter(volume_dir: String, file_name: String, title: String) -> Result<String, AppError>`

### 12.4 导入/导出命令（Phase 1 必须具备接口）

- `import_asset(project_root: String, input_path: String, kind: "lore" | "prompt") -> Result<String, AppError>`
  - 返回 asset_id 或生成的 JSON 路径。
- `import_manuscript(project_root: String, input_path: String) -> Result<(), AppError>`

- `export_book_single(project_root: String, format: "txt" | "md" | "docx", output_path: String) -> Result<(), AppError>`
- `export_tree_multi(project_root: String, format: "txt" | "md" | "docx", output_dir: String) -> Result<(), AppError>`

### 12.5 AI proposal / history 相关（为右侧助手预留）

- `save_ai_proposal(project_root: String, proposal: AiProposal) -> Result<(), AppError>`
- `append_chapter_history_event(project_root: String, chapter_id: String, event: ChapterHistoryEvent) -> Result<(), AppError>`


---

## 13. Zustand Store（Frontend 状态管理）


```ts
interface EditorState {
  editor: Editor | null;

  projectRoot: string | null;
  libraryRoot: string | null;

  currentChapterPath: string | null;
  currentChapterId: string | null;
  isSaving: boolean;
  isDirty: boolean;

  setEditor: (instance: Editor) => void;
  openProject: (projectRoot: string) => Promise<void>;
  loadChapter: (path: string) => Promise<void>;
  saveCurrentChapter: () => Promise<void>;

  // AI 预留：支持按 block 定位插入（不要只插 text）
  applyPatch: (patch: unknown) => void;
}
```

---

## 14. AI Agent 接口预留

> 本节为 Phase 2+ AI 写作助手预留接口设计，Phase 1 可先不实现 UI，但数据结构和 Store 方法需预留。

### 14.1 前端 AI Agent Hooks

```ts
interface AiAgentHooks {
  // AI 读取上下文
  getCurrentChapterContent: () => TiptapDoc | null;
  getBlockById: (blockId: string) => TiptapBlock | null;
  getSelectedBlocks: () => TiptapBlock[];
  getCurrentWordCount: () => number;
  
  // AI 写入
  insertBlocksAfter: (afterBlockId: string | null, blocks: TiptapBlock[]) => void;
  replaceBlock: (blockId: string, newBlock: TiptapBlock) => void;
  deleteBlocks: (blockIds: string[]) => void;
  
  // AI 标记（用于审阅）
  highlightBlocks: (blockIds: string[], color: 'ai-new' | 'ai-modified') => void;
  clearAiHighlights: () => void;
}
```

### 14.2 AI 操作核心原则

1. **Block ID 是 AI 操作的锚点**：所有定位、修改都依赖稳定的 block id
2. **结构化输出**：AI 应输出 Tiptap JSON 而非纯文本，便于精确 patch
3. **可审阅**：AI 修改后高亮显示，用户可快速识别
4. **可回滚**：每次 AI 操作前保存快照 + 记录 history event

### 14.3 AI 生成内容的 Highlight 颜色约定

```ts
const AI_HIGHLIGHT_COLORS = {
  'ai-new': '#c6f6d5',      // 绿色：AI 新增内容
  'ai-modified': '#fefcbf', // 黄色：AI 修改内容
  'ai-pending': '#e9d8fd',  // 紫色：AI 待确认内容
};
```

---

## 15. Phase 1 验收标准（Acceptance Criteria）

### 15.1 工程化
- `pnpm tauri dev` 启动无报错。
- Rust 与 TS 类型通过 tauri-specta 同步；关键 JSON 在前端用 Zod 校验。

### 15.2 持久化闭环
- 新建作品（在 library_root 下）→ 自动生成 `magic_novel/` 与 `content/`。
- 新建卷 → 生成卷目录与 `_volume.json`。
- 新建章 → 写入 chapter JSON。

### 15.3 Block Identity
- 任意 paragraph/heading/blockquote 节点必须包含 `attrs.id`。
- 复制粘贴后 id 被重写为新 UUID。
- 保存/重开后 id 保持不变。

### 15.4 编辑器功能
- 自动保存功能正常工作（debounce 2s）。
- 字数统计实时更新。
- 光标位置可恢复。

### 15.5 导入
- 导入 `md/docx/txt` 为 lore/prompt 资产：生成 AssetTree JSON，标题层级正确。
- 导入 manuscript（md/docx）：`#` 生成卷、`##` 生成章，生成 chapter JSON 且 block ids 完整。

### 15.6 导出
- 导出整本为 txt/md/docx 成功。

### 15.7 AI 变更底座
- 能保存 proposal 文件。
- 能向章节 history 追加事件（jsonl）。
- patch 至少支持 insert/update/delete 三类操作结构，并可被前端解释执行（Phase 1 可先不做完整 UI）。
