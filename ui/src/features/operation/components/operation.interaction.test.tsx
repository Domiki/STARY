// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../../../i18n'
import { gpuInfo, pickFolder, skyStarCount, startJob } from '../../../ipc/commands'
import type { Role } from '../../../ipc/types'
import { placeInfo } from '../../file-list/model/placement'
import { info } from '../../file-list/model/test-fixtures'
import { type Workspace, emptyWorkspace } from '../../file-list/model/workspace'
import { useFileListStore } from '../../file-list/store'
import { SettingsDialog } from '../../settings/components/SettingsDialog'
import { useMaskStore } from '../../mask/store'
import { useSettingsStore } from '../../settings/store'
import { useJobStore } from '../job-store'
import { DEFAULT_STACKING, DEFAULT_TRAILS, useOperationStore } from '../store'
import { OperationPanel } from './OperationPanel'

vi.mock('../../../ipc/commands', () => ({
  pickFolder: vi.fn(async () => '/chosen/folder'),
  startJob: vi.fn(async () => {}),
  cancelJob: vi.fn(async () => {}),
  probeFiles: vi.fn(),
  expandPaths: vi.fn(),
  pickFiles: vi.fn(),
  listenForDrops: vi.fn(),
  gpuInfo: vi.fn(async () => null),
  skyStarCount: vi.fn(async () => 99),
  setMask: vi.fn(async () => {}),
  clearMask: vi.fn(async () => {}),
  detectSky: vi.fn(),
  shapeSky: vi.fn(),
}))

function seed(entries: [Role, string][]): Workspace {
  let workspace = placeInfo(emptyWorkspace(), info({ path: '/sky/lights/1.raf' })).workspace
  const sessionId = workspace.sessions[0]!.id
  for (const [role, path] of entries) {
    workspace = placeInfo(workspace, info({ path }), { sessionId, role }).workspace
  }
  return workspace
}

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(startJob).mockClear()
  useFileListStore.setState({ workspace: seed([]) })
  useOperationStore.setState({
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
  })
  useMaskStore.setState({ entries: {} })
  useJobStore.setState({ phase: 'idle', kind: null, progress: null, outcome: null })
  vi.mocked(gpuInfo).mockReset().mockResolvedValue(null)
  useSettingsStore.setState({ dialogOpen: false, device: 'cpu', gpu: undefined, memoryPercent: 50, readMode: 'exact' })
})

afterEach(cleanup)

describe('calibration section', () => {
  it('exists without calibration files but nothing can be applied', () => {
    render(<OperationPanel />)
    const toggle = screen.getByRole('switch', { name: /Apply calibration/ }) as HTMLInputElement
    expect(toggle.disabled).toBe(true)
    expect(screen.getAllByText('None')).toHaveLength(4)
  })

  it('summarises each role of the active session', () => {
    useFileListStore.setState({
      workspace: seed([['dark', '/sky/darks/1.raf'], ['dark', '/sky/darks/2.raf'], ['bias', '/sky/b.raf']]),
    })
    render(<OperationPanel />)
    expect(screen.getByText('2 frames')).toBeTruthy()
    expect(screen.getByText('1 frames')).toBeTruthy()
    const toggle = screen.getByRole('switch', { name: /Apply calibration/ }) as HTMLInputElement
    expect(toggle.disabled).toBe(false)
  })

  it('shows the save folders only when the matching save option is on', async () => {
    useFileListStore.setState({
      workspace: seed([['dark', '/sky/darks/1.raf']]),
    })
    render(<OperationPanel />)
    expect(screen.queryByTitle('/sky/lights/stary-output/masters')).toBeNull()

    fireEvent.click(screen.getByRole('switch', { name: /Save masters/ }))
    expect(screen.getByTitle('/sky/lights/stary-output/masters')).toBeTruthy()
    expect(screen.queryByTitle('/sky/lights/stary-output/calibrated')).toBeNull()

    fireEvent.click(screen.getByRole('switch', { name: /Save calibrated lights/ }))
    expect(screen.getByTitle('/sky/lights/stary-output/calibrated')).toBeTruthy()
    expect(screen.getByText(/About .* will be written/)).toBeTruthy()

    fireEvent.click(screen.getAllByText('Change…')[0]!)
    await waitFor(() => expect(useOperationStore.getState().folders.master).toBe('/chosen/folder'))
    expect(useOperationStore.getState().folders.calibrated).toBeNull()
    expect(vi.mocked(pickFolder)).toHaveBeenCalled()
  })

  it('disables both save options without anything to save', () => {
    render(<OperationPanel />)
    const masters = screen.getByRole('switch', { name: /Save masters/ }) as HTMLInputElement
    const calibrated = screen.getByRole('switch', {
      name: /Save calibrated lights/,
    }) as HTMLInputElement
    expect(masters.disabled).toBe(true)
    expect(calibrated.disabled).toBe(true)
  })

  it('only offers to run calibration when something is set to be saved', async () => {
    useFileListStore.setState({ workspace: seed([['dark', '/sky/darks/1.raf']]) })
    render(<OperationPanel />)
    const run = screen.getByRole('button', { name: 'Run calibration only' }) as HTMLButtonElement
    expect(run.disabled).toBe(true)

    fireEvent.click(screen.getByRole('switch', { name: /Save masters/ }))
    expect(run.disabled).toBe(false)
    fireEvent.click(run)
    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalled())
    expect(vi.mocked(startJob).mock.calls.at(-1)![0]).toMatchObject({
      kind: 'calibration',
      masters_folder: '/sky/lights/stary-output/masters',
    })
  })
})

