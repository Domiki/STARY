import { create } from 'zustand'

import { appVersion, checkUpdate } from '../../ipc/commands'
import type { NewerRelease } from '../../ipc/types'
import { loadCheckUpdates, saveCheckUpdates } from './preference'

/** What the last look for a newer release found. */
export type Outcome = 'none' | 'checking' | 'newer' | 'latest' | 'failed'

interface UpdateState {
  /** Look for a newer release when the program starts. */
  enabled: boolean
  version: string | null
  outcome: Outcome
  release: NewerRelease | null
  /** The window that tells about the newer release is open. */
  dialogOpen: boolean
  setEnabled: (enabled: boolean) => void
  /** At start: only when the person did not turn it off, and quietly when it fails. */
  checkAtStart: () => Promise<void>
  /** On request (the About window): says what it found, also when there is nothing new. */
  checkNow: () => Promise<void>
  closeDialog: () => void
}

async function look(set: (state: Partial<UpdateState>) => void): Promise<Outcome> {
  set({ outcome: 'checking' })
  try {
    const version = await appVersion()
    const release = await checkUpdate(version)
    set({ version, release, outcome: release ? 'newer' : 'latest' })
    return release ? 'newer' : 'latest'
  } catch {
    set({ outcome: 'failed' })
    return 'failed'
  }
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  enabled: loadCheckUpdates(),
  version: null,
  outcome: 'none',
  release: null,
  dialogOpen: false,
  setEnabled: (enabled) => {
    saveCheckUpdates(enabled)
    set({ enabled })
  },
  checkAtStart: async () => {
    if (!get().enabled) return
    if ((await look(set)) === 'newer') set({ dialogOpen: true })
  },
  checkNow: async () => {
    if ((await look(set)) === 'newer') set({ dialogOpen: true })
  },
  closeDialog: () => set({ dialogOpen: false }),
}))
