// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { createRef } from 'react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../../../i18n'
import { detectSky, shapeSky, skyStarCount } from '../../../ipc/commands'
import { placeInfo } from '../../file-list/model/placement'
import { info } from '../../file-list/model/test-fixtures'
import { emptyWorkspace } from '../../file-list/model/workspace'
import { useFileListStore } from '../../file-list/store'
import { useJobStore } from '../../operation/job-store'
import { DEFAULT_STACKING, DEFAULT_TRAILS, useOperationStore } from '../../operation/store'
import { useSettingsStore } from '../../settings/store'
import { useMaskStore } from '../store'
import { MaskDialogs } from './MaskDialogs'
import { MaskOverlay } from './MaskOverlay'
import { MaskToolbar } from './MaskToolbar'

vi.mock('../../../ipc/commands', () => ({
  detectSky: vi.fn(),
  shapeSky: vi.fn(),
  setMask: vi.fn(async () => {}),
  clearMask: vi.fn(async () => {}),
  skyStarCount: vi.fn(),
  startJob: vi.fn(async () => {}),
  cancelJob: vi.fn(async () => {}),
  probeFiles: vi.fn(),
  expandPaths: vi.fn(),
  pickFiles: vi.fn(),
  listenForDrops: vi.fn(),
  gpuInfo: vi.fn(async () => null),
}))

const WIDTH = 40
const HEIGHT = 30
const target = { sessionId: 's1', path: '/sky/1.jpg' }

beforeAll(async () => {
  await initI18n('en')
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)
})

beforeEach(() => {
  vi.mocked(detectSky).mockReset().mockResolvedValue({
    width: WIDTH,
    height: HEIGHT,
    region: new Uint8Array(WIDTH * HEIGHT).fill(255),
    contours: [],
    skyFraction: 1,
  })
  vi.mocked(shapeSky)
    .mockReset()
    .mockImplementation(async (region) => region.bytes.slice())
  useMaskStore.setState({
    entries: {},
    busy: false,
    tool: 'hand',
    brushRadius: 6,
    hardness: 1,
    confirming: null,
  })
  useJobStore.setState({ phase: 'idle', kind: null, progress: null, outcome: null, skyWarning: null })
})

afterEach(cleanup)

async function appliedMask() {
  await useMaskStore.getState().detect(target)
  useMaskStore.getState().apply('s1')
}

const maskNow = () => useMaskStore.getState().entries.s1?.state.mask

describe('toolbar', () => {
  it('finds the sky and then offers the contour sliders and Apply', async () => {
    render(<MaskToolbar {...target} />)
    expect(screen.queryByRole('button', { name: 'Apply' })).toBeNull()
    expect((screen.getByRole('radio', { name: 'Sky' }) as HTMLButtonElement).disabled).toBe(true)
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Find sky' })))
    expect(detectSky).toHaveBeenCalledWith('/sky/1.jpg', [])
    expect(screen.getByRole('slider', { name: 'Offset' })).toBeTruthy()
    expect(screen.getByRole('slider', { name: 'Feather' })).toBeTruthy()
    expect(screen.getByRole('radio', { name: 'Refine' })).toBeTruthy()
    expect(screen.queryByRole('slider', { name: 'Hardness' })).toBeNull()
    act(() => void fireEvent.click(screen.getByRole('button', { name: 'Apply' })))
    expect(useMaskStore.getState().entries.s1?.state.phase).toBe('applied')
  })

  it('has a hardness for the brush once the mask is applied, and no Refine brush', async () => {
    await appliedMask()
    render(<MaskToolbar {...target} />)
    expect(screen.getByRole('slider', { name: 'Hardness' })).toBeTruthy()
    expect(screen.queryByRole('radio', { name: 'Refine' })).toBeNull()
    act(() => void fireEvent.change(screen.getByRole('slider', { name: 'Hardness' }), { target: { value: '40' } }))
    expect(useMaskStore.getState().hardness).toBeCloseTo(0.4)
  })

  it('undoes and redoes with the buttons', async () => {
    await appliedMask()
    render(<MaskToolbar {...target} />)
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Undo' })))
    expect(useMaskStore.getState().entries.s1?.state.phase).toBe('contour')
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Redo' })))
    expect(useMaskStore.getState().entries.s1?.state.phase).toBe('applied')
  })
})

