import type { Role } from '../../ipc/types'
import type { Session } from '../file-list/model/workspace'

export type CalibrationRole = Exclude<Role, 'light'>

export const CALIBRATION_ROLES: CalibrationRole[] = ['bias', 'dark', 'flat', 'dark_flat']

export type RoleStatus =
  | { kind: 'none' }
  | { kind: 'master' }
  | { kind: 'frames'; count: number }

export function roleStatus(session: Session | undefined, role: CalibrationRole): RoleStatus {
  const slot = session?.slots[role]
  if (!slot || slot.kind === 'empty') return { kind: 'none' }
  return slot.kind === 'master' ? { kind: 'master' } : { kind: 'frames', count: slot.items.length }
}

export const hasCalibrationData = (session: Session | undefined): boolean =>
  CALIBRATION_ROLES.some((role) => roleStatus(session, role).kind !== 'none')

export const hasFramesToIntegrate = (session: Session | undefined): boolean =>
  CALIBRATION_ROLES.some((role) => roleStatus(session, role).kind === 'frames')

export const hasLights = (session: Session | undefined): boolean =>
  session !== undefined && session.slots.light.kind === 'frames'

export const lightCount = (session: Session | undefined): number =>
  session?.slots.light.kind === 'frames' ? session.slots.light.items.length : 0

const BYTES_PER_CFA_SAMPLE = 2

export function calibratedLightsBytes(session: Session | undefined): number | null {
  const slot = session?.slots.light
  if (!slot || slot.kind !== 'frames') return null
  return slot.items.reduce(
    (total, item) => total + item.width * item.height * BYTES_PER_CFA_SAMPLE,
    0,
  )
}
