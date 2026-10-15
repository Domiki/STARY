import { ROLES, type Role } from '../../ipc/types'
import type { Target } from './model/placement'

export const DROP_ROLE_ATTRIBUTE = 'data-drop-role'
export const DROP_SESSION_ATTRIBUTE = 'data-drop-session'

const isRole = (value: string | undefined): value is Role =>
  ROLES.some((role) => role === value)

export function dropTargetOf(element: Element | null): Target | null {
  const zone = element?.closest<HTMLElement>(`[${DROP_ROLE_ATTRIBUTE}]`)
  const role = zone?.dataset.dropRole
  const sessionId = zone?.dataset.dropSession
  return sessionId && isRole(role) ? { sessionId, role } : null
}

export const dropTargetAt = (x: number, y: number): Target | null =>
  dropTargetOf(document.elementFromPoint(x, y))
