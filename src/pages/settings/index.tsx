import { SettingsPage as SettingsFullPage } from '@/components/settings/SettingsFullPage'

interface SettingsPageProps {
  open?: boolean
  onClose?: () => void
}

export function SettingsPage(_props: SettingsPageProps) {
  return <SettingsFullPage />
}
