import { useFileListStore } from '../file-list/store'
import { maskOf, useMaskStore } from '../mask/store'
import { useSettingsStore } from '../settings/store'
import { choicesFrom } from './choices'
import { maskSessionOf } from './reference-session'
import type { OperationChoices } from './job-request'
import { useOperationStore } from './store'

export function useOperationChoices(): OperationChoices {
  const operation = useOperationStore()
  const memoryPercent = useSettingsStore((state) => state.memoryPercent)
  const device = useSettingsStore((state) => state.device)
  const readMode = useSettingsStore((state) => state.readMode)
  const sessionId = useFileListStore(
    (state) => maskSessionOf(state.workspace, operation.mode, operation.referenceSession)?.id,
  )
  const maskReady = useMaskStore((state) => maskOf(state.entries, sessionId).phase === 'applied' && !state.busy)
  return choicesFrom(operation, { memoryPercent, device, readMode }, maskReady)
}
