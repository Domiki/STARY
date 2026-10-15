import { useTranslation } from 'react-i18next'

import { useFileListStore } from '../../file-list/store'
import { canRun } from '../job-request'
import { useJobStore } from '../job-store'
import { useOperationStore } from '../store'
import { useOperationChoices } from '../useOperationChoices'

export function RunBar() {
  const { t } = useTranslation()
  const mode = useOperationStore((state) => state.mode)
  const workspace = useFileListStore((state) => state.workspace)
  const choices = useOperationChoices()
  const running = useJobStore((state) => state.phase === 'running')
  const start = useJobStore((state) => state.start)
  const runnable = !running && canRun(mode, mode, workspace, choices)

  return (
    <div className="border-t border-twilight bg-midnight px-3 py-2">
      <button
        type="button"
        disabled={!runnable}
        onClick={() => void start(mode)}
        className="w-full rounded bg-ember/90 py-1.5 text-midnight disabled:opacity-35"
      >
        {t('operation.mode.run')}
      </button>
    </div>
  )
}
