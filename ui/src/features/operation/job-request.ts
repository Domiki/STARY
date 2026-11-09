import type {
  ComputeDevice,
  JobKind,
  JobRequest,
  JobSlot,
  ProbeInfo,
  ReadMode,
  Role,
  StackingChoices,
  TrailsRequest,
} from '../../ipc/types'
import { representativeOf } from '../file-list/model/representative'
import { type Session, type Slot, type Workspace, slotItems } from '../file-list/model/workspace'
import {
  hasCalibrationData,
  hasFramesToIntegrate,
  hasLights,
  lightCount,
} from './calibration-status'
import { defaultOutputFolder } from './output-folder'
import {
  activeSession,
  joinableExposures,
  otherSessionsToStack,
  referenceSessionOf,
} from './reference-session'
import type { Mode } from './store'

export interface GroundChoices {
  masking: boolean
  separate: boolean
  /** The active session has a mask that was applied. */
  maskReady: boolean
}

export interface OperationChoices {
  applyCalibration: boolean
  saveMasters: boolean
  saveCalibrated: boolean
  folders: { master: string | null; calibrated: string | null; result: string | null }
  quality: JobRequest['quality']
  trails: TrailsRequest
  stacking: StackingChoices
  memoryPercent: number
  device: ComputeDevice
  read: ReadMode
  ground: GroundChoices
  /** The session the stacks line up on; `null` is the active one. */
  referenceSession: string | null
  /** Stack the exposures apart and join them (when the sessions hold more than one). */
  hdr: boolean
}

const MIN_STACK_FRAMES = 2

const pathsOf = (items: ProbeInfo[]) => items.map((item) => item.path)

function toJobSlot(slot: Slot): JobSlot {
  switch (slot.kind) {
    case 'empty':
      return { kind: 'none' }
    case 'master':
      return { kind: 'master', path: slot.item.path }
    case 'frames':
      return { kind: 'frames', paths: pathsOf(slotItems(slot)) }
  }
}

/** Where the representative image stands among the lights; the stacks line up on it. */
export const referenceIndexOf = (session: Session): number => {
  const representative = representativeOf(session)
  const index = slotItems(session.slots.light).findIndex(({ path }) => path === representative?.path)
  return Math.max(index, 0)
}

export { activeSession }

const stackedLights = (workspace: Workspace, choices: OperationChoices) => {
  const reference = referenceSessionOf(workspace, choices.referenceSession)
  if (!reference) return 0
  return [reference, ...otherSessionsToStack(workspace, reference)].reduce(
    (total, session) => total + lightCount(session),
    0,
  )
}

export function canRun(kind: JobKind, mode: Mode, workspace: Workspace, choices: OperationChoices) {
  const session = activeSession(workspace)
  if (kind !== 'calibration' && choices.ground.masking && !choices.ground.maskReady) return false
  if (kind === 'trails') {
    const needed = choices.trails.method === 'gap_filling' ? MIN_STACK_FRAMES : 1
    return mode === 'trails' && hasLights(session) && lightCount(session) >= needed
  }
  if (kind === 'stacking') {
    return mode === 'stacking' && stackedLights(workspace, choices) >= MIN_STACK_FRAMES
  }
  const savesMasters = choices.saveMasters && hasFramesToIntegrate(session)
  const savesLights = choices.saveCalibrated && hasCalibrationData(session) && hasLights(session)
  return savesMasters || savesLights
}

const slotsOf = (session: Session): Record<Exclude<Role, 'light'>, JobSlot> => ({
  bias: toJobSlot(session.slots.bias),
  dark: toJobSlot(session.slots.dark),
  dark_flat: toJobSlot(session.slots.dark_flat),
  flat: toJobSlot(session.slots.flat),
})

/** Turns what is on screen into the request the backend runs. Returns null when there is nothing to do. */
export function buildJobRequest(
  kind: JobKind,
  mode: Mode,
  workspace: Workspace,
  choices: OperationChoices,
): JobRequest | null {
  const stacking = kind === 'stacking'
  const session = stacking
    ? referenceSessionOf(workspace, choices.referenceSession)
    : activeSession(workspace)
  if (!session || !canRun(kind, mode, workspace, choices)) return null
  const others = stacking ? otherSessionsToStack(workspace, session) : []
  const sessions = [session, ...others]
  const hdr = stacking && choices.hdr && joinableExposures(sessions)
  const savesMasters = choices.saveMasters && sessions.some(hasFramesToIntegrate)
  const savesCalibrated = choices.saveCalibrated && sessions.some(hasCalibrationData)
  return {
    kind,
    lights: pathsOf(slotItems(session.slots.light)),
    ...slotsOf(session),
    more_sessions: others.map((other) => ({
      lights: pathsOf(slotItems(other.slots.light)),
      ...slotsOf(other),
    })),
    apply_calibration: choices.applyCalibration && sessions.some(hasCalibrationData),
    masters_folder: savesMasters
      ? (choices.folders.master ?? defaultOutputFolder(workspace, 'master'))
      : null,
    calibrated_folder: savesCalibrated
      ? (choices.folders.calibrated ?? defaultOutputFolder(workspace, 'calibrated'))
      : null,
    result_folder: choices.folders.result ?? defaultOutputFolder(workspace, 'result'),
    quality: choices.quality,
    trails: choices.trails,
    stacking: { ...choices.stacking, reference_index: referenceIndexOf(session) },
    memory_percent: choices.memoryPercent,
    device: choices.device,
    read: choices.read,
    hdr,
    masking:
      kind !== 'calibration' && choices.ground.masking
        ? {
            session_id: session.id,
            ground: choices.ground.separate && !hdr ? 'separate' : 'with_sky',
          }
        : null,
  }
}
