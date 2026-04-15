# Magic Novel

Magic Novel 是一个面向长篇创作的桌面写作工具，聚焦于小说项目管理、规划承接、章节编辑和 AI 辅助创作。

## 功能概览

- 工作台：创建、打开和管理作品
- 项目首页：查看规划 Manifest、合同状态与开写门槛
- 编辑器：按卷 / 章节组织内容，支持保存、导入、导出与字数统计
- AI 辅助：支持剧情推演、角色润色、章节协作等创作场景
- 多模型配置：支持 OpenAI、Anthropic、Gemini 及 OpenAI 兼容接口

## 技术栈

- 前端：React 19 + TypeScript + Vite
- 桌面端：Tauri 2
- 后端：Rust
- UI / 状态管理：Radix UI、TipTap、Zustand

## 环境要求

在本地启动前，请先准备：

- Node.js 20+
- pnpm 9+
- Rust stable
- Tauri 2 开发环境

如需安装 Tauri 环境，可参考官方文档：<https://tauri.app/start/prerequisites/>

## 快速开始

### 1. 安装依赖

```bash
pnpm install
```

### 2. 启动前端开发环境

```bash
pnpm dev
```

### 3. 启动桌面应用开发环境

```bash
pnpm tauri dev
```

## 构建

### 构建前端资源

```bash
pnpm build
```

### 构建桌面应用

```bash
pnpm tauri build
```

## 常用校验命令

```bash
pnpm lint
pnpm test:tool-agent
pnpm test:agent-chat
pnpm test:session-regression
pnpm test:tool-provider-smoke
pnpm check:governance
```

## 使用流程

1. 启动应用后，先在设置页配置作品存储目录和模型 Provider。
2. 在工作台创建新作品，或打开已有项目。
3. 进入项目首页，查看规划合同与推荐下一步。
4. 满足写作门槛后进入编辑器，开始卷章创作。
5. 按需使用 AI 助手完成剧情推演、章节生成或内容润色。

## 目录结构

```text
magic-novel/
├─ src/          # React 前端
├─ src-tauri/    # Tauri / Rust 后端
├─ public/       # 静态资源
├─ scripts/      # 测试与治理脚本
├─ schemas/      # 工具与协议快照
├─ tests/        # Rust 集成测试
└─ docs/         # 补充资料
```

## 配置说明

- Provider 配置会持久化到用户目录下的 `.magic/setting.json`
- 当前支持的 Provider 类型：
  - OpenAI
  - Anthropic
  - Gemini
  - OpenAI Compatible
