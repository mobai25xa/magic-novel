import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores(['dist', 'src-tauri/target/**']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@typescript-eslint/ban-ts-comment': 'warn',
      '@typescript-eslint/no-require-imports': 'warn',
      'react-refresh/only-export-components': 'off',
      'react-hooks/exhaustive-deps': 'warn',
      'react-hooks/immutability': 'warn',
      'react-hooks/set-state-in-effect': 'warn',
      'react-hooks/use-memo': 'warn',
      'no-empty': ['warn', { allowEmptyCatch: true }],
    },
  },
  {
    files: ['src/components/**/*.{ts,tsx}', 'src/pages/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: [
                '@/lib/tauri-commands',
                '@/platform/tauri/clients',
                '@/platform/tauri/clients/*',
                '@/lib/tool-gateway',
                '@/lib/tool-gateway/*',
              ],
              message: 'Use feature facades from src/features instead of importing infra/platform modules in components/pages.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['src/magic-ui/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: [
                '@/state',
                '@/state/*',
                '@/agent',
                '@/agent/*',
                '@/stores',
                '@/stores/*',
                '@/features',
                '@/features/*',
                '@/platform',
                '@/platform/*',
                '@/lib/tauri-commands',
                '@/lib/tauri-commands/*',
                '@/lib/tool-gateway',
                '@/lib/tool-gateway/*',
                '@/lib/agent-chat',
                '@/lib/agent-chat/*',
              ],
              message: 'magic-ui must stay presentation-only and cannot depend on app state/agent/features/platform layers.',
            },
          ],
        },
      ],
    },
  },
])
