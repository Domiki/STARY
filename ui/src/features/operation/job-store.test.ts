import { beforeEach, describe, expect, it, vi } from 'vitest'

import { cancelJob, startJob } from '../../ipc/commands'
import { placeInfo } from '../file-list/model/placement'
import { info } from '../file-list/model/test-fixtures'
import { emptyWorkspace } from '../file-list/model/workspace'
import { useFileListStore } from '../file-list/store'
import { useJobStore } from './job-store'
import { useOperationStore } from './store'

vi.mock('../../ipc/commands', () => ({
  startJob: vi.fn(async () => {}),
  cancelJob: vi.fn(async () => {}),
  probeFiles: vi.fn(),
  expandPaths: vi.fn(),
  pickFiles: vi.fn(),
  listenForDrops: vi.fn(),
}))

const withLight = () => {
  const workspace = placeInfo(emptyWorkspace(), info({ path: '/sky/lights/1.raf' })).workspace
  useFileListStore.setState({ workspace })
}

const idle = { phase: 'idle', kind: null, progress: null, outcome: null } as const

beforeEach(() => {
  vi.mocked(startJob).mockReset().mockResolvedValue(undefined)
  vi.mocked(cancelJob).mockReset().mockResolvedValue(undefined)
  useJobStore.setState(idle)
  useOperationStore.setState({ mode: 'trails' })
  useFileListStore.setState({ workspace: emptyWorkspace() })
})

describe('job store', () => {
  it('does nothing when there is nothing to run', async () => {
    await useJobStore.getState().start('trails')
    expect(startJob).not.toHaveBeenCalled()
    expect(useJobStore.getState().phase).toBe('idle')
  })

  it('runs one job at a time', async () => {
    withLight()
    await useJobStore.getState().start('trails')
    await useJobStore.getState().start('trails')
    expect(startJob).toHaveBeenCalledTimes(1)
    expect(useJobStore.getState()).toMatchObject({ phase: 'running', kind: 'trails' })
  })

  it('reports a request the backend refused', async () => {
    withLight()
    vi.mocked(startJob).mockRejectedValue({ code: 'busy', message: 'a job is running' })
    await useJobStore.getState().start('trails')
    expect(useJobStore.getState()).toMatchObject({
      phase: 'idle',
      outcome: { status: 'error', code: 'busy' },
    })
  })

  it('keeps the latest progress while running', async () => {
    withLight()
    await useJobStore.getState().start('trails')
    useJobStore.getState().handleProgress({ stage: 'lights', done: 3, total: 40 })
    expect(useJobStore.getState().progress).toEqual({ stage: 'lights', done: 3, total: 40 })
  })

  it('adds each finished result to the Result section', async () => {
    withLight()
    const addResult = vi.fn(async () => {})
    useFileListStore.setState({ addResult })
    await useJobStore.getState().start('trails')
    await useJobStore.getState().handleFinished({
      status: 'done',
      seconds: 2.5,
      frames: { used: 40, total: 40, misaligned: 0, poor: 0, left_out: [] },
      device: { kind: 'gpu', name: 'Test GPU' },
      one_read_full: null,
      outputs: [
        { kind: 'result', path: '/out/a.tiff' },
        { kind: 'master', path: '/out/masters/dark.fits' },
      ],
    })
    expect(addResult).toHaveBeenCalledTimes(1)
    expect(addResult).toHaveBeenCalledWith('/out/a.tiff')
    expect(useJobStore.getState()).toMatchObject({
      phase: 'idle',
      outcome: {
        status: 'done',
        kind: 'trails',
        seconds: 2.5,
        results: 1,
        device: { kind: 'gpu', name: 'Test GPU' },
        oneReadFull: null,
      },
    })
  })

  it('goes back to idle when cancelled or failed', async () => {
    withLight()
    await useJobStore.getState().start('trails')
    await useJobStore.getState().handleFinished({ status: 'cancelled' })
    expect(useJobStore.getState()).toMatchObject({ phase: 'idle', outcome: { status: 'cancelled' } })

    await useJobStore.getState().start('trails')
    await useJobStore
      .getState()
      .handleFinished({ status: 'error', failure: { code: 'raw', message: 'bad file' } })
    expect(useJobStore.getState().outcome).toEqual({ status: 'error', code: 'raw', message: 'bad file' })
  })

  it('only asks the backend to cancel while running', async () => {
    await useJobStore.getState().cancel()
    expect(cancelJob).not.toHaveBeenCalled()
    withLight()
    await useJobStore.getState().start('trails')
    await useJobStore.getState().cancel()
    expect(cancelJob).toHaveBeenCalledTimes(1)
  })
})
