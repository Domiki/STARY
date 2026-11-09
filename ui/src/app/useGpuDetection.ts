import { useEffect } from 'react'

import { useSettingsStore } from '../features/settings/store'
import { isTauri } from '../ipc/commands'

/** Looks for the GPU when the program starts, so that it is already chosen when a job is run. */
export function useGpuDetection() {
  useEffect(() => {
    if (isTauri()) void useSettingsStore.getState().detectGpu()
  }, [])
}
