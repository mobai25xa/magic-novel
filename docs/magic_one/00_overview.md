# Magic Novel - 开发总览与路线图

> 本文档是 Phase 1 开发的总览，包含项目结构、开发阶段划分、里程碑和依赖关系。

---

## 1. 项目信息

| 项目名称 | Magic Novel |
|----------|-------------|
| 版本 | Phase 1 (Foundation) |
| 技术栈 | Tauri v2 + React 18 + TypeScript + Tiptap |
| 目标 | 构建本地优先、AI-ready 的小说写作软件内核 |
| 图标 | `docs/image/magic.png` |

---

## 2. 目录结构（目标）

```text
magic-novel/
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 入口
│   │   ├── lib.rs                # 库导出
│   │   ├── commands/             # Tauri 命令
│   │   │   ├── mod.rs
│   │   │   ├── project.rs        # 项目操作
│   │   │   ├── chapter.rs        # 章节操作
│   │   │   ├── volume.rs         # 卷操作
│   │   │   ├── import.rs         # 导入
│   │   │   ├── export.rs         # 导出
│   │   │   └── ai.rs             # AI 相关
│   │   ├── models/               # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── project.rs
│   │   │   ├── chapter.rs
│   │   │   ├── volume.rs
│   │   │   ├── asset.rs
│   │   │   ├── proposal.rs
│   │   │   └── error.rs
│   │   ├── services/             # 业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── file_system.rs
│   │   │   ├── migration.rs
│   │   │   └── word_count.rs
│   │   └── utils/                # 工具函数
│   │       ├── mod.rs
│   │       └── atomic_write.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/                    # 应用图标
│       └── icon.png              # 从 magic.png 复制
├── src/                          # React 前端
│   ├── main.tsx                  # 入口
│   ├── App.tsx                   # 主应用
│   ├── components/
│   │   ├── layout/
│   │   │   ├── TopBar.tsx
│   │   │   ├── LeftPanel.tsx
│   │   │   ├── EditorPanel.tsx
│   │   │   └── RightPanel.tsx
│   │   ├── editor/
│   │   │   ├── NovelEditor.tsx
│   │   │   ├── EditorToolbar.tsx
│   │   │   └── extensions/
│   │   │       └── unique-id.ts
│   │   ├── tree/
│   │   │   ├── ContentTree.tsx
│   │   │   └── TreeNode.tsx
│   │   └── ui/                   # shadcn/ui 组件
│   ├── stores/
│   │   ├── editor-store.ts
│   │   └── project-store.ts
│   ├── hooks/
│   │   ├── use-auto-save.ts
│   │   └── use-word-count.ts
│   ├── lib/
│   │   ├── tauri-commands.ts     # Tauri 命令封装
│   │   ├── schemas.ts            # Zod schemas
│   │   └── utils.ts
│   └── styles/
│       └── editor.css
├── docs/
│   ├── md/
│   │   └── analysis01.md         # 需求分析文档
│   ├── magic_one/                # 开发文档
│   │   ├── 00_overview.md        # 本文档
│   │   ├── 01_project_setup.md
│   │   ├── 02_rust_backend.md
│   │   ├── 03_frontend_editor.md
│   │   ├── 04_ui_components.md
│   │   ├── 05_import_export.md
│   │   └── 06_ai_infrastructure.md
│   └── image/
│       └── magic.png
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 3. 开发阶段划分

### Phase 1-A: 项目脚手架（Day 1）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 初始化 Tauri v2 + React 项目 | 可运行的空白应用 | `pnpm tauri dev` 正常启动 |
| 配置 Tailwind + shadcn/ui | 样式系统就绪 | 能使用 shadcn 组件 |
| 配置 tauri-specta | 类型同步就绪 | Rust 类型能导出到 TS |
| 设置应用图标 | 应用显示 magic.png 图标 | 窗口图标正确 |

### Phase 1-B: Rust 后端核心（Day 2-3）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 定义数据模型 | models/*.rs | 所有 JSON 结构对应 Rust struct |
| 实现项目操作 | create_project, open_project | 能创建/打开项目 |
| 实现卷/章操作 | create_volume, create_chapter, read/save_chapter | CRUD 完整 |
| 实现原子写入 | atomic_write.rs | 写入不会损坏文件 |
| 实现错误处理 | AppError | 统一错误类型 |

### Phase 1-C: 前端编辑器（Day 4-5）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 实现 UniqueIdExtension | extensions/unique-id.ts | block 自动分配 id |
| 配置 Tiptap 编辑器 | NovelEditor.tsx | 支持所有 Phase 1 marks/nodes |
| 实现编辑器工具栏 | EditorToolbar.tsx | Bold/Italic/Strike/Highlight/Heading |
| 实现自动保存 | use-auto-save.ts | debounce 2s 保存 |
| 实现字数统计 | use-word-count.ts | 实时更新 |

### Phase 1-D: UI 布局与交互（Day 6-7）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 三栏布局 | TopBar + LeftPanel + EditorPanel + RightPanel | 布局正确 |
| 左栏目录树 | ContentTree.tsx | 显示卷/章结构 |
| 左下信息面板 | ChapterInfo 区域 | 显示字数/状态 |
| 右栏 AI 占位 | RightPanel 显示"AI 助手即将推出" | 占位正确 |
| 目录树交互 | 新建卷/章、重命名、删除 | 操作正常 |

### Phase 1-E: 导入导出（Day 8-9）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 实现资产导入 | import_asset 命令 | md/docx/txt → AssetTree |
| 实现正文导入 | import_manuscript 命令 | md/docx → 卷/章 |
| 实现单文件导出 | export_book_single 命令 | 整本导出 txt/md/docx |
| 实现多文件导出 | export_tree_multi 命令 | 按目录结构导出 |

### Phase 1-F: AI 底座与收尾（Day 10）

| 任务 | 产出 | 验收标准 |
|------|------|----------|
| 实现 proposal 存储 | save_ai_proposal 命令 | 能保存 proposal JSON |
| 实现 history 追加 | append_chapter_history_event 命令 | 能追加 jsonl |
| 前端 patch 解释器 | applyPatch 方法 | 能执行 insert/update/delete |
| 集成测试 | 完整流程测试 | 创建→编辑→保存→导出 |

---

## 4. 里程碑与依赖关系

```
Phase 1-A (脚手架)
    │
    ├─────────────────┐
    ▼                 ▼
