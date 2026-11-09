import { useTranslation } from 'react-i18next'

import { useSettingsStore } from '../features/settings/store'
import { IconButton } from '../shared/IconButton'
import { GearIcon } from '../shared/icons'
import appIcon from '../assets/app-icon.png'
import wordmark from '../assets/logo-wordmark.png'

export function TopBar() {
  const { t } = useTranslation()
  const setDialogOpen = useSettingsStore((state) => state.setDialogOpen)
  return (
    <header className="flex h-11 shrink-0 items-center justify-between border-b border-twilight px-4">
      <span className="flex items-center gap-2.5">
        <img src={appIcon} alt="" className="h-7 w-7" draggable={false} />
        <img src={wordmark} alt={t('app.name')} className="h-6 w-auto" draggable={false} />
      </span>
      <IconButton label={t('topBar.settings')} onClick={() => setDialogOpen(true)}>
        <GearIcon />
      </IconButton>
    </header>
  )
}
