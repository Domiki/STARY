import { create } from 'zustand'

import { clearMask, detectSky, setMask, shapeSky } from '../../ipc/commands'
import type { Contour, SkyStroke } from '../../ipc/types'
import { type History, NO_HISTORY, record, redo, undo } from './model/history'

export type Tool = 'hand' | 'sky' | 'ground' | 'refine'
export type Phase = 'none' | 'contour' | 'applied'

export const DEFAULT_FEATHER = 4

/** What is painted for one session, as it is at one moment: a step of the history. */
export interface MaskState {
  phase: Phase
  width: number
  height: number
  strokes: SkyStroke[]
  /** Contour phase: 255 where the sky was found. */
  region: Uint8Array | null
  contours: Contour[]
  offset: number
  feather: number
  /** Contour phase: the mask the region gives with the offset and the feather. */
  shaped: Uint8Array | null
  /** Applied phase: the mask, 255 for sky and 0 for ground. */
  mask: Uint8Array | null
}

export const EMPTY_MASK: MaskState = {
  phase: 'none',
  width: 0,
  height: 0,
  strokes: [],
  region: null,
  contours: [],
  offset: 0,
  feather: DEFAULT_FEATHER,
  shaped: null,
  mask: null,
}

interface Entry {
  state: MaskState
  history: History<MaskState>
}

export interface Redetect {
  sessionId: string
  path: string
}

interface MaskStore {
  entries: Record<string, Entry>
  busy: boolean
  /** What went wrong the last time the sky was looked for. */
  failure: string | null
  tool: Tool
  brushRadius: number
  hardness: number
  /** Asked to find the sky where a mask already is: waits for the answer. */
  confirming: Redetect | null
  setTool: (tool: Tool) => void
  setBrushRadius: (radius: number) => void
  setHardness: (hardness: number) => void
  requestDetect: (target: Redetect) => Promise<void>
  cancelConfirm: () => void
  detect: (target: Redetect) => Promise<void>
  guide: (target: Redetect, stroke: SkyStroke) => Promise<void>
  shape: (sessionId: string, change: { offset?: number; feather?: number }) => Promise<void>
  apply: (sessionId: string) => void
  commitPaint: (sessionId: string, mask: Uint8Array) => void
  clear: (sessionId: string) => void
  undo: (sessionId: string) => Promise<void>
  redo: (sessionId: string) => Promise<void>
  /** Hands the mask of a session to the backend, where the job reads it. */
  publish: (sessionId: string) => Promise<boolean>
}

const entryOf = (entries: Record<string, Entry>, sessionId: string): Entry =>
  entries[sessionId] ?? { state: EMPTY_MASK, history: NO_HISTORY }

export const maskOf = (entries: Record<string, Entry>, sessionId: string | undefined): MaskState =>
  (sessionId ? entries[sessionId]?.state : undefined) ?? EMPTY_MASK

const latest = new Map<string, number>()
const nextRequest = (sessionId: string) => {
  const request = (latest.get(sessionId) ?? 0) + 1
  latest.set(sessionId, request)
  return request
}
const isLatest = (sessionId: string, request: number) => latest.get(sessionId) === request

const pendingShape = new Map<string, { offset: number; feather: number } | null>()

