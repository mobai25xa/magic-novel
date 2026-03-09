import { Folder } from 'lucide-react'

import { Input } from '@/magic-ui/components'
import { useSettingsStore } from '@/state/settings'

import { SettingsButton } from './settings-dialog-button'
import type { SettingsDialogTranslations } from './settings-dialog-types'
import type { TempState } from './use-settings-dialog-controller'

export function renderProjectsContent(
  translations: SettingsDialogTranslations,
  temp: TempState,
  onSelectDirectory: () => Promise<void>,
  resetProjectGenres: () => void,
) {
  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">{translations.settings.projectsRoot}</h3>
        <p className="text-sm text-muted-foreground">{translations.settings.projectsRootDescription}</p>
      </div>

      <div className="flex gap-2 items-center">
        <Input value={temp.tempDir} onChange={(e) => temp.setTempDir(e.target.value)} placeholder={translations.settings.selectRootDirectory} readOnly className="flex-1 max-w-md" />
        <SettingsButton onClick={onSelectDirectory} variant="outline" className="flex items-center gap-1 px-3 py-2 whitespace-nowrap">
          <Folder className="h-4 w-4" />
          {translations.common.browse}
        </SettingsButton>
      </div>

      {temp.tempDir && (
        <p className="text-xs text-muted-foreground">{translations.settings.projectsWillBeSavedIn} {temp.tempDir}/[{translations.home.projectName}]/</p>
      )}

      <div className="settings-section space-y-3">
        <div>
          <h4 className="text-sm font-medium">{translations.projectType.type}</h4>
          <p className="text-xs text-muted-foreground">{translations.settingsExtra.genreManageHint}</p>
        </div>

        <div className="flex gap-2">
          <Input value={temp.newGenre} onChange={(e) => temp.setNewGenre(e.target.value)} placeholder={translations.projectType.addGenre} className="flex-1" />
          <SettingsButton variant="outline" onClick={() => {
            const g = temp.newGenre.trim()
            if (!g) return
            temp.setTempProjectGenres((prev) => (prev.includes(g) ? prev : [...prev, g]))
            temp.setNewGenre('')
          }}>{translations.projectType.addGenre}</SettingsButton>
        </div>

        <div className="flex flex-wrap gap-2">
          {temp.tempProjectGenres.map((g) => (
            <SettingsButton
              key={g}
              type="button"
              variant="outline"
              onClick={() => temp.setTempProjectGenres((prev) => prev.filter((x) => x !== g))}
              className="text-sm"
              title={translations.settingsExtra.clickToDelete}
            >
              {g}
            </SettingsButton>
          ))}
          {temp.tempProjectGenres.length === 0 && <div className="text-xs text-muted-foreground">{translations.settingsExtra.noGenres}</div>}
        </div>

        <div className="flex justify-end">
          <SettingsButton variant="outline" onClick={() => {
            resetProjectGenres()
            temp.setTempProjectGenres(useSettingsStore.getState().projectGenres)
          }}>{translations.projectType.resetGenres}</SettingsButton>
        </div>
      </div>
    </div>
  )
}
