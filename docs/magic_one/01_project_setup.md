# 项目初始化指南

> 本文档详细描述如何从零初始化 Magic Novel 项目。

---

## 1. 环境要求

### 1.1 必需工具

| 工具 | 版本要求 | 安装命令/链接 |
|------|----------|---------------|
| Node.js | >= 18.0 | https://nodejs.org |
| pnpm | >= 8.0 | `npm install -g pnpm` |
| Rust | >= 1.70 | https://rustup.rs |
| Tauri CLI | v2 | `cargo install tauri-cli --version "^2"` |

### 1.2 验证安装

```bash
node --version    # v18.x 或更高
pnpm --version    # 8.x 或更高
rustc --version   # 1.70 或更高
cargo tauri --version  # tauri-cli 2.x
```

---

## 2. 项目初始化

### 2.1 创建 Tauri v2 + React 项目

```bash
# 创建项目
pnpm create tauri-app magic-novel --template react-ts

# 进入项目目录
cd magic-novel

# 安装依赖
pnpm install
```

### 2.2 验证基础项目

```bash
pnpm tauri dev
```

应该能看到一个空白的 Tauri 窗口。

---

## 3. 配置 Tailwind CSS

### 3.1 安装依赖

```bash
pnpm add -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

### 3.2 配置 tailwind.config.js

```js
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

### 3.3 添加 Tailwind 指令到 src/styles/index.css

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

### 3.4 在 src/main.tsx 中引入样式

```tsx
import './styles/index.css'
```

---

## 4. 配置 shadcn/ui

### 4.1 初始化 shadcn/ui

```bash
pnpm dlx shadcn-ui@latest init
```

选项：
- TypeScript: Yes
- Style: Default
- Base color: Slate
- CSS variables: Yes
- tailwind.config.js location: tailwind.config.js
- Components location: src/components/ui
- Utils location: src/lib/utils

### 4.2 安装常用组件

```bash
pnpm dlx shadcn-ui@latest add button
pnpm dlx shadcn-ui@latest add input
pnpm dlx shadcn-ui@latest add dialog
pnpm dlx shadcn-ui@latest add dropdown-menu
pnpm dlx shadcn-ui@latest add tooltip
pnpm dlx shadcn-ui@latest add scroll-area
pnpm dlx shadcn-ui@latest add separator
```

---

## 5. 安装 Tiptap 编辑器

### 5.1 安装核心依赖

```bash
pnpm add @tiptap/react @tiptap/pm @tiptap/starter-kit
```

### 5.2 安装独立扩展

```bash
pnpm add @tiptap/extension-document \
         @tiptap/extension-text \
         @tiptap/extension-paragraph \
         @tiptap/extension-heading \
         @tiptap/extension-blockquote \
         @tiptap/extension-hard-break \
         @tiptap/extension-bold \
         @tiptap/extension-italic \
         @tiptap/extension-strike \
         @tiptap/extension-highlight \
         @tiptap/extension-history
```

---

## 6. 安装其他前端依赖

```bash
# 状态管理
pnpm add zustand

# Schema 校验
pnpm add zod

# UUID 生成
pnpm add uuid
pnpm add -D @types/uuid

# 防抖
pnpm add lodash.debounce
pnpm add -D @types/lodash.debounce
```

---

## 7. 配置 Rust 后端

### 7.1 更新 src-tauri/Cargo.toml

```toml
[package]
name = "magic-novel"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
specta = { version = "2", features = ["derive"] }
tauri-specta = { version = "2", features = ["typescript"] }
thiserror = "1"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

### 7.2 创建目录结构

```bash
cd src-tauri/src
mkdir commands models services utils
```

### 7.3 创建模块文件

```bash
# 创建 mod.rs 文件
touch commands/mod.rs models/mod.rs services/mod.rs utils/mod.rs

