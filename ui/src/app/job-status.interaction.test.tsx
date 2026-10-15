// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../i18n'
import { cancelJob } from '../ipc/commands'
import { useJobStore } from '../features/operation/job-store'
import { StatusBar } from './StatusBar'

vi.mock('../ipc/commands', () => ({
  startJob: vi.fn(async () => {}),
  cancelJob: vi.fn(async () => {}),
  probeFiles: vi.fn(),
  expandPaths: vi.fn(),
  pickFiles: vi.fn(),
  listenForDrops: vi.fn(),
}))

const idle = { phase: 'idle', kind: null, progress: null, outcome: null } as const

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(cancelJob).mockClear()
  useJobStore.setState(idle)
})

afterEach(cleanup)

describe('status bar job progress', () => {
  it('shows the normal status while no job has run', () => {
    render(<StatusBar />)
    expect(screen.getByText('Ready')).toBeTruthy()
    expect(screen.queryByRole('progressbar')).toBeNull()
  })

  it('shows the stage, a progress bar and Cancel while running', () => {
    render(<StatusBar />)
    act(() => useJobStore.setState({ phase: 'running', kind: 'trails' }))
    expect(screen.getByText('Starting…')).toBeTruthy()

    act(() =>
      useJobStore.setState({ progress: { stage: 'lights', done: 10, total: 40 } }),
    )
    expect(screen.getByText('Lights')).toBeTruthy()
    expect(screen.getByText('10/40')).toBeTruthy()
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('25')

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(cancelJob).toHaveBeenCalledTimes(1)
  })

  it('warns when masters made in one read had much room used up', () => {
    useJobStore.setState({
      ...idle,
      outcome: { status: 'done', kind: 'calibration', seconds: 1, results: 0, frames: null, device: null, oneReadFull: 250 },
    })
    render(<StatusBar />)
    expect(screen.getByText(/made by reading once/)).toBeTruthy()
  })

  it('says nothing about one read when little room was used', () => {
    useJobStore.setState({
      ...idle,
      outcome: { status: 'done', kind: 'calibration', seconds: 1, results: 0, frames: null, device: null, oneReadFull: 26 },
    })
    render(<StatusBar />)
    expect(screen.queryByText(/made by reading once/)).toBeNull()
  })

  it.each([
    [{ status: 'done', kind: 'trails', seconds: 2.56, results: 1, frames: null, device: null, oneReadFull: null }, 'Done in 2.6 s. The result was added to Result.'],
    [{ status: 'done', kind: 'calibration', seconds: 1.8, results: 0, frames: null, device: null, oneReadFull: null }, 'Done in 1.8 s'],
    [{ status: 'done', kind: 'stacking', seconds: 30, results: 1, frames: { used: 40, total: 40, misaligned: 0, poor: 0, left_out: [] }, device: null, oneReadFull: null }, 'Done in 30.0 s. The result was added to Result.'],
    [{ status: 'done', kind: 'stacking', seconds: 30, results: 1, frames: { used: 36, total: 40, misaligned: 1, poor: 3, left_out: [] }, device: null, oneReadFull: null }, 'Done in 30.0 s. 36 of 40 frames were stacked (1 could not be aligned, 3 left out for poor quality). The result was added to Result.'],
    [{ status: 'error', code: 'not_enough_frames', message: '' }, 'Fewer than two frames could be aligned. Check that the frames show the same part of the sky.'],
    [{ status: 'cancelled' }, 'Cancelled'],
    [{ status: 'error', code: 'no_input', message: '' }, 'There are no light frames to process.'],
    [{ status: 'error', code: 'unheard_of', message: 'boom' }, 'The job failed. boom'],
  ] as const)('tells the outcome %#', (outcome, text) => {
    useJobStore.setState({ ...idle, outcome })
    render(<StatusBar />)
    expect(screen.getByText(text)).toBeTruthy()
  })
})