describe('mode section', () => {
  it('switches between Trails and Stacking options', () => {
    render(<OperationPanel />)
    expect(screen.queryByText('Alignment model')).toBeNull()
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    expect(screen.getByText('Alignment model')).toBeTruthy()
    expect(screen.getByText('Ground masking')).toBeTruthy()
  })

  it('runs Trails with the chosen demosaic quality and no choice of result format', async () => {
    render(<OperationPanel />)
    const run = screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement
    expect(run.disabled).toBe(false)

    expect(screen.queryByLabelText('Result format')).toBeNull()
    fireEvent.change(screen.getByLabelText('Demosaic quality'), { target: { value: 'high' } })
    fireEvent.click(run)

    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0]).toMatchObject({
      kind: 'trails',
      lights: ['/sky/lights/1.raf'],
      quality: 'high',
      result_folder: '/sky/lights/stary-output',
    })
  })

  it('disables Run while a job is running', () => {
    render(<OperationPanel />)
    const run = screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement
    act(() => useJobStore.setState({ phase: 'running', kind: 'trails' }))
    expect(run.disabled).toBe(true)
    act(() => useJobStore.setState({ phase: 'idle', kind: null }))
    expect(run.disabled).toBe(false)
  })

  it('needs two lights to stack', () => {
    render(<OperationPanel />)
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    const run = screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement
    expect(run.disabled).toBe(true)
    act(() => {
      useFileListStore.setState({
        workspace: seed([['light', '/sky/lights/2.raf']]),
      })
    })
    expect(run.disabled).toBe(false)
  })

  it('stacks with the alignment and rejection choices on screen', async () => {
    useFileListStore.setState({ workspace: seed([['light', '/sky/lights/2.raf']]) })
    render(<OperationPanel />)
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    fireEvent.change(screen.getByLabelText('Alignment model'), { target: { value: 'homography' } })
    fireEvent.change(screen.getByLabelText('Resampling'), { target: { value: 'lanczos3' } })
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'sigma' } })
    fireEvent.change(screen.getByLabelText('Reject high (σ)'), { target: { value: '2.5' } })
    fireEvent.click(screen.getByRole('button', { name: 'Run' }))

    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0]).toMatchObject({
      kind: 'stacking',
      lights: ['/sky/lights/1.raf', '/sky/lights/2.raf'],
      stacking: {
        model: 'homography',
        resampler: 'lanczos3',
        combine: 'sigma',
        low: 3,
        high: 2.5,
        reference_index: 0,
      },
    })
  })

  it('skips poor frames unless told not to', async () => {
    useFileListStore.setState({ workspace: seed([['light', '/sky/lights/2.raf']]) })
    render(<OperationPanel />)
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    const toggle = screen.getByRole('switch', { name: 'Skip poor frames' }) as HTMLInputElement
    expect(toggle.checked).toBe(true)
    fireEvent.click(toggle)
    expect(toggle.checked).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: 'Run' }))
    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0]).toMatchObject({
      stacking: { skip_poor_frames: false },
    })
  })

  it('hides the rejection limits when the frames are only averaged', () => {
    render(<OperationPanel />)
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    expect(screen.getByLabelText('Reject low (σ)')).toBeTruthy()
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'average' } })
    expect(screen.queryByLabelText('Reject low (σ)')).toBeNull()
  })

  it('offers the three ways to put star trails together', () => {
    render(<OperationPanel />)
    const options = screen.getAllByRole('option').map((option) => option.textContent)
    expect(options).toEqual(expect.arrayContaining(['Lighten', 'Comet', 'Gap Filling']))
    const comet = screen.getByRole('option', { name: 'Comet' }) as HTMLOptionElement
    expect(comet.disabled).toBe(false)
  })

  it('asks for the tail length only for a comet and sends it', async () => {
    render(<OperationPanel />)
    expect(screen.queryByLabelText('Tail length (frames)')).toBeNull()
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'comet' } })
    fireEvent.change(screen.getByLabelText('Tail length (frames)'), { target: { value: '25' } })
    fireEvent.click(screen.getByRole('button', { name: 'Run' }))
    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0]).toMatchObject({
      kind: 'trails',
      trails: { method: 'comet', tail_frames: 25, reverse_order: false },
    })
  })

  it('offers the reverse order only for a comet and sends it', async () => {
    render(<OperationPanel />)
    expect(screen.queryByRole('switch', { name: 'Reverse order' })).toBeNull()
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'comet' } })
    fireEvent.click(screen.getByRole('switch', { name: 'Reverse order' }))
    fireEvent.click(screen.getByRole('button', { name: 'Run' }))
    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0]).toMatchObject({
      trails: { method: 'comet', reverse_order: true },
    })
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'lighten' } })
    expect(screen.queryByRole('switch', { name: 'Reverse order' })).toBeNull()
  })

  it('explains gap filling and needs two lights for it', () => {
    render(<OperationPanel />)
    fireEvent.change(screen.getByLabelText('Combine method'), { target: { value: 'gap_filling' } })
    expect(screen.getByText(/Fills the gaps between the trails/)).toBeTruthy()
    const run = screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement
    expect(run.disabled).toBe(true)
    act(() => {
      useFileListStore.setState({ workspace: seed([['light', '/sky/lights/2.raf']]) })
    })
    expect(run.disabled).toBe(false)
  })
})