# 创建具体模块文件
touch commands/project.rs commands/chapter.rs commands/volume.rs
touch commands/import.rs commands/export.rs commands/ai.rs
touch models/project.rs models/chapter.rs models/volume.rs
touch models/asset.rs models/proposal.rs models/error.rs
touch services/file_system.rs services/migration.rs services/word_count.rs
touch utils/atomic_write.rs
```

---

## 8. 配置 tauri-specta 类型同步

### 8.1 在 src-tauri/src/lib.rs 中配置

```rust
use specta::collect_types;
use tauri_specta::ts;

mod commands;
mod models;
mod services;
mod utils;

pub fn run() {
    let builder = tauri::Builder::default();
    
    #[cfg(debug_assertions)]
    let builder = {
        let specta_builder = ts::builder()
            .commands(collect_types![
                commands::project::create_project,
                commands::project::open_project,
                commands::chapter::read_chapter,
                commands::chapter::save_chapter,
                // ... 其他命令
            ])
            .path("../src/lib/tauri-bindings.ts");
        
        specta_builder.into_plugin_with_setup(builder, |_app, plugin| {
            plugin.mount();
            Ok(())
        })
    };
    
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 8.2 生成类型文件

```bash
pnpm tauri dev
# 首次运行会自动生成 src/lib/tauri-bindings.ts
```

---

## 9. 设置应用图标

### 9.1 复制图标文件

```bash
# 从 docs/image/magic.png 复制到 src-tauri/icons/
cp docs/image/magic.png src-tauri/icons/icon.png
```

### 9.2 生成多尺寸图标

```bash
# 使用 Tauri 图标生成器
cargo tauri icon src-tauri/icons/icon.png
```

这会在 `src-tauri/icons/` 下生成所有需要的图标尺寸。

### 9.3 更新 tauri.conf.json

确认 `src-tauri/tauri.conf.json` 中图标配置正确：

```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

---

## 10. 配置窗口

### 10.1 更新 tauri.conf.json 窗口配置

```json
{
  "app": {
    "windows": [
      {
        "title": "Magic Novel",
        "width": 1400,
        "height": 900,
        "minWidth": 1000,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false,
        "center": true
      }
    ]
  }
}
```

---

## 11. 创建基础前端结构

### 11.1 创建目录

```bash
cd src
mkdir -p components/layout components/editor components/tree components/ui
mkdir stores hooks lib styles
```

### 11.2 创建基础文件

```bash
# 布局组件
touch components/layout/TopBar.tsx
touch components/layout/LeftPanel.tsx
touch components/layout/EditorPanel.tsx
touch components/layout/RightPanel.tsx

# 编辑器组件
touch components/editor/NovelEditor.tsx
touch components/editor/EditorToolbar.tsx
mkdir components/editor/extensions
touch components/editor/extensions/unique-id.ts

# 目录树
touch components/tree/ContentTree.tsx
touch components/tree/TreeNode.tsx

# 状态管理
touch stores/editor-store.ts
touch stores/project-store.ts

# Hooks
touch hooks/use-auto-save.ts
touch hooks/use-word-count.ts

# 工具库
touch lib/tauri-commands.ts
touch lib/schemas.ts
touch lib/utils.ts

# 样式
touch styles/editor.css
```

---

## 12. 初始化验证清单

完成以上步骤后，执行以下验证：

- [ ] `pnpm tauri dev` 正常启动
- [ ] 窗口标题显示 "Magic Novel"
- [ ] 窗口图标显示正确（magic.png）
- [ ] Tailwind 样式生效
- [ ] shadcn/ui Button 组件可用
- [ ] 无 TypeScript 编译错误
- [ ] 无 Rust 编译错误

---

## 13. 常见问题

### Q1: Tauri v2 安装失败

确保 Rust 版本 >= 1.70，并更新 cargo：

```bash
rustup update
cargo install tauri-cli --version "^2" --force
```

### Q2: tauri-specta 类型不更新

删除旧的生成文件重新生成：

```bash
rm src/lib/tauri-bindings.ts
pnpm tauri dev
```

### Q3: 图标不显示

确保运行了 `cargo tauri icon` 命令生成所有尺寸。

---

## 下一步

项目初始化完成后，继续阅读 [02_rust_backend.md](./02_rust_backend.md) 开始 Rust 后端开发。
