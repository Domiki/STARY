import { useTranslation } from 'react-i18next'

import { openExternal } from '../../../ipc/commands'
import { Modal } from '../../../shared/Modal'
import { useUpdateStore } from '../store'

const dateOf = (iso: string | null) => (iso ? iso.slice(0, 10) : null)

export function UpdateDialog() {
  const { t } = useTranslation()
  const { dialogOpen, release, version, closeDialog } = useUpdateStore()
  if (!dialogOpen || !release) return null

  const download = () => {
    void openExternal(release.url)
    closeDialog()
  }
  const published = dateOf(release.published)

  return (
    <Modal title={t('update.title')} closeLabel={t('update.close')} onClose={closeDialog}>
      <p>{t('update.message', { next: release.version, current: version ?? '' })}</p>
      <div className="space-y-1.5">
        <div className="flex items-baseline justify-between text-xs text-mist">
          <span>{t('update.notes')}</span>
          {published && <span>{published}</span>}
        </div>
        <pre
          tabIndex={0}
          aria-label={t('update.notes')}
          className="max-h-64 overflow-auto whitespace-pre-wrap rounded border border-twilight bg-black/20 p-3 font-[inherit] text-[13px] leading-relaxed text-moon select-text"
        >
          {release.notes.trim() || t('update.noNotes')}
        </pre>
      </div>
      <div className="flex justify-end gap-2">
        <button
          type="button"
          onClick={closeDialog}
          className="rounded border border-twilight px-3 py-1 hover:bg-twilight/50"
        >
          {t('update.close')}
        </button>
        <button
          type="button"
          autoFocus
          onClick={download}
          className="rounded border border-ember/70 bg-ember/15 px-3 py-1 text-ember hover:bg-ember/25"
        >
          {t('update.download')}
        </button>
      </div>
    </Modal>
  )
}
