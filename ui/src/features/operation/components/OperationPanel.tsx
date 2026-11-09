import { useTranslation } from 'react-i18next'

import { useFileListStore } from '../../file-list/store'
import { useJobStore } from '../job-store'
import { CalibrationSection } from './CalibrationSection'
import { ModeSection } from './ModeSection'
import { RunBar } from './RunBar'

export function OperationPanel() {
  const { t } = useTranslation()
  const running = useJobStore((state) => state.phase === 'running')
  const addingFiles = useFileListStore(
    (state) => state.importProgress !== null || state.scanning > 0 || state.pendingDrop !== null,
  )
  const locked = addingFiles || running
  return (
    <div className="flex h-full min-h-0 flex-col bg-twilight/10">
      <header className="border-b border-twilight px-3 py-2">
        <h2>{t('operation.title')}</h2>
      </header>
      <div
        inert={locked}
        className={`flex min-h-0 flex-1 flex-col transition-opacity ${locked ? 'opacity-50' : ''}`}
      >
        <div className="min-h-0 flex-1 overflow-auto">
          <CalibrationSection />
          <ModeSection />
        </div>
        <RunBar />
      </div>
    </div>
  )
}
