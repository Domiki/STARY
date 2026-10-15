import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '../../../shared/ConfirmDialog'
import { sessionDisplayName } from '../session-name'
import { useFileListStore } from '../store'

export function RemovalConfirm() {
  const { t } = useTranslation()
  const pending = useFileListStore((state) => state.pendingRemoval)
  const sessions = useFileListStore((state) => state.workspace.sessions)
  const { confirmRemoval, cancelRemoval } = useFileListStore.getState()
  if (!pending) return null

  const sessionIndex =
    pending.kind === 'session'
      ? sessions.findIndex(({ id }) => id === pending.sessionId)
      : -1
  const sessionName =
    pending.kind === 'session' ? sessionDisplayName(sessions[sessionIndex], sessionIndex, t) : ''

  return (
    <ConfirmDialog
      title={
        pending.kind === 'session'
          ? t('confirm.sessionTitle', { name: sessionName })
          : t('confirm.filesTitle', { count: pending.paths.length })
      }
      message={
        pending.kind === 'session'
          ? t('confirm.sessionMessage', { count: pending.fileCount })
          : t('confirm.filesMessage')
      }
      note={t('confirm.note')}
      confirmLabel={t('confirm.confirm')}
      cancelLabel={t('confirm.cancel')}
      onConfirm={confirmRemoval}
      onCancel={cancelRemoval}
    />
  )
}
