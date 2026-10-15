import { useTranslation } from 'react-i18next'

import type { ProbeInfo } from '../../../ipc/types'
import { CollapsibleSection } from '../../../shared/CollapsibleSection'
import {
  fileNameOf,
  formatBytes,
  formatCapturedAt,
  formatDimensions,
  formatExposure,
} from '../../../shared/format'
import { layoutLabel } from '../../file-list/layout-label'
import { findItem } from '../../file-list/model/workspace'
import { useFileListStore } from '../../file-list/store'

type Row = [label: string, value: string | null]

function rowsOf(info: ProbeInfo, t: ReturnType<typeof useTranslation>['t']): Row[] {
  return [
    [t('properties.file'), fileNameOf(info.path)],
    [t('properties.path'), info.path],
    [t('properties.format'), t(`formats.${info.format}`)],
    [t('properties.size'), formatBytes(info.fingerprint.size_bytes)],
    [t('properties.dimensions'), formatDimensions(info.width, info.height)],
    [t('properties.layout'), layoutLabel(info.layout, t)],
    [t('properties.bitDepth'), info.bit_depth ? t('properties.bits', { count: info.bit_depth }) : null],
    [t('properties.camera'), info.camera],
    [t('properties.exposure'), info.exposure_s ? formatExposure(info.exposure_s) : null],
    [t('properties.iso'), info.iso ? String(info.iso) : null],
    [t('properties.capturedAt'), info.captured_at ? formatCapturedAt(info.captured_at) : null],
    [t('properties.kind'), info.stary_kind ? t(`kinds.${info.stary_kind}`) : null],
    [t('properties.stackCount'), info.stack_count ? String(info.stack_count) : null],
    [t('properties.calibrated'), info.calibrated ? t('properties.yes') : null],
  ]
}

export function PropertiesSection() {
  const { t } = useTranslation()
  const info = useFileListStore((state) => findItem(state.workspace, state.selection.primary))
  const rows = info ? rowsOf(info, t) : []

  return (
    <CollapsibleSection title={t('properties.title')}>
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 px-3 pb-2 text-xs">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-mist">{label}</dt>
            <dd className="break-all select-text">{value ?? t('properties.none')}</dd>
          </div>
        ))}
      </dl>
    </CollapsibleSection>
  )
}
