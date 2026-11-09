import type { ProbeInfo } from '../../ipc/types'
import { type Session, type Workspace, slotItems } from '../file-list/model/workspace'
import { hasLights } from './calibration-status'
import type { Mode } from './store'

export const activeSession = (workspace: Workspace): Session | undefined =>
  workspace.sessions.find(({ id }) => id === workspace.activeSessionId)

/** The session the stacks line up on: the one chosen (when it has lights), else the active one. */
export function referenceSessionOf(
  workspace: Workspace,
  chosenId: string | null,
): Session | undefined {
  const chosen = workspace.sessions.find(({ id }) => id === chosenId)
  return chosen && hasLights(chosen) ? chosen : activeSession(workspace)
}

/** The session a sky mask is made for: the reference session when stacking. */
export const maskSessionOf = (
  workspace: Workspace,
  mode: Mode,
  chosenId: string | null,
): Session | undefined =>
  mode === 'stacking' ? referenceSessionOf(workspace, chosenId) : activeSession(workspace)

const sameSensor = (a: ProbeInfo, b: ProbeInfo) =>
  a.width === b.width && a.height === b.height && JSON.stringify(a.layout) === JSON.stringify(b.layout)

/** The other sessions that are stacked with the reference one: those whose frames fit its picture. */
export function otherSessionsToStack(workspace: Workspace, reference: Session): Session[] {
  const first = slotItems(reference.slots.light)[0]
  if (!first) return []
  return workspace.sessions.filter(
    (session) =>
      session.id !== reference.id &&
      hasLights(session) &&
      sameSensor(first, slotItems(session.slots.light)[0]!),
  )
}

const exposureKey = (session: Session): string | null => {
  const keys = new Set(slotItems(session.slots.light).map((light) => `${light.exposure_s}|${light.iso}`))
  const [key] = keys
  return keys.size === 1 && key !== undefined && !key.startsWith('null') ? key : null
}

/**
 * Whether the sessions hold lights of more than one exposure, each session of a single one: only
 * then can the exposures be stacked apart and joined.
 */
export function joinableExposures(sessions: Session[]): boolean {
  const keys = sessions.map(exposureKey)
  return keys.every((key) => key !== null) && new Set(keys).size >= 2
}

/** The sessions a stacking job takes: the reference session and those stacked with it. */
export const sessionsToStack = (workspace: Workspace, chosenId: string | null): Session[] => {
  const reference = referenceSessionOf(workspace, chosenId)
  return reference ? [reference, ...otherSessionsToStack(workspace, reference)] : []
}
