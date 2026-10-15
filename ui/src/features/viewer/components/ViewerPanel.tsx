import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { useFileListStore } from '../../file-list/store'
import { MaskOverlay } from '../../mask/components/MaskOverlay'
import { MaskToolbar } from '../../mask/components/MaskToolbar'
import { useJobStore } from '../../operation/job-store'
import { useMaskTarget } from '../../mask/useMaskTarget'
import { useViewerStore } from '../store'
import { ViewerToolbar } from './ViewerToolbar'
import { useViewport } from './useViewport'

const PIXELATED_FROM_SCALE = 2

export function ViewerPanel() {
  const { t } = useTranslation()
  const selectedPath = useFileListStore((state) => state.selection.primary)
  const { phase, view, show } = useViewerStore()
  const width = view?.width ?? 0
  const height = view?.height ?? 0
  const viewport = useViewport(width, height)
  const maskTarget = useMaskTarget()
  const running = useJobStore((state) => state.phase === 'running')

  useEffect(() => void show(selectedPath), [selectedPath, show])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ViewerToolbar
        scale={viewport.scale}
        previewWidth={width}
        previewHeight={height}
        onZoomIn={viewport.zoomIn}
        onZoomOut={viewport.zoomOut}
        onFit={viewport.reset}
      />
      {maskTarget && phase === 'ready' && (
        <div inert={running} className={running ? 'opacity-50' : ''}>
          <MaskToolbar {...maskTarget} />
        </div>
      )}
      <div
        ref={viewport.containerRef}
        className="relative min-h-0 flex-1 cursor-grab overflow-hidden bg-black/30 active:cursor-grabbing"
        {...viewport.handlers}
      >
        {phase === 'ready' && view && (
          <img
            src={view.url}
            alt=""
            draggable={false}
            width={width}
            height={height}
            className="pointer-events-none absolute top-1/2 left-1/2 max-w-none"
            style={{
              marginLeft: -width / 2,
              marginTop: -height / 2,
              transform: `translate(${viewport.panX}px, ${viewport.panY}px) scale(${viewport.scale})`,
              imageRendering: viewport.scale >= PIXELATED_FROM_SCALE ? 'pixelated' : 'auto',
            }}
          />
        )}
        {maskTarget && phase === 'ready' && view && (
          <div inert={running} className="contents">
            <MaskOverlay
              {...maskTarget}
              width={width}
              height={height}
              viewport={viewport}
              containerRef={viewport.containerRef}
            />
          </div>
        )}
        {phase === 'loading' && (
          <p className="absolute inset-0 flex items-center justify-center text-mist">
            {t('viewer.loading')}
          </p>
        )}
        {phase === 'error' && (
          <p className="absolute inset-0 flex items-center justify-center text-danger">
            {t('viewer.error')}
          </p>
        )}
      </div>
    </div>
  )
}
