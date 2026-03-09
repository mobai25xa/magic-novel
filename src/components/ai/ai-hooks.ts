import { useTranslation } from '@/hooks/use-translation'

export function useAiTranslations() {
  const { translations } = useTranslation()
  return translations.ai
}