describe('ground masking', () => {
  const ground = () => screen.getByRole('switch', { name: 'Stack ground separately' }) as HTMLInputElement
  const masking = () => screen.getByRole('switch', { name: 'Ground masking' }) as HTMLInputElement

  it('is the same in both modes, and separate stacking needs masking on', () => {
    render(<OperationPanel />)
    expect(masking().checked).toBe(false)
    expect(ground().disabled).toBe(true)
    fireEvent.click(masking())
    expect(ground().disabled).toBe(false)
    fireEvent.click(ground())
    expect(useOperationStore.getState()).toMatchObject({ groundMasking: true, groundSeparate: true })
    fireEvent.click(screen.getByRole('radio', { name: 'Stacking' }))
    expect(masking().checked).toBe(true)
    expect(ground().checked).toBe(true)
    fireEvent.click(masking())
    expect(ground().checked).toBe(false)
  })

  it('does not run until the mask is applied, and says so', () => {
    render(<OperationPanel />)
    fireEvent.click(masking())
    expect((screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement).disabled).toBe(true)
  })

  it('runs with the mask of the session once it is applied', async () => {
    const workspace = seed([])
    const sessionId = workspace.sessions[0]!.id
    useFileListStore.setState({ workspace })
    useMaskStore.setState({
      entries: {
        [sessionId]: {
          state: {
            phase: 'applied',
            width: 2,
            height: 2,
            strokes: [],
            region: null,
            contours: [],
            offset: 0,
            feather: 4,
            shaped: null,
            mask: new Uint8Array([255, 255, 0, 0]),
          },
          history: { past: [], future: [] },
        },
      },
    })
    vi.mocked(skyStarCount).mockResolvedValue(50)
    render(<OperationPanel />)
    fireEvent.click(masking())
    fireEvent.click(ground())
    const run = screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement
    expect(run.disabled).toBe(false)
    fireEvent.click(run)
    await waitFor(() => expect(vi.mocked(startJob)).toHaveBeenCalledTimes(1))
    expect(vi.mocked(startJob).mock.calls[0]![0].masking).toEqual({
      session_id: sessionId,
      ground: 'separate',
    })
  })
})