export const useMaskStore = create<MaskStore>((set, get) => {
  const replace = (sessionId: string, state: MaskState, remember = true) =>
    set(({ entries }) => {
      const entry = entryOf(entries, sessionId)
      return {
        entries: {
          ...entries,
          [sessionId]: {
            state,
            history: remember ? record(entry.history, entry.state) : entry.history,
          },
        },
      }
    })

  const detected = async (
    path: string,
    strokes: SkyStroke[],
    offset: number,
    feather: number,
  ): Promise<MaskState> => {
    const found = await detectSky(path, strokes)
    const region = { width: found.width, height: found.height, bytes: found.region }
    const shaped = await shapeSky(region, offset, feather)
    return {
      phase: 'contour',
      width: found.width,
      height: found.height,
      strokes,
      region: found.region,
      contours: found.contours,
      offset,
      feather,
      shaped,
      mask: null,
    }
  }

  const working = async (job: () => Promise<void>) => {
    set({ busy: true, failure: null })
    try {
      await job()
    } catch (error) {
      set({ failure: (error as { message?: string }).message ?? String(error) })
    } finally {
      set({ busy: false })
    }
  }

  const reshape = async (sessionId: string, offset: number, feather: number) => {
    const { state } = entryOf(get().entries, sessionId)
    if (!state.region) return
    const shaped = await shapeSky(
      { width: state.width, height: state.height, bytes: state.region },
      offset,
      feather,
    )
    set(({ entries }) => {
      const entry = entryOf(entries, sessionId)
      return {
        entries: { ...entries, [sessionId]: { ...entry, state: { ...entry.state, offset, feather, shaped } } },
      }
    })
  }

  return {
    entries: {},
    busy: false,
    failure: null,
    tool: 'hand',
    brushRadius: 24,
    hardness: 0.8,
    confirming: null,
    setTool: (tool) => set({ tool }),
    setBrushRadius: (brushRadius) => set({ brushRadius }),
    setHardness: (hardness) => set({ hardness }),

    requestDetect: async (target) => {
      if (maskOf(get().entries, target.sessionId).phase === 'none') return get().detect(target)
      set({ confirming: target })
    },
    cancelConfirm: () => set({ confirming: null }),

    detect: async (target) => {
      set({ confirming: null })
      const request = nextRequest(target.sessionId)
      await working(async () => {
        const state = await detected(target.path, [], 0, DEFAULT_FEATHER)
        if (isLatest(target.sessionId, request)) replace(target.sessionId, state)
      })
    },

    guide: async (target, stroke) => {
      const { state } = entryOf(get().entries, target.sessionId)
      if (state.phase !== 'contour') return
      const request = nextRequest(target.sessionId)
      await working(async () => {
        const next = await detected(
          target.path,
          [...state.strokes, stroke],
          state.offset,
          state.feather,
        )
        if (isLatest(target.sessionId, request)) replace(target.sessionId, next)
      })
    },

    shape: async (sessionId, change) => {
      const { state } = entryOf(get().entries, sessionId)
      const wanted = {
        offset: change.offset ?? state.offset,
        feather: change.feather ?? state.feather,
      }
      set(({ entries }) => {
        const entry = entryOf(entries, sessionId)
        return {
          entries: { ...entries, [sessionId]: { ...entry, state: { ...entry.state, ...wanted } } },
        }
      })
      if (pendingShape.has(sessionId)) {
        pendingShape.set(sessionId, wanted)
        return
      }
      pendingShape.set(sessionId, null)
      try {
        await reshape(sessionId, wanted.offset, wanted.feather)
        for (let again = pendingShape.get(sessionId); again; again = pendingShape.get(sessionId)) {
          pendingShape.set(sessionId, null)
          await reshape(sessionId, again.offset, again.feather)
        }
      } finally {
        pendingShape.delete(sessionId)
      }
    },

    apply: (sessionId) => {
      const { state } = entryOf(get().entries, sessionId)
      if (state.phase !== 'contour' || !state.shaped) return
      if (get().tool === 'refine') set({ tool: 'hand' })
      replace(sessionId, {
        ...EMPTY_MASK,
        phase: 'applied',
        width: state.width,
        height: state.height,
        mask: state.shaped,
      })
    },

    commitPaint: (sessionId, mask) => {
      const { state } = entryOf(get().entries, sessionId)
      if (state.phase !== 'applied') return
      replace(sessionId, { ...state, mask })
    },

    clear: (sessionId) => {
      nextRequest(sessionId)
      replace(sessionId, EMPTY_MASK)
      void clearMask(sessionId).catch(() => {})
    },

    undo: async (sessionId) => {
      const entry = entryOf(get().entries, sessionId)
      const back = undo(entry.history, entry.state)
      if (!back) return
      nextRequest(sessionId)
      set(({ entries }) => ({
        entries: { ...entries, [sessionId]: { state: back.present, history: back.history } },
      }))
    },

    redo: async (sessionId) => {
      const entry = entryOf(get().entries, sessionId)
      const forward = redo(entry.history, entry.state)
      if (!forward) return
      nextRequest(sessionId)
      set(({ entries }) => ({
        entries: { ...entries, [sessionId]: { state: forward.present, history: forward.history } },
      }))
    },

    publish: async (sessionId) => {
      const { state } = entryOf(get().entries, sessionId)
      if (state.phase !== 'applied' || !state.mask) return false
      await setMask(sessionId, { width: state.width, height: state.height, bytes: state.mask })
      return true
    },
  }
})

export const canUndo = (entries: Record<string, Entry>, sessionId: string | undefined) =>
  sessionId !== undefined && (entries[sessionId]?.history.past.length ?? 0) > 0

export const canRedo = (entries: Record<string, Entry>, sessionId: string | undefined) =>
  sessionId !== undefined && (entries[sessionId]?.history.future.length ?? 0) > 0
