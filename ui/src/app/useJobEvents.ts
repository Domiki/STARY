import { useEffect } from 'react'

import { useJobStore } from '../features/operation/job-store'
import { isTauri, listenForJobs } from '../ipc/commands'

export function useJobEvents() {
  useEffect(() => {
    if (!isTauri()) return
    let stop: (() => void) | undefined
    let cancelled = false
    void listenForJobs({
      onProgress: (progress) => useJobStore.getState().handleProgress(progress),
      onFinished: (finished) => void useJobStore.getState().handleFinished(finished),
    }).then((unlisten) => {
      if (cancelled) unlisten()
      else stop = unlisten
    })
    return () => {
      cancelled = true
      stop?.()
    }
  }, [])
}
