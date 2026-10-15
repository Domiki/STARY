import { ROLES, type Role } from '../../../ipc/types'
import { type Workspace, slotItems } from './workspace'

export type Location = { kind: 'session'; sessionId: string; role: Role } | { kind: 'result' }

export function locate(workspace: Workspace, path: string): Location | null {
  for (const session of workspace.sessions) {
    for (const role of ROLES) {
      if (slotItems(session.slots[role]).some((item) => item.path === path)) {
        return { kind: 'session', sessionId: session.id, role }
      }
    }
  }
  return workspace.results.some((item) => item.path === path) ? { kind: 'result' } : null
}
