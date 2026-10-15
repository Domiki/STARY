import type { TFunction } from 'i18next'

import type { PixelLayout } from '../../ipc/types'

export function layoutLabel(layout: PixelLayout, t: TFunction): string {
  if (layout.kind !== 'cfa') return t(`layouts.${layout.kind}`)
  return layout.value.kind === 'bayer'
    ? t('layouts.bayer', { pattern: layout.value.value })
    : t('layouts.x_trans')
}
