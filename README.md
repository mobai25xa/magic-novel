# React + TypeScript + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Babel](https://babeljs.io/) (or [oxc](https://oxc.rs) when used in [rolldown-vite](https://vite.dev/guide/rolldown)) for Fast Refresh
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/) for Fast Refresh

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type-aware lint rules:

```js
export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...

      // Remove tseslint.configs.recommended and replace with this
      tseslint.configs.recommendedTypeChecked,
      // Alternatively, use this for stricter rules
      tseslint.configs.strictTypeChecked,
      // Optionally, add this for stylistic rules
      tseslint.configs.stylisticTypeChecked,

      // Other configs...
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

## 代码规范

### 单文件行数限制

为保持代码可读性和可维护性，项目对单文件代码量有明确限制：

| 语言 | 纯逻辑上限 | 含测试上限 |
|------|-----------|-----------|
| Rust (`.rs`) | 600 行 | 800 行 |
| TypeScript/TSX (`.ts`/`.tsx`) | 500 行 | 700 行 |

### 拆分原则

- 按职责边界拆分，而非机械按行数切割
- 提取的模块使用 `pub(crate)` 可见性，仅在需要跨 crate 时使用 `pub`
- 拆分后每个文件应有清晰的单一职责（如：调度、格式化、错误处理、schema 构建）
- 辅助函数保持私有，仅暴露必要的公共接口

### 示例：`agent_engine/` 模块结构

```
agent_engine/
├── loop_engine.rs        # 核心循环（AgentLoop::run）
├── tool_scheduler.rs     # 工具调度与并行分组
├── tool_dispatch.rs      # 工具执行与输入解析
├── tool_schemas.rs       # OpenAI 工具 schema 构建
├── tool_formatters.rs    # 工具结果格式化
├── tool_errors.rs        # 错误构造与资源锁辅助
├── context_loader.rs     # 上下文加载/注入/缓存
├── worker_dispatch.rs    # Worker 子循环调度
├── compaction.rs         # 上下文压缩
├── ...
```
