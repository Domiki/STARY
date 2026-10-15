import { useTranslation } from 'react-i18next'

import { useSettingsStore } from '../features/settings/store'
import { IconButton } from '../shared/IconButton'
import { GearIcon, StarIcon } from '../shared/icons'

export function TopBar() {
  const { t } = useTranslation()
  const setDialogOpen = useSettingsStore((state) => state.setDialogOpen)
  return (
    <header className="flex h-11 shrink-0 items-center justify-between border-b border-twilight px-4">
      <span className="flex items-center gap-2 text-lg tracking-[0.25em]">
        <StarIcon className="text-ember" width={14} height={14} />
        {t('app.name')}
      </span>
      <IconButton label={t('topBar.settings')} onClick={() => setDialogOpen(true)}>
        <GearIcon />
      </IconButton>
    </header>
  )
}