Phase 1-B          Phase 1-C
(Rust 后端)        (前端编辑器)
    │                 │
    └────────┬────────┘
             ▼
        Phase 1-D
        (UI 布局)
             │
             ▼
        Phase 1-E
        (导入导出)
             │
             ▼
        Phase 1-F
        (AI 底座)
```

---

## 5. 技术依赖版本

### Rust (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = ["specta"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
specta = "2"
tauri-specta = "2"
```

### Frontend (package.json)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tiptap/react": "^2",
    "@tiptap/pm": "^2",
    "@tiptap/extension-document": "^2",
    "@tiptap/extension-text": "^2",
    "@tiptap/extension-paragraph": "^2",
    "@tiptap/extension-heading": "^2",
    "@tiptap/extension-blockquote": "^2",
    "@tiptap/extension-hard-break": "^2",
    "@tiptap/extension-bold": "^2",
    "@tiptap/extension-italic": "^2",
    "@tiptap/extension-strike": "^2",
    "@tiptap/extension-highlight": "^2",
    "@tiptap/extension-history": "^2",
    "react": "^18",
    "react-dom": "^18",
    "zustand": "^4",
    "zod": "^3",
    "uuid": "^9",
    "lodash.debounce": "^4"
  },
  "devDependencies": {
    "@types/react": "^18",
    "@types/react-dom": "^18",
    "@types/lodash.debounce": "^4",
    "typescript": "^5",
    "vite": "^5",
    "@vitejs/plugin-react": "^4",
    "tailwindcss": "^3",
    "autoprefixer": "^10",
    "postcss": "^8"
  }
}
```

---

## 6. 文档索引

| 文档 | 内容 |
|------|------|
| [01_project_setup.md](./01_project_setup.md) | 项目初始化完整步骤 |
| [02_rust_backend.md](./02_rust_backend.md) | Rust 后端模块详细设计 |
| [03_frontend_editor.md](./03_frontend_editor.md) | Tiptap 编辑器实现细节 |
| [04_ui_components.md](./04_ui_components.md) | UI 组件与布局设计 |
| [05_import_export.md](./05_import_export.md) | 导入导出功能实现 |
| [06_ai_infrastructure.md](./06_ai_infrastructure.md) | AI 底座设计 |

---

## 7. 开发约定

### 代码风格
- Rust: `cargo fmt` + `cargo clippy`
- TypeScript: ESLint + Prettier
- 不写注释，代码自解释

### Git 提交规范
```
<type>: <description>

Types: feat, fix, refactor, docs, style, test, chore
```

### 分支策略
- `main`: 稳定版本
- `dev`: 开发分支
- `feature/*`: 功能分支
