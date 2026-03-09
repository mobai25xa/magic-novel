import { scanProjectsForSettings } from '@/features/settings-management'

export async function syncProjectsAfterDirectoryChange(input: {
  nextDir: string
  clearAllProjects: () => void
}) {
  try {
    localStorage.removeItem('magic-novel-projects')
    input.clearAllProjects()

    await scanProjectsForSettings(input.nextDir)
  } catch (error) {
    console.error('Failed to switch directory:', error)
  }
}

export function applyTempSettings(input: {
  tempGoal: number
  setDailyWordGoal: (value: number) => void
  tempTheme: import('@/types/theme').ThemeMode
  setTheme: (value: import('@/types/theme').ThemeMode) => void
  tempLanguage: import('@/types/theme').Language
  setLanguage: (value: import('@/types/theme').Language) => void
  tempFirstLineIndent: boolean
  setFirstLineIndent: (value: boolean) => void
  tempEditorFontSize: number
  setEditorFontSize: (value: number) => void
  tempEditorLineHeight: number
  setEditorLineHeight: (value: number) => void
  tempEditorContentWidth: number
  setEditorContentWidth: (value: number) => void
  tempEditorFontFamily: import('@/state/settings').EditorFontPresetKey
  setEditorFontFamily: (value: import('@/state/settings').EditorFontPresetKey) => void
  tempEditorTextAlign: import('@/state/settings').EditorTextAlign
  setEditorTextAlign: (value: import('@/state/settings').EditorTextAlign) => void
  tempApprovalMode: import('@/state/settings').ApprovalMode
  setApprovalMode: (value: import('@/state/settings').ApprovalMode) => void
  tempCapabilityMode: import('@/state/settings').CapabilityMode
  setCapabilityMode: (value: import('@/state/settings').CapabilityMode) => void
  tempProjectGenres: string[]
  setProjectGenres: (value: string[]) => void
}) {
  if (Number.isFinite(input.tempGoal) && input.tempGoal > 0) {
    input.setDailyWordGoal(input.tempGoal)
  }

  input.setTheme(input.tempTheme)
  input.setLanguage(input.tempLanguage)
  input.setFirstLineIndent(input.tempFirstLineIndent)
  input.setEditorFontSize(input.tempEditorFontSize)
  input.setEditorLineHeight(input.tempEditorLineHeight)
  input.setEditorContentWidth(input.tempEditorContentWidth)
  input.setEditorFontFamily(input.tempEditorFontFamily)
  input.setEditorTextAlign(input.tempEditorTextAlign)
  input.setApprovalMode(input.tempApprovalMode)
  input.setCapabilityMode(input.tempCapabilityMode)
  input.setProjectGenres(input.tempProjectGenres)
}
