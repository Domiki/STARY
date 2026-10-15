import type { PixelLayout, ProbeInfo, Role, StaryKind } from '../../../ipc/types'
import {
  type Session,
  type Slot,
  type Workspace,
  emptySlots,
  sessionItems,
  slotItems,
  updateSession,
} from './workspace'

export type RejectionReason =
  | 'duplicate'
  | 'wrong_role'
  | 'slot_holds_master'
  | 'slot_holds_frames'
  | 'incompatible_size'
  | 'incompatible_layout'
  | 'read_failed'
  | 'unsupported_file'
  | 'decode_failed'
  | 'internal'

export interface Rejection {
  path: string
  reason: RejectionReason
  role?: Role
  expected?: Role
}

export interface Target {
  sessionId: string
  role: Role
}

export interface Placement {
  workspace: Workspace
  rejection: Rejection | null
}

const MASTER_ROLES: Partial<Record<StaryKind, Role>> = {
  master_bias: 'bias',
  master_dark: 'dark',
  master_flat: 'flat',
  master_dark_flat: 'dark_flat',
}

export const masterRoleOf = (kind: StaryKind | null): Role | null =>
  kind === null ? null : (MASTER_ROLES[kind] ?? null)

let sessionCounter = 0
export const newSessionId = () => `session-${++sessionCounter}`

export const createSession = (id = newSessionId()): Session => ({
  id,
  name: null,
  representativePath: null,
  slots: emptySlots(),
})

export function initialWorkspace(): Workspace {
  const session = createSession()
  return { sessions: [session], results: [], activeSessionId: session.id }
}

export function ensureActiveSession(workspace: Workspace): Workspace {
  if (workspace.sessions.some((session) => session.id === workspace.activeSessionId)) {
    return workspace
  }
  const first = workspace.sessions[0]
  if (first) return { ...workspace, activeSessionId: first.id }
  const session = createSession()
  return { ...workspace, sessions: [session], activeSessionId: session.id }
}

const sameLayout = (a: PixelLayout, b: PixelLayout) => JSON.stringify(a) === JSON.stringify(b)

function compatibilityRejection(session: Session, info: ProbeInfo): Rejection | null {
  const reference = sessionItems(session)[0]
  if (!reference) return null
  if (reference.width !== info.width || reference.height !== info.height) {
    return { path: info.path, reason: 'incompatible_size' }
  }
  return sameLayout(reference.layout, info.layout)
    ? null
    : { path: info.path, reason: 'incompatible_layout' }
}

const reject = (workspace: Workspace, rejection: Rejection): Placement => ({
  workspace,
  rejection,
})

function slotRejection(slot: Slot, info: ProbeInfo, role: Role, asMaster: boolean) {
  if (slot.kind === 'master') return { path: info.path, reason: 'slot_holds_master', role } as const
  if (asMaster && slot.kind === 'frames') {
    return { path: info.path, reason: 'slot_holds_frames', role } as const
  }
  return null
}

function placeInSlot(workspace: Workspace, info: ProbeInfo, target: Target, asMaster: boolean) {
  const session = workspace.sessions.find(({ id }) => id === target.sessionId)
  if (!session) return reject(workspace, { path: info.path, reason: 'internal' })
  const slot = session.slots[target.role]
  const rejection =
    (sessionItems(session).some((item) => item.path === info.path)
      ? ({ path: info.path, reason: 'duplicate' } as const)
      : null) ??
    slotRejection(slot, info, target.role, asMaster) ??
    compatibilityRejection(session, info)
  if (rejection) return reject(workspace, rejection)
  const nextSlot: Slot = asMaster
    ? { kind: 'master', item: info }
    : { kind: 'frames', items: [...slotItems(slot), info] }
  return {
    workspace: updateSession(workspace, session.id, (current) => ({
      ...current,
      slots: { ...current.slots, [target.role]: nextSlot },
    })),
    rejection: null,
  }
}

export function placeResult(workspace: Workspace, info: ProbeInfo): Placement {
  return workspace.results.some((item) => item.path === info.path)
    ? reject(workspace, { path: info.path, reason: 'duplicate' })
    : { workspace: { ...workspace, results: [...workspace.results, info] }, rejection: null }
}

export function placeInfo(workspace: Workspace, info: ProbeInfo, explicit?: Target): Placement {
  if (info.stary_kind === 'stack_result') return placeResult(workspace, info)
  const prepared = ensureActiveSession(workspace)
  const sessionId = explicit?.sessionId ?? prepared.activeSessionId
  const masterRole = masterRoleOf(info.stary_kind)
  if (sessionId === null) return reject(prepared, { path: info.path, reason: 'internal' })
  if (masterRole !== null) {
    return explicit && explicit.role !== masterRole
      ? reject(prepared, { path: info.path, reason: 'wrong_role', expected: masterRole })
      : placeInSlot(prepared, info, { sessionId, role: masterRole }, true)
  }
  return placeInSlot(prepared, info, { sessionId, role: explicit?.role ?? 'light' }, false)
}
