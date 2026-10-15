import { representativeOf } from '../file-list/model/representative'
import { useFileListStore } from '../file-list/store'
import { activeSession } from '../operation/job-request'
import { useOperationStore } from '../operation/store'

/**
 * Where the sky can be painted now: on the representative image of the active session, while
 * ground masking is on and that image is the one on show.
 */
export function useMaskTarget(): { sessionId: string; path: string } | null {
  const masking = useOperationStore((state) => state.groundMasking)
  const session = useFileListStore((state) => activeSession(state.workspace))
  const selected = useFileListStore((state) => state.selection.primary)
  const representative = session ? representativeOf(session) : null
  if (!masking || !session || !representative || selected !== representative.path) return null
  return { sessionId: session.id, path: representative.path }
}