describe('dialogs', () => {
  it('asks before the sky is found over a mask, and finds it only when told to', async () => {
    await appliedMask()
    render(
      <>
        <MaskToolbar {...target} />
        <MaskDialogs />
      </>,
    )
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Find sky' })))
    expect(screen.getByText('The mask made so far will be erased. Is that all right?')).toBeTruthy()
    expect(detectSky).toHaveBeenCalledTimes(1)
    act(() => void fireEvent.click(screen.getByRole('button', { name: 'Cancel' })))
    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(useMaskStore.getState().entries.s1?.state.phase).toBe('applied')

    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Find sky' })))
    const dialog = screen.getByRole('alertdialog')
    await act(async () => void fireEvent.click(dialog.querySelector('button:last-of-type')!))
    expect(detectSky).toHaveBeenCalledTimes(2)
    expect(useMaskStore.getState().entries.s1?.state.phase).toBe('contour')
  })
})

describe('painting on the overlay', () => {
  const box = { left: 0, top: 0, width: 400, height: 300, right: 400, bottom: 300, x: 0, y: 0, toJSON: () => ({}) }

  function overlay() {
    const container = createRef<HTMLDivElement>()
    const holder = document.createElement('div')
    holder.getBoundingClientRect = () => box
    ;(container as { current: HTMLDivElement | null }).current = holder
    render(
      <MaskOverlay
        {...target}
        width={WIDTH}
        height={HEIGHT}
        viewport={{ scale: 10, panX: 0, panY: 0 }}
        containerRef={container}
      />,
    )
  }

  const brushLayer = () => screen.getByTestId('mask-brush-layer')

  const press = (layer: HTMLElement, x: number, y: number) =>
    fireEvent.pointerDown(layer, { clientX: x, clientY: y, pointerId: 1 })

  it('paints ground with a hard brush into the mask and keeps it as one step', async () => {
    await appliedMask()
    useMaskStore.setState({ tool: 'ground', hardness: 1 })
    overlay()
    const layer = brushLayer()
    press(layer, 200, 150)
    fireEvent.pointerMove(layer, { clientX: 230, clientY: 150, pointerId: 1 })
    fireEvent.pointerUp(layer, { clientX: 230, clientY: 150, pointerId: 1 })
    const mask = maskNow()!
    expect(mask[15 * WIDTH + 20]).toBe(0)
    expect(mask[15 * WIDTH + 23]).toBe(0)
    expect(mask[2 * WIDTH + 2]).toBe(255)
    await act(async () => void (await useMaskStore.getState().undo('s1')))
    expect(maskNow()![15 * WIDTH + 20]).toBe(255)
  })

  it('leaves in-between values at the edge of a soft brush', async () => {
    await appliedMask()
    useMaskStore.setState({ tool: 'ground', hardness: 0, brushRadius: 8 })
    overlay()
    const layer = brushLayer()
    press(layer, 200, 150)
    fireEvent.pointerUp(layer, { clientX: 200, clientY: 150, pointerId: 1 })
    const mask = maskNow()!
    const across = Array.from({ length: 9 }, (_, step) => mask[15 * WIDTH + 20 + step]!)
    expect(across[0]).toBeLessThan(40)
    expect(across.some((value) => value > 20 && value < 235)).toBe(true)
    expect(across.every((value, at) => at === 0 || value >= across[at - 1]!)).toBe(true)
  })

  it('paints sky back into the ground', async () => {
    await appliedMask()
    useMaskStore.getState().commitPaint('s1', new Uint8Array(WIDTH * HEIGHT))
    useMaskStore.setState({ tool: 'sky', hardness: 1 })
    overlay()
    const layer = brushLayer()
    press(layer, 100, 100)
    fireEvent.pointerUp(layer, { clientX: 100, clientY: 100, pointerId: 1 })
    expect(maskNow()![10 * WIDTH + 10]).toBe(255)
    expect(maskNow()![25 * WIDTH + 35]).toBe(0)
  })

  it('guides the search with a stroke while the contour is shown', async () => {
    await useMaskStore.getState().detect(target)
    useMaskStore.setState({ tool: 'refine' })
    overlay()
    const layer = brushLayer()
    press(layer, 200, 150)
    fireEvent.pointerMove(layer, { clientX: 260, clientY: 150, pointerId: 1 })
    await act(async () => void fireEvent.pointerUp(layer, { clientX: 260, clientY: 150, pointerId: 1 }))
    const strokes = vi.mocked(detectSky).mock.calls.at(-1)?.[1]
    expect(strokes).toHaveLength(1)
    expect(strokes?.[0]).toMatchObject({ paint: 'refine', radius: 6 })
    expect(strokes?.[0]?.points[0]).toEqual([20, 15])
  })

  it('has no brush layer for the hand tool', async () => {
    await appliedMask()
    overlay()
    expect(screen.queryByTestId('mask-brush-layer')).toBeNull()
  })
})

