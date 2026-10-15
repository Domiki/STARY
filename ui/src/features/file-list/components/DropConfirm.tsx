import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '../../../shared/ConfirmDialog'
import { sessionDisplayName } from '../session-name'
import { useFileListStore } from '../store'

export function DropConfirm() {
  const { t } = useTranslation()
  const pending = useFileListStore((state) => state.pendingDrop)
  const sessions = useFileListStore((state) => state.workspace.sessions)
  const { confirmDrop, cancelDrop } = useFileListStore.getState()
  if (!pending) return null

  const index = sessions.findIndex(({ id }) => id === pending.target.sessionId)
  const values = {
    count: pending.paths.length,
    session: sessionDisplayName(sessions[index], index, t),
    role: t(`roles.${pending.target.role}`),
  }

  return (
    <ConfirmDialog
      tone="neutral"
      title={t('dropConfirm.title', values)}
      message={t('dropConfirm.message', values)}
      confirmLabel={t('dropConfirm.confirm')}
      cancelLabel={t('dropConfirm.cancel')}
      onConfirm={confirmDrop}
      onCancel={cancelDrop}
    />
  )
}
