import { useEffect } from 'react'

import { useUpdateStore } from '../features/update/store'
import { isTauri } from '../ipc/commands'

let checked = false

/** Looks once, when the program starts, for a release that is newer than this one. */
export function useUpdateCheck() {
  useEffect(() => {
    if (checked || !isTauri()) return
    checked = true
    void useUpdateStore.getState().checkAtStart()
  }, [])
}
