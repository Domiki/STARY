import type { ProbeInfo, Role } from '../../../ipc/types'
import { type Session, slotItems } from './workspace'

export type WarningKey = 'exposure_differs' | 'iso_differs' | 'format_mixed' | 'low_accuracy'

export interface ItemWarning {
  key: WarningKey
  actual?: string
  expected?: string
}

type Comparison = 'exposure' | 'iso'

const COMPARED_WITH_LIGHT: Record<Role, Comparison[]> = {
  light: [],
  dark: ['exposure', 'iso'],
  flat: ['iso'],
  dark_flat: [],
  bias: ['iso'],
}

const formatExposure = (seconds: number) => `${Number(seconds.toFixed(3))}s`

const differs = (a: number | null, b: number | null) =>
  a !== null && b !== null && Math.abs(a - b) > 1e-6

function compare(item: ProbeInfo, reference: ProbeInfo, what: Comparison): ItemWarning[] {
  if (what === 'exposure') {
    return differs(item.exposure_s, reference.exposure_s)
      ? [
          {
            key: 'exposure_differs',
            actual: formatExposure(item.exposure_s ?? 0),
            expected: formatExposure(reference.exposure_s ?? 0),
          },
        ]
      : []
  }
  return differs(item.iso, reference.iso)
    ? [{ key: 'iso_differs', actual: String(item.iso), expected: String(reference.iso) }]
    : []
}

function warningsForItem(
  session: Session,
  role: Role,
  item: ProbeInfo,
  isMaster: boolean,
): ItemWarning[] {
  const slotFirst = slotItems(session.slots[role])[0]
  const light = slotItems(session.slots.light)[0]
  const flat = slotItems(session.slots.flat)[0]
  const sessionFirst = light ?? flat ?? slotFirst
  const withinSlot =
    !isMaster && slotFirst && slotFirst !== item
      ? (['exposure', 'iso'] as Comparison[]).flatMap((what) => compare(item, slotFirst, what))
      : []
  const lightReference = role === 'dark_flat' ? flat : light
  const againstLight =
    lightReference && lightReference !== item
      ? (role === 'dark_flat' ? (['exposure', 'iso'] as Comparison[]) : COMPARED_WITH_LIGHT[role])
          .flatMap((what) => compare(item, lightReference, what))
      : []
  const mixed =
    sessionFirst && sessionFirst.format !== item.format
      ? [{ key: 'format_mixed' } as ItemWarning]
      : []
  const lowAccuracy =
    role !== 'light' && (item.format === 'jpeg' || item.format === 'png')
      ? [{ key: 'low_accuracy' } as ItemWarning]
      : []
  return [...withinSlot, ...againstLight, ...mixed, ...lowAccuracy]
}

export function warningsOf(session: Session): Map<string, ItemWarning[]> {
  const entries = (Object.keys(session.slots) as Role[]).flatMap((role) => {
    const slot = session.slots[role]
    return slotItems(slot).map(
      (item) => [item.path, warningsForItem(session, role, item, slot.kind === 'master')] as const,
    )
  })
  return new Map(entries.filter(([, warnings]) => warnings.length > 0))
}
