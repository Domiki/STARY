import { useFileListStore } from '../file-list/store'
import { joinableExposures, sessionsToStack } from './reference-session'
import { useOperationStore } from './store'

/** Whether the exposures of the sessions can be joined, and whether they are going to be. */
export function useHdr(): { available: boolean; active: boolean } {
  const chosen = useOperationStore((state) => state.referenceSession)
  const wanted = useOperationStore((state) => state.hdr)
  const mode = useOperationStore((state) => state.mode)
  const available = useFileListStore((state) =>
    joinableExposures(sessionsToStack(state.workspace, chosen)),
  )
  return { available, active: mode === 'stacking' && wanted && available }
}
