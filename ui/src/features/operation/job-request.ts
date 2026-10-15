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

export const activeSession = (workspace: Workspace): Session | undefined =>
  workspace.sessions.find(({ id }) => id === workspace.activeSessionId)

export function canRun(kind: JobKind, mode: Mode, workspace: Workspace, choices: OperationChoices) {
  const session = activeSession(workspace)
  if (kind !== 'calibration' && choices.ground.masking && !choices.ground.maskReady) return false
  if (kind === 'trails') {
    const needed = choices.trails.method === 'gap_filling' ? MIN_STACK_FRAMES : 1
    return mode === 'trails' && hasLights(session) && lightCount(session) >= needed
  }
  if (kind === 'stacking') return mode === 'stacking' && lightCount(session) >= MIN_STACK_FRAMES
  const savesMasters = choices.saveMasters && hasFramesToIntegrate(session)
  const savesLights = choices.saveCalibrated && hasCalibrationData(session) && hasLights(session)
  return savesMasters || savesLights
}

/** Turns what is on screen into the request the backend runs. Returns null when there is nothing to do. */
export function buildJobRequest(
  kind: JobKind,
  mode: Mode,
  workspace: Workspace,
  choices: OperationChoices,
): JobRequest | null {
  const session = activeSession(workspace)
  if (!session || !canRun(kind, mode, workspace, choices)) return null
  const slots: Record<Exclude<Role, 'light'>, JobSlot> = {
    bias: toJobSlot(session.slots.bias),
    dark: toJobSlot(session.slots.dark),
    dark_flat: toJobSlot(session.slots.dark_flat),
    flat: toJobSlot(session.slots.flat),
  }
  const savesMasters = choices.saveMasters && hasFramesToIntegrate(session)
  const savesCalibrated = choices.saveCalibrated && hasCalibrationData(session)
  return {
    kind,
    lights: pathsOf(slotItems(session.slots.light)),
    ...slots,
    apply_calibration: choices.applyCalibration && hasCalibrationData(session),
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
    masking:
      kind !== 'calibration' && choices.ground.masking
        ? {
            session_id: session.id,
            ground: choices.ground.separate ? 'separate' : 'with_sky',
          }
        : null,
  }
}
