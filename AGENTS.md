# Repository Guidelines

## Project Structure & Module Organization
`src/` is the React/Vite frontend. `src/features` is the application-facing boundary that pages and components should consume; `src/components` and `src/pages` must not import `@/lib/tauri-commands`, `@/platform/tauri/clients`, or `@/lib/tool-gateway` directly. `src/magic-ui` is presentation-only and cannot depend on `state`, `stores`, `agent`, `features`, or `platform`. `src-tauri/src/lib.rs` wires the Tauri commands and the Rust layers: legacy `commands/services/models/utils` plus the newer `application/domain/infrastructure/interfaces/kernel` stack, with agent-specific subsystems under `agent_engine`, `agent_tools`, `mission`, `review`, `knowledge`, and `llm`.

## Build, Test, and Development Commands
Run commands from `magic-novel/`. Use `pnpm dev` for Vite, `pnpm build` for the frontend bundle, and `pnpm lint` for ESLint. Rust/Tauri work should target `src-tauri/Cargo.toml`, for example `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo run --manifest-path src-tauri/Cargo.toml --bin tool_provider_smoke`, or the exact snapshot check behind `pnpm test:tool-schema-inventory`. `src-tauri/tauri.conf.json` already hooks Tauri builds to `pnpm build`.

## Coding Style & Naming Conventions
ESLint uses `@eslint/js`, `typescript-eslint`, `react-hooks`, and `react-refresh`. App TS/TSX is intentionally not strict-mode TypeScript (`tsconfig.app.json` sets `strict: false`), while `vite.config.ts` is strict through `tsconfig.node.json`. Honor the `@/*` path alias. Follow the documented file-size limits from `README.md`: Rust files should stay near 600 lines (800 with tests), and TS/TSX files near 500 lines (700 with tests). In Rust, prefer `pub(crate)` for internal splits and keep helper functions private.

## Testing Guidelines
Prefer targeted checks before broad suites. Useful examples are `cargo test --manifest-path src-tauri/Cargo.toml agent_tools::registry::tests::test_tool_schema_inventory_matches_snapshot -- --exact`, `pnpm test:tool-agent`, and `pnpm test:agent-chat`. Add or update script-driven regressions in `scripts/` when behavior spans frontend and Tauri.

## Commit & Pull Request Guidelines
Recent history uses `feat`, `fix`, `merge`, and `wip`, sometimes with milestone scopes such as `feat(M4): ...`. Follow that shape: imperative subject, optional milestone scope, and no vague “update code” messages. No pull request template was found under `.github`.

## Agent Instructions
For Cargo operations, default to the repository target directory at `src-tauri/target`. Do not pass ad-hoc `--target-dir` values for normal `cargo build`, `cargo test`, `cargo run`, `cargo clippy`, or `cargo fmt` flows. Use an isolated `--target-dir` only when the user explicitly asks for isolation or the shared target is demonstrably blocked or corrupted; if isolation is necessary, state the reason and remove the temporary target directory before finishing. Do not leave routine `target_*` directories beside the repo.
