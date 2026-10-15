import type { ProbeInfo, Role } from '../../../ipc/types'
import { ROLES } from '../../../ipc/types'

export type Slot =
  | { kind: 'empty' }
  | { kind: 'frames'; items: ProbeInfo[] }
  | { kind: 'master'; item: ProbeInfo }

export interface Session {
  id: string
  name: string | null
  /** The light the user picked as the representative one; the first light stands in when none. */
  representativePath: string | null
  slots: Record<Role, Slot>
}

export interface Workspace {
  sessions: Session[]
  results: ProbeInfo[]
  activeSessionId: string | null
}

export const emptyWorkspace = (): Workspace => ({
  sessions: [],
  results: [],
  activeSessionId: null,
})

export const emptySlots = (): Record<Role, Slot> => ({
  light: { kind: 'empty' },
  dark: { kind: 'empty' },
  flat: { kind: 'empty' },
  dark_flat: { kind: 'empty' },
  bias: { kind: 'empty' },
})

export const slotItems = (slot: Slot): ProbeInfo[] => {
  switch (slot.kind) {
    case 'empty':
      return []
    case 'frames':
      return slot.items
    case 'master':
      return [slot.item]
  }
}

export const sessionItems = (session: Session): ProbeInfo[] =>
  ROLES.flatMap((role) => slotItems(session.slots[role]))

export const allItems = (workspace: Workspace): ProbeInfo[] => [
  ...workspace.sessions.flatMap(sessionItems),
  ...workspace.results,
]

export const findItem = (workspace: Workspace, path: string | null): ProbeInfo | null =>
  path === null ? null : (allItems(workspace).find((item) => item.path === path) ?? null)

export const updateSession = (
  workspace: Workspace,
  sessionId: string,
  update: (session: Session) => Session,
): Workspace => ({
  ...workspace,
  sessions: workspace.sessions.map((session) =>
    session.id === sessionId ? update(session) : session,
  ),
})