describe('running with too few stars in the sky', () => {
  function session() {
    const workspace = placeInfo(emptyWorkspace(), info({ path: '/sky/1.jpg' })).workspace
    placeInfo(workspace, info({ path: '/sky/2.jpg' }))
    useFileListStore.setState({ workspace: placeInfo(workspace, info({ path: '/sky/2.jpg' })).workspace })
    useOperationStore.setState({
      mode: 'stacking',
      trails: DEFAULT_TRAILS,
      stacking: DEFAULT_STACKING,
      groundMasking: true,
      groundSeparate: false,
    })
    useSettingsStore.setState({ device: 'cpu', memoryPercent: 50 })
    return useFileListStore.getState().workspace.sessions[0]!.id
  }

  it('warns first, and runs when told to', async () => {
    const { startJob } = await import('../../../ipc/commands')
    vi.mocked(startJob).mockClear()
    const id = session()
    useMaskStore.setState({
      entries: {
        [id]: {
          state: {
            phase: 'applied',
            width: WIDTH,
            height: HEIGHT,
            strokes: [],
            region: null,
            contours: [],
            offset: 0,
            feather: 4,
            shaped: null,
            mask: new Uint8Array(WIDTH * HEIGHT).fill(255),
          },
          history: { past: [], future: [] },
        },
      },
    })
    vi.mocked(skyStarCount).mockResolvedValue(3)
    render(<MaskDialogs />)
    await act(async () => void (await useJobStore.getState().start('stacking')))
    expect(startJob).not.toHaveBeenCalled()
    expect(screen.getByText(/Only 3 stars were found/)).toBeTruthy()
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Run' })))
    expect(startJob).toHaveBeenCalledTimes(1)
    expect(vi.mocked(startJob).mock.calls[0]?.[0].masking).toEqual({ session_id: id, ground: 'with_sky' })
  })

  it('runs at once when the sky has enough stars', async () => {
    const { startJob } = await import('../../../ipc/commands')
    vi.mocked(startJob).mockClear()
    const id = session()
    useMaskStore.setState({
      entries: {
        [id]: {
          state: {
            phase: 'applied',
            width: WIDTH,
            height: HEIGHT,
            strokes: [],
            region: null,
            contours: [],
            offset: 0,
            feather: 4,
            shaped: null,
            mask: new Uint8Array(WIDTH * HEIGHT).fill(255),
          },
          history: { past: [], future: [] },
        },
      },
    })
    vi.mocked(skyStarCount).mockResolvedValue(80)
    await act(async () => void (await useJobStore.getState().start('stacking')))
    expect(startJob).toHaveBeenCalledTimes(1)
  })
})

describe('failures', () => {
  it('says so when the sky could not be found', async () => {
    vi.mocked(detectSky).mockRejectedValue({ code: 'io', message: 'no such file' })
    render(<MaskToolbar {...target} />)
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Find sky' })))
    expect(screen.getByText('Could not find the sky.')).toBeTruthy()
    expect(useMaskStore.getState().busy).toBe(false)
  })
})
