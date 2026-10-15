import { create } from 'zustand'

import { openImage } from '../../ipc/commands'
import type { CommandError, ImageView } from '../../ipc/types'

type Phase = 'idle' | 'loading' | 'ready' | 'error'

interface ViewerState {
  phase: Phase
  view: ImageView | null
  show: (path: string | null) => Promise<void>
}

let latestRequest = 0

const release = (view: ImageView | null): void => {
  if (view) URL.revokeObjectURL(view.url)
}

export const useViewerStore = create<ViewerState>((set, get) => ({
  phase: 'idle',
  view: null,
  show: async (path) => {
    const request = ++latestRequest
    release(get().view)
    if (path === null) return set({ phase: 'idle', view: null })
    set({ phase: 'loading', view: null })
    try {
      const view = await openImage(path)
      if (request !== latestRequest) return release(view)
      set({ phase: 'ready', view })
    } catch (error) {
      if (request !== latestRequest) return
      console.error('open_image failed', error as CommandError)
      set({ phase: 'error', view: null })
    }
  },
}))
