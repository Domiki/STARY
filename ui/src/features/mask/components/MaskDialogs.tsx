import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '../../../shared/ConfirmDialog'
import { useJobStore } from '../../operation/job-store'
import { useMaskStore } from '../store'

/** Asks before the sky is found again over a mask, and before a run with too few stars in the sky. */
export function MaskDialogs() {
  const { t } = useTranslation()
  const confirming = useMaskStore((state) => state.confirming)
  const warning = useJobStore((state) => state.skyWarning)
  return (
    <>
      {confirming && (
        <ConfirmDialog
          tone="neutral"
          title={t('mask.confirmTitle')}
          message={t('mask.confirmMessage')}
          confirmLabel={t('mask.detect')}
          cancelLabel={t('mask.cancel')}
          onConfirm={() => void useMaskStore.getState().detect(confirming)}
          onCancel={() => useMaskStore.getState().cancelConfirm()}
        />
      )}
      {warning && (
        <ConfirmDialog
          tone="neutral"
          title={t('mask.fewStarsTitle')}
          message={t('mask.fewStarsMessage', { count: warning.stars })}
          confirmLabel={t('mask.runAnyway')}
          cancelLabel={t('mask.cancel')}
          onConfirm={() => void useJobStore.getState().start(warning.kind, true)}
          onCancel={() => useJobStore.getState().dismissSkyWarning()}
        />
      )}
    </>
  )
}
