import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Histogram } from '../../../ipc/types'
import { CollapsibleSection } from '../../../shared/CollapsibleSection'
import { CHART_HEIGHT, CHART_WIDTH, areaPath, histogramPeak } from '../histogram-path'

const CHANNELS = [
  { key: 'red', color: '#d2979c' },
  { key: 'green', color: '#9ec2a7' },
  { key: 'blue', color: '#93a3c2' },
] as const

const percent = (fraction: number) => `${(fraction * 100).toFixed(2)}%`

export function HistogramSection({ histogram }: { histogram: Histogram | null }) {
  const { t } = useTranslation()
  const [log, setLog] = useState(false)
  const peak = histogram ? histogramPeak([histogram.red, histogram.green, histogram.blue]) : 0

  return (
    <CollapsibleSection
      title={t('inspector.histogram')}
      actions={
        <label className="flex cursor-pointer items-center gap-1 text-xs text-mist">
          <input type="checkbox" checked={log} onChange={(event) => setLog(event.target.checked)} />
          {t('inspector.logScale')}
        </label>
      }
    >
      <div className="px-3 pb-2">
        <svg
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          preserveAspectRatio="none"
          className="h-24 w-full rounded bg-black/30"
          role="img"
          aria-label={t('inspector.histogram')}
        >
          {histogram &&
            CHANNELS.map(({ key, color }) => (
              <path
                key={key}
                d={areaPath(histogram[key], peak, log)}
                fill={color}
                fillOpacity={0.35}
                stroke={color}
                strokeWidth={0.6}
                vectorEffect="non-scaling-stroke"
              />
            ))}
        </svg>
        <div className="mt-1 flex justify-between text-[11px] text-mist">
          <span>
            {t('inspector.clippedLow')} {histogram ? percent(histogram.clipped_low) : ''}
          </span>
          <span>
            {t('inspector.clippedHigh')} {histogram ? percent(histogram.clipped_high) : ''}
          </span>
        </div>
      </div>
    </CollapsibleSection>
  )
}
