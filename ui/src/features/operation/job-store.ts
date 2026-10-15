import { create } from 'zustand'

import { cancelJob, skyStarCount, startJob } from '../../ipc/commands'
import type {
  CommandError,
  FrameCounts,
  JobFinished,
  JobKind,
  JobProgress,
  UsedDevice,
} from '../../ipc/types'
import { representativeOf } from '../file-list/model/representative'
import { useFileListStore } from '../file-list/store'
import { useMaskStore } from '../mask/store'
import { readChoices } from './choices'
import { activeSession, buildJobRequest } from './job-request'
import { useOperationStore } from './store'

/** Fewer stars than this in the sky and the frames cannot be aligned on it. */
export const MIN_SKY_STARS = 10

export type Outcome =
  | {
      status: 'done'
      kind: JobKind
      seconds: number
      results: number
      frames: FrameCounts | null
      device: UsedDevice | null
      oneReadFull: number | null
    }
  | { status: 'cancelled' }
  | { status: 'error'; code: string; message: string }

interface JobState {
  phase: 'idle' | 'running'
  kind: JobKind | null
  progress: JobProgress | null
  outcome: Outcome | null
  /** Why each light of the last job that is not in its result was left out, by path. */
  leftOut: Record<string, string>
  /** Asked to run with a sky that has too few stars to align on: waits for the answer. */
  skyWarning: { kind: JobKind; stars: number } | null
  start: (kind: JobKind, confirmed?: boolean) => Promise<void>
  dismissSkyWarning: () => void
  cancel: () => Promise<void>
  handleProgress: (progress: JobProgress) => void
  handleFinished: (finished: JobFinished) => Promise<void>
}

const failed = (error: unknown): Outcome => {
  const { code, message } = error as Partial<CommandError>
  return { status: 'error', code: code ?? 'internal', message: message ?? String(error) }
}

export const useJobStore = create<JobState>((set, get) => ({
  phase: 'idle',
  kind: null,
  progress: null,
  outcome: null,
  leftOut: {},
  skyWarning: null,
  start: async (kind, confirmed = false) => {
    if (get().phase === 'running') return
    const workspace = useFileListStore.getState().workspace
    const request = buildJobRequest(kind, useOperationStore.getState().mode, workspace, readChoices())
    if (!request) return
    set({ skyWarning: null, leftOut: {} })
    try {
      if (request.masking) {
        const { session_id } = request.masking
        await useMaskStore.getState().publish(session_id)
        const session = activeSession(workspace)
        const representative = session && representativeOf(session)
        const stars =
          kind === 'stacking' && representative
            ? await skyStarCount(session_id, representative.path).catch(() => null)
            : null
        if (!confirmed && stars !== null && stars < MIN_SKY_STARS) {
          set({ skyWarning: { kind, stars } })
          return
        }
      }
      set({ phase: 'running', kind, progress: null, outcome: null })
      await startJob(request)
    } catch (error) {
      set({ phase: 'idle', kind: null, outcome: failed(error) })
    }
  },
  dismissSkyWarning: () => set({ skyWarning: null }),
  cancel: async () => {
    if (get().phase === 'running') await cancelJob()
  },
  handleProgress: (progress) => set({ progress }),
  handleFinished: async (finished) => {
    const kind = get().kind ?? 'trails'
    if (finished.status === 'done') {
      const results = finished.outputs.filter((output) => output.kind === 'result')
      for (const output of results) await useFileListStore.getState().addResult(output.path)
      set({
        phase: 'idle',
        kind: null,
        progress: null,
        leftOut: Object.fromEntries(
          (finished.frames?.left_out ?? []).map(({ path, reason }) => [path, reason]),
        ),
        outcome: {
          status: 'done',
          kind,
          seconds: finished.seconds,
          results: results.length,
          frames: finished.frames,
          device: finished.device,
          oneReadFull: finished.one_read_full,
        },
      })
    } else if (finished.status === 'cancelled') {
      set({ phase: 'idle', kind: null, progress: null, outcome: { status: 'cancelled' } })
    } else {
      set({
        phase: 'idle',
        kind: null,
        progress: null,
        outcome: { status: 'error', ...finished.failure },
      })
    }
  },
}))

useJobStore.subscribe((state) => {
  const locked = state.phase === 'running'
  if (useFileListStore.getState().locked !== locked) useFileListStore.setState({ locked })
})
