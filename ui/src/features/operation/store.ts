import { create } from 'zustand'

import type {
  DemosaicQuality,
  StackingChoices,
  TrailsRequest,
} from '../../ipc/types'
import type { OutputKind } from './output-folder'

export type Mode = 'trails' | 'stacking'

interface OperationState {
  mode: Mode
  applyCalibration: boolean
  saveMasters: boolean
  saveCalibrated: boolean
  folders: Record<OutputKind, string | null>
  quality: DemosaicQuality
  trails: TrailsRequest
  stacking: StackingChoices
  groundMasking: boolean
  groundSeparate: boolean
  setMode: (mode: Mode) => void
  setApplyCalibration: (apply: boolean) => void
  setSaveMasters: (save: boolean) => void
  setSaveCalibrated: (save: boolean) => void
  setFolder: (kind: OutputKind, folder: string | null) => void
  setQuality: (quality: DemosaicQuality) => void
  setTrails: (change: Partial<TrailsRequest>) => void
  setStacking: (change: Partial<StackingChoices>) => void
  setGroundMasking: (on: boolean) => void
  setGroundSeparate: (on: boolean) => void
}

export const DEFAULT_TRAILS: TrailsRequest = { method: 'lighten', tail_frames: 10, reverse_order: false }

export const DEFAULT_STACKING: StackingChoices = {
  model: 'similarity',
  resampler: 'bicubic',
  combine: 'sigma',
  low: 3,
  high: 3,
  skip_poor_frames: true,
}

export const useOperationStore = create<OperationState>((set) => ({
  mode: 'trails',
  applyCalibration: true,
  saveMasters: false,
  saveCalibrated: false,
  folders: { master: null, calibrated: null, result: null },
  quality: 'fast',
  trails: DEFAULT_TRAILS,
  stacking: DEFAULT_STACKING,
  groundMasking: false,
  groundSeparate: false,
  setMode: (mode) => set({ mode }),
  setApplyCalibration: (applyCalibration) => set({ applyCalibration }),
  setSaveMasters: (saveMasters) => set({ saveMasters }),
  setSaveCalibrated: (saveCalibrated) => set({ saveCalibrated }),
  setQuality: (quality) => set({ quality }),
  setTrails: (change) => set((state) => ({ trails: { ...state.trails, ...change } })),
  setStacking: (change) => set((state) => ({ stacking: { ...state.stacking, ...change } })),
  setGroundMasking: (groundMasking) => set({ groundMasking }),
  setGroundSeparate: (groundSeparate) => set({ groundSeparate }),
  setFolder: (kind, folder) =>
    set((state) => ({ folders: { ...state.folders, [kind]: folder } })),
}))