describe('while files are being added', () => {
  it('leaves the operation view inert until they are in', () => {
    const { container } = render(<OperationPanel />)
    const body = () => container.querySelector('[inert]')
    expect(body()).toBeNull()
    act(() => useFileListStore.setState({ scanning: 1 }))
    expect(body()).not.toBeNull()
    act(() => useFileListStore.setState({ scanning: 0, importProgress: { done: 1, total: 2 } }))
    expect(body()).not.toBeNull()
    act(() => useFileListStore.setState({ importProgress: null, pendingDrop: { paths: ['/a'], target: { sessionId: 's', role: 'light' } } }))
    expect(body()).not.toBeNull()
    act(() => useFileListStore.setState({ pendingDrop: null }))
    expect(body()).toBeNull()
  })

  it('is inert while a job runs', () => {
    const { container } = render(<OperationPanel />)
    act(() => useJobStore.setState({ phase: 'running', kind: 'trails' }))
    expect(container.querySelector('[inert]')).not.toBeNull()
    act(() => useJobStore.setState({ phase: 'idle', kind: null }))
    expect(container.querySelector('[inert]')).toBeNull()
  })
})

describe('general and calibration options', () => {
  it('lets the device be chosen in the settings, but not a GPU that was not found', async () => {
    await useSettingsStore.getState().detectGpu()
    useSettingsStore.setState({ dialogOpen: true })
    render(<SettingsDialog />)
    expect(screen.getByText('GPU: none was found')).toBeTruthy()
    expect((screen.getByRole('radio', { name: 'GPU' }) as HTMLButtonElement).disabled).toBe(true)
    expect(useSettingsStore.getState().device).toBe('cpu')
  })

  it('offers the GPU that was found, chosen from the start', async () => {
    vi.mocked(gpuInfo).mockResolvedValue({ name: 'Test GPU', backend: 'Metal', shares_memory: true })
    await useSettingsStore.getState().detectGpu()
    expect(useSettingsStore.getState().device).toBe('gpu')
    useSettingsStore.setState({ dialogOpen: true })
    render(<SettingsDialog />)
    expect(screen.getByText('GPU: Test GPU (Metal)')).toBeTruthy()
    const gpu = screen.getByRole('radio', { name: 'GPU' }) as HTMLButtonElement
    expect(gpu.disabled).toBe(false)
    fireEvent.click(screen.getByRole('radio', { name: 'CPU' }))
    expect(useSettingsStore.getState().device).toBe('cpu')
    fireEvent.click(gpu)
    expect(useSettingsStore.getState().device).toBe('gpu')
  })

  it('has no General section in the operation panel any more', () => {
    render(<OperationPanel />)
    expect(screen.queryByText('General')).toBeNull()
    expect(screen.queryByRole('radio', { name: 'GPU' })).toBeNull()
    expect(screen.queryByRole('slider')).toBeNull()
  })

  it('chooses how the frames of the masters are read', () => {
    useSettingsStore.setState({ readMode: 'exact' })
    render(<OperationPanel />)
    expect(screen.getByRole('radio', { name: 'Exact' }).getAttribute('aria-checked')).toBe('true')
    fireEvent.click(screen.getByRole('radio', { name: 'Fast' }))
    expect(useSettingsStore.getState().readMode).toBe('fast')
  })


  it('changes the memory budget in the settings', () => {
    useSettingsStore.setState({ dialogOpen: true })
    render(<SettingsDialog />)
    const slider = screen.getByRole('slider')
    act(() => void fireEvent.change(slider, { target: { value: '65' } }))
    expect(useSettingsStore.getState().memoryPercent).toBe(65)
  })
})
