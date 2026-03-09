import type { ProviderType } from './settings-types'

export type EditorFontPresetKey = 'sans' | 'serif' | 'kaiti' | 'fangsong'

export type EditorTextAlign = 'center' | 'left'

export const EDITOR_FONT_PRESETS: Record<EditorFontPresetKey, { label: { zh: string; en: string }; fontFamily: string }> = {
  sans: {
    label: { zh: '黑体 (默认)', en: 'Sans-serif (Default)' },
    fontFamily: '"PingFang SC", "Microsoft YaHei", "Noto Sans SC", "Hiragino Sans GB", "WenQuanYi Micro Hei", -apple-system, BlinkMacSystemFont, sans-serif',
  },
  serif: {
    label: { zh: '宋体', en: 'Serif' },
    fontFamily: '"Noto Serif SC", "STSong", "SimSun", "Songti SC", serif',
  },
  kaiti: {
    label: { zh: '楷体', en: 'Kaiti' },
    fontFamily: '"STKaiti", "KaiTi", "Kaiti SC", serif',
  },
  fangsong: {
    label: { zh: '仿宋', en: 'Fangsong' },
    fontFamily: '"STFangsong", "FangSong", "Fangsong SC", serif',
  },
}

export const DEFAULT_PROJECT_GENRES = ['玄幻', '仙侠', '都市', '科幻', '悬疑', '历史', '言情', '同人', '轻小说'] as const

export interface ProviderConfig {
  defaultBaseUrl: string
  baseUrlPlaceholder: string
  apiKeyPlaceholder: string
  presetModels: string[]
}

export const PROVIDER_CONFIGS: Record<ProviderType, ProviderConfig> = {
  openai: {
    defaultBaseUrl: 'https://api.openai.com/v1',
    baseUrlPlaceholder: 'https://api.openai.com/v1',
    apiKeyPlaceholder: 'sk-...',
    presetModels: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo', 'o1', 'o1-mini', 'o3-mini'],
  },
  anthropic: {
    defaultBaseUrl: 'https://api.anthropic.com',
    baseUrlPlaceholder: 'https://api.anthropic.com',
    apiKeyPlaceholder: 'sk-ant-...',
    presetModels: ['claude-sonnet-4-5-20250514', 'claude-haiku-4-5-20250514', 'claude-opus-4-20250514', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022'],
  },
  gemini: {
    defaultBaseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    baseUrlPlaceholder: 'https://generativelanguage.googleapis.com/v1beta',
    apiKeyPlaceholder: 'AIza...',
    presetModels: ['gemini-2.0-flash', 'gemini-2.0-flash-lite', 'gemini-1.5-pro', 'gemini-1.5-flash'],
  },
  'openai-compatible': {
    defaultBaseUrl: '',
    baseUrlPlaceholder: 'https://your-api-endpoint/v1',
    apiKeyPlaceholder: 'sk-...',
    presetModels: [],
  },
}
