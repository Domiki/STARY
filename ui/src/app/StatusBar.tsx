import { useTranslation } from 'react-i18next'

import { allItems } from '../features/file-list/model/workspace'
import { useExportStore } from '../features/export/store'
import { useFileListStore } from '../features/file-list/store'
import { useJobStore } from '../features/operation/job-store'
import { useSettingsStore } from '../features/settings/store'
import { fileNameOf } from '../shared/format'
import { ProgressBar } from '../shared/ProgressBar'
import { JobStatus } from './JobStatus'

export function StatusBar() {
  const { t } = useTranslation()
  const status = useFileListStore((state) => state.status)
  const importing = useFileListStore((state) => state.importProgress)
  const scanning = useFileListStore((state) => state.scanning > 0)
  const jobActive = useJobStore((state) => state.phase === 'running' || state.outcome !== null)
  const device = useSettingsStore((state) => state.device)
  const count = useFileListStore((state) => allItems(state.workspace).length)
  const saved = useExportStore((state) => state.saved)
  const message = saved
      ? t('export.saved', { name: fileNameOf(saved) })
      : status && status.rejected > 0
      ? `${t('status.added', { count: status.added })} · ${t('status.rejected', { count: status.rejected })}`
      : status
        ? t('status.added', { count: status.added })
        : t('status.ready')
  return (
    <footer className="flex h-7 shrink-0 items-center justify-between border-t border-twilight px-4 text-xs text-mist">
      {importing ? (
        <span className="flex items-center gap-3">
          <span>{t('status.importing')}</span>
          <ProgressBar done={importing.done} total={importing.total} />
          <span className="tabular-nums">
            {importing.done}/{importing.total}
          </span>
        </span>
      ) : scanning ? (
        <span>{t('status.scanning')}</span>
      ) : jobActive ? (
        <JobStatus />
      ) : (
        <span>{message}</span>
      )}
      <span className="flex gap-4">
        <span>{t('status.device', { device: t(`operation.general.device.${device}`) })}</span>
        <span>{t('status.items', { count })}</span>
      </span>
    </footer>
  )
}
