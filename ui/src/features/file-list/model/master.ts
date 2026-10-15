import type { Role } from '../../../ipc/types'
import type { Session, Workspace } from './workspace'
import { updateSession } from './workspace'

export type MasterAvailability = 'ok' | 'light_role' | 'other_files' | 'already_master' | 'missing'

export function masterAvailability(session: Session, role: Role, path: string): MasterAvailability {
  const slot = session.slots[role]
  if (role === 'light') return 'light_role'
  if (slot.kind === 'master') return 'already_master'
  if (slot.kind === 'empty') return 'missing'
  if (!slot.items.some((item) => item.path === path)) return 'missing'
  return slot.items.length === 1 ? 'ok' : 'other_files'
}

export function setMaster(
  workspace: Workspace,
  sessionId: string,
  role: Role,
  path: string,
): { workspace: Workspace; availability: MasterAvailability } {
  const session = workspace.sessions.find(({ id }) => id === sessionId)
  const availability = session ? masterAvailability(session, role, path) : 'missing'
  if (availability !== 'ok') return { workspace, availability }
  return {
    availability,
    workspace: updateSession(workspace, sessionId, (current) => {
      const slot = current.slots[role]
      return slot.kind === 'frames'
        ? { ...current, slots: { ...current.slots, [role]: { kind: 'master', item: slot.items[0]! } } }
        : current
    }),
  }
}

export const clearMaster = (workspace: Workspace, sessionId: string, role: Role): Workspace =>
  updateSession(workspace, sessionId, (session) => {
    const slot = session.slots[role]
    return slot.kind === 'master'
      ? { ...session, slots: { ...session.slots, [role]: { kind: 'frames', items: [slot.item] } } }
      : session
  })
