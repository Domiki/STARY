import { useTranslation } from 'react-i18next'

import { useJobStore } from '../features/operation/job-store'
import { ProgressBar } from '../shared/ProgressBar'

/** Thousandths: a master made in one read with this many pixels full of spoiled values is doubtful. */
const ONE_READ_WARNING = 100

export function JobStatus() {
  const { t } = useTranslation()
  const { phase, progress, outcome, cancel } = useJobStore()

  if (phase === 'running') {
    return (
      <span className="flex items-center gap-3">
        <span>{progress ? t(`job.stage.${progress.stage}`) : t('job.starting')}</span>
        <ProgressBar done={progress?.done ?? 0} total={progress?.total ?? 0} />
        {progress && (
          <span className="tabular-nums">
            {progress.done}/{progress.total}
          </span>
        )}
        <button
          type="button"
          onClick={() => void cancel()}
          className="rounded border border-twilight px-2 text-moon hover:bg-twilight/50"
        >
          {t('job.cancel')}
        </button>
      </span>
    )
  }
  if (!outcome) return null
  if (outcome.status === 'done') {
    const seconds = outcome.seconds.toFixed(1)
    const skipped = outcome.frames !== null && outcome.frames.used < outcome.frames.total
    const text = skipped
      ? t('job.doneSkipped', { seconds, ...outcome.frames })
      : t(outcome.results > 0 ? 'job.doneResult' : 'job.done', { seconds })
    return (
      <span>
        {text}
        {outcome.oneReadFull !== null && outcome.oneReadFull >= ONE_READ_WARNING && (
          <span className="ml-2 text-caution">{t('job.oneReadWarning')}</span>
        )}
        {outcome.device && (
          <span className="ml-2 text-mist/80">
            {outcome.device.kind === 'gpu'
              ? t('job.deviceGpu', { name: outcome.device.name })
              : t('job.deviceCpu')}
          </span>
        )}
      </span>
    )
  }
  if (outcome.status === 'cancelled') return <span>{t('job.cancelled')}</span>
  return (
    <span className="text-danger" title={outcome.message}>
      {t([`job.errors.${outcome.code}`, 'job.errors.other'], { message: outcome.message })}
    </span>
  )
}
