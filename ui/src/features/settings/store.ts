import { create } from 'zustand'

import { i18n, resolveFor } from '../../i18n'
import { gpuInfo } from '../../ipc/commands'
import type { ComputeDevice, GpuInfo, ReadMode } from '../../ipc/types'
import { loadLanguagePreference, saveLanguagePreference } from './language-preference'

export type { ComputeDevice }

export const DEFAULT_MEMORY_PERCENT = 50

interface SettingsState {
  languagePreference: string
  dialogOpen: boolean
  device: ComputeDevice
  /** `undefined` until it has been looked for, `null` when there is none. */
  gpu: GpuInfo | null | undefined
  memoryPercent: number
  readMode: ReadMode
  detectGpu: () => Promise<void>
  setLanguagePreference: (preference: string) => void
  overrideLanguage: (code: string) => void
  setDialogOpen: (open: boolean) => void
  setDevice: (device: ComputeDevice) => void
  setMemoryPercent: (percent: number) => void
  setReadMode: (mode: ReadMode) => void
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  languagePreference: loadLanguagePreference(),
  dialogOpen: false,
  device: 'cpu',
  gpu: undefined,
  memoryPercent: DEFAULT_MEMORY_PERCENT,
  readMode: 'exact',
  detectGpu: async () => {
    if (get().gpu !== undefined) return
    const gpu = await gpuInfo().catch(() => null)
    set({ gpu, ...(gpu ? { device: 'gpu' as const } : {}) })
  },
  setLanguagePreference: (preference) => {
    saveLanguagePreference(preference)
    void i18n.changeLanguage(resolveFor(preference))
    set({ languagePreference: preference })
  },
  overrideLanguage: (code) => void i18n.changeLanguage(resolveFor(code)),
  setDialogOpen: (dialogOpen) => set({ dialogOpen }),
  setDevice: (device) => set({ device }),
  setMemoryPercent: (memoryPercent) => set({ memoryPercent }),
  setReadMode: (readMode) => set({ readMode }),
}))
