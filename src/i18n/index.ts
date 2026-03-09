import type { Language } from '@/types/theme'
import { zh } from './locales/zh'
import { en } from './locales/en'

const translations = {
  zh,
  en,
}

export type TranslationKey = keyof typeof zh
export type NestedTranslationKey<T> = T extends object
  ? { [K in keyof T]: T[K] extends object ? `${K & string}.${NestedTranslationKey<T[K]> & string}` : K & string }[keyof T]
  : never

export function getTranslation(language: Language) {
  return translations[language] || translations.zh
}

// Helper function to get nested translation value
export function getNestedValue(obj: unknown, path: string): string {
  const keys = path.split('.')
  let result: unknown = obj
  for (const key of keys) {
    if (result && typeof result === 'object' && key in result) {
      result = (result as Record<string, unknown>)[key]
    } else {
      return path // Return the path as fallback
    }
  }
  return typeof result === 'string' ? result : path
}

export { zh, en }
