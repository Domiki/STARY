import type { ProbeInfo } from '../../../ipc/types'
import { type Session, type Workspace, slotItems, updateSession } from './workspace'

export const lightsOf = (session: Session): ProbeInfo[] => slotItems(session.slots.light)

/** The representative light of a session: the one picked, or else the first light. */
export const representativeOf = (session: Session): ProbeInfo | null => {
  const lights = lightsOf(session)
  return lights.find(({ path }) => path === session.representativePath) ?? lights[0] ?? null
}

export const isRepresentative = (session: Session, path: string): boolean =>
  representativeOf(session)?.path === path

export function setRepresentative(
  workspace: Workspace,
  sessionId: string,
  path: string,
): Workspace {
  const session = workspace.sessions.find(({ id }) => id === sessionId)
  if (!session || !lightsOf(session).some((light) => light.path === path)) return workspace
  return updateSession(workspace, sessionId, (current) => ({
    ...current,
    representativePath: path,
  }))
}
