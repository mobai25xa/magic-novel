import { useMemo } from 'react'
import { useSettingsStore } from '@/stores/settings-store'
import { getTranslation, getNestedValue } from '@/i18n'
import type { Translations } from '@/i18n/locales/zh'

export function useTranslation() {
  const language = useSettingsStore((state) => state.language)
  
  const t = useMemo(() => {
    const translations = getTranslation(language)
    
    return (key: string): string => {
      return getNestedValue(translations, key)
    }
  }, [language])
  
  const translations = useMemo(() => getTranslation(language), [language])
  
  return { t, translations, language }
}

// Type-safe translation hook with dot notation support
export function useT() {
  const { t } = useTranslation()
  return t
}

// Direct access to translation object
export function useTranslations(): Translations {
  const { translations } = useTranslation()
  return translations
}
