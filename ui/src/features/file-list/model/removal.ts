import { ROLES, type Role } from '../../../ipc/types'
import { type Slot, type Workspace, emptySlots, updateSession } from './workspace'

const withoutPath = (slot: Slot, path: string): Slot => {
  if (slot.kind === 'master') return slot.item.path === path ? { kind: 'empty' } : slot
  if (slot.kind === 'frames') {
    const items = slot.items.filter((item) => item.path !== path)
    return items.length ? { kind: 'frames', items } : { kind: 'empty' }
  }
  return slot
}

export const removeFromSession = (
  workspace: Workspace,
  sessionId: string,
  role: Role,
  path: string,
): Workspace =>
  updateSession(workspace, sessionId, (session) => ({
    ...session,
    representativePath: session.representativePath === path ? null : session.representativePath,
    slots: { ...session.slots, [role]: withoutPath(session.slots[role], path) },
  }))

export function removeSession(workspace: Workspace, sessionId: string): Workspace {
  const sessions = workspace.sessions.filter((session) => session.id !== sessionId)
  const activeStillExists = sessions.some((session) => session.id === workspace.activeSessionId)
  return {
    ...workspace,
    sessions,
    activeSessionId: activeStillExists ? workspace.activeSessionId : (sessions[0]?.id ?? null),
  }
}

export function removePaths(workspace: Workspace, paths: string[]): Workspace {
  const gone = new Set(paths)
  return {
    ...workspace,
    sessions: workspace.sessions.map((session) => ({
      ...session,
      representativePath:
        session.representativePath !== null && gone.has(session.representativePath)
          ? null
          : session.representativePath,
      slots: ROLES.reduce(
        (slots, role) => ({
          ...slots,
          [role]: [...gone].reduce((slot, path) => withoutPath(slot, path), session.slots[role]),
        }),
        emptySlots(),
      ),
    })),
    results: workspace.results.filter((item) => !gone.has(item.path)),
  }
}
