import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { listenForCloseRequest, quitApp } from '../ipc/commands'
import { ConfirmDialog } from '../shared/ConfirmDialog'

/** Asks before the program is closed while a job is running, since the job would be lost. */
export function CloseGuard() {
  const { t } = useTranslation()
  const [asking, setAsking] = useState(false)

  useEffect(() => {
    const stop = listenForCloseRequest(() => setAsking(true))
    return () => void stop.then((unlisten) => unlisten())
  }, [])

  if (!asking) return null
  return (
    <ConfirmDialog
      title={t('closeGuard.title')}
      message={t('closeGuard.message')}
      confirmLabel={t('closeGuard.quit')}
      cancelLabel={t('closeGuard.stay')}
      onConfirm={() => void quitApp()}
      onCancel={() => setAsking(false)}
    />
  )
}
