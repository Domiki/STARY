import { useTranslation } from 'react-i18next'

import { IconButton } from '../../../shared/IconButton'
import { FitIcon, ZoomInIcon, ZoomOutIcon } from '../../../shared/icons'

interface Props {
  scale: number
  previewWidth: number
  previewHeight: number
  onZoomIn: () => void
  onZoomOut: () => void
  onFit: () => void
}

export function ViewerToolbar({ scale, previewWidth, previewHeight, onZoomIn, onZoomOut, onFit }: Props) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center gap-1 border-b border-twilight px-2 py-1">
      <IconButton label={t('viewer.zoomOut')} onClick={onZoomOut}>
        <ZoomOutIcon />
      </IconButton>
      <span className="w-12 text-center text-xs tabular-nums">{Math.round(scale * 100)}%</span>
      <IconButton label={t('viewer.zoomIn')} onClick={onZoomIn}>
        <ZoomInIcon />
      </IconButton>
      <IconButton label={t('viewer.fit')} onClick={onFit}>
        <FitIcon />
      </IconButton>
      <span className="ml-auto text-xs text-mist">
        {t('viewer.previewOf', { width: previewWidth, height: previewHeight })}
      </span>
    </div>
  )
}
