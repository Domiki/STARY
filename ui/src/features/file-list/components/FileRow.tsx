import { useTranslation } from 'react-i18next'

import type { ProbeInfo } from '../../../ipc/types'
import { Badge } from '../../../shared/Badge'
import { IconButton } from '../../../shared/IconButton'
import { CloseIcon, RepresentativeIcon, WarningIcon } from '../../../shared/icons'
import { fileNameOf } from '../../../shared/format'
import { useJobStore } from '../../operation/job-store'
import type { ItemWarning } from '../model/warnings'
import { useFileListStore } from '../store'
import { useRowHandlers } from './useRowHandlers'

interface Props {
  info: ProbeInfo
  isMaster?: boolean
  isRepresentative?: boolean
  warnings?: ItemWarning[]
}

export function FileRow({ info, isMaster, isRepresentative, warnings = [] }: Props) {
  const { selected, onClick, onContextMenu, onRemove } = useRowHandlers(info.path)
  const { t } = useTranslation()
  const locked = useFileListStore((state) => state.locked)
  const leftOut = useJobStore((state) => state.leftOut[info.path])
  const warningText = warnings
    .map(({ key, actual, expected }) => t(`warnings.${key}`, { actual, expected }))
    .join('\n')
  return (
    <div
      role="option"
      aria-selected={selected}
      tabIndex={0}
      onClick={onClick}
      onContextMenu={onContextMenu}
      className={`group flex h-6 cursor-pointer items-center gap-2 border-l-2 pr-1 pl-6 text-[13px] ${
        selected
          ? 'border-ember bg-twilight/60'
          : 'border-transparent hover:bg-twilight/30'
      }`}
    >
      <span className="min-w-0 flex-1 truncate" title={info.path}>
        {fileNameOf(info.path)}
      </span>
      {isRepresentative && (
        <span
          role="img"
          aria-label={t('fileList.representative')}
          title={t('fileList.representative')}
          className="text-ember"
        >
          <RepresentativeIcon width={12} height={12} />
        </span>
      )}
      {leftOut && (
        <span title={t(`fileList.leftOutReason.${leftOut}`)}>
          <Badge tone="caution">{t('fileList.leftOut')}</Badge>
        </span>
      )}
      {isMaster && <Badge tone="ember">{t('fileList.master')}</Badge>}
      {info.calibrated && <Badge>{t('fileList.calibrated')}</Badge>}
      {warnings.length > 0 && (
        <span title={warningText} className="text-caution">
          <WarningIcon />
        </span>
      )}
      {(
        <IconButton
          label={t('fileList.removeItem')}
          disabled={locked}
          className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          onClick={(event) => {
            event.stopPropagation()
            onRemove()
          }}
        >
          <CloseIcon />
        </IconButton>
      )}
    </div>
  )
}
