import { useFileListStore } from '../file-list/store'
import { maskOf, useMaskStore } from '../mask/store'
import { useSettingsStore } from '../settings/store'
import { maskSessionOf } from './reference-session'
import type { OperationChoices } from './job-request'
import { useOperationStore } from './store'

type Operation = ReturnType<typeof useOperationStore.getState>
type Settings = ReturnType<typeof useSettingsStore.getState>

export const choicesFrom = (
  operation: Operation,
  settings: Pick<Settings, 'memoryPercent' | 'device' | 'readMode'>,
  maskReady: boolean,
): OperationChoices => ({
  applyCalibration: operation.applyCalibration,
  saveMasters: operation.saveMasters,
  saveCalibrated: operation.saveCalibrated,
  folders: operation.folders,
  quality: operation.quality,
  trails: operation.trails,
  stacking: operation.stacking,
  memoryPercent: settings.memoryPercent,
  device: settings.device,
  read: settings.readMode,
  ground: {
    masking: operation.groundMasking,
    separate: operation.groundSeparate,
    maskReady,
  },
  referenceSession: operation.referenceSession,
  hdr: operation.hdr,
})

/** What is on screen to choose from, read at this moment (for the moment a job starts). */
export function readChoices(): OperationChoices {
  const operation = useOperationStore.getState()
  const workspace = useFileListStore.getState().workspace
  const sessionId = maskSessionOf(workspace, operation.mode, operation.referenceSession)?.id
  const { entries, busy } = useMaskStore.getState()
  const maskReady = maskOf(entries, sessionId).phase === 'applied' && !busy
  return choicesFrom(operation, useSettingsStore.getState(), maskReady)
}
