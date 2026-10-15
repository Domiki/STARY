import { beforeEach, describe, expect, it, vi } from 'vitest'

import { clearMask, detectSky, setMask, shapeSky } from '../../ipc/commands'
import type { SkyDetection } from '../../ipc/types'
import { EMPTY_MASK, maskOf, useMaskStore } from './store'

vi.mock('../../ipc/commands', () => ({
  detectSky: vi.fn(),
  shapeSky: vi.fn(),
  setMask: vi.fn(async () => {}),
  clearMask: vi.fn(async () => {}),
}))

const WIDTH = 4
const HEIGHT = 3
const target = { sessionId: 's1', path: '/sky/1.raf' }

const detection = (sky: number[]): SkyDetection => ({
  width: WIDTH,
  height: HEIGHT,
  region: new Uint8Array(sky),
  contours: [[[0, 1], [4, 1]]],
  skyFraction: 0.5,
})

const sample = () => new Array<number>(WIDTH * HEIGHT).fill(255)

beforeEach(() => {
  vi.mocked(detectSky).mockReset().mockImplementation(async () => detection(sample()))
  vi.mocked(shapeSky)
    .mockReset()
    .mockImplementation(async (region) => region.bytes.slice())
  vi.mocked(setMask).mockClear()
  vi.mocked(clearMask).mockClear()
  useMaskStore.setState({ entries: {}, busy: false, tool: 'hand', confirming: null })
})

const state = () => maskOf(useMaskStore.getState().entries, 's1')

describe('finding the sky', () => {
  it('shows the contour at once when there is no mask yet', async () => {
    await useMaskStore.getState().requestDetect(target)
    expect(useMaskStore.getState().confirming).toBeNull()
    expect(state()).toMatchObject({ phase: 'contour', width: WIDTH, height: HEIGHT, offset: 0 })
    expect(state().contours).toHaveLength(1)
    expect(state().shaped).not.toBeNull()
  })

  it('asks before it erases a mask that is there, and keeps it when the answer is no', async () => {
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().apply('s1')
    const before = state()
    await useMaskStore.getState().requestDetect(target)
    expect(useMaskStore.getState().confirming).toEqual(target)
    expect(detectSky).toHaveBeenCalledTimes(1)
    useMaskStore.getState().cancelConfirm()
    expect(state()).toBe(before)
  })

  it('finds the sky again after the question is answered, and the erased mask can be got back', async () => {
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().apply('s1')
    const applied = state()
    await useMaskStore.getState().detect(target)
    expect(state().phase).toBe('contour')
    await useMaskStore.getState().undo('s1')
    expect(state()).toBe(applied)
  })
})

describe('guiding the search', () => {
  it('looks again with every stroke so far', async () => {
    await useMaskStore.getState().detect(target)
    const one = { paint: 'ground' as const, radius: 5, points: [[1, 1]] as [number, number][] }
    const two = { paint: 'refine' as const, radius: 9, points: [[2, 2]] as [number, number][] }
    await useMaskStore.getState().guide(target, one)
    await useMaskStore.getState().guide(target, two)
    expect(vi.mocked(detectSky).mock.calls.at(-1)?.[1]).toEqual([one, two])
    expect(state().strokes).toEqual([one, two])
  })

  it('does nothing once the mask is applied', async () => {
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().apply('s1')
    const calls = vi.mocked(detectSky).mock.calls.length
    await useMaskStore.getState().guide(target, { paint: 'sky', radius: 3, points: [[0, 0]] })
    expect(vi.mocked(detectSky).mock.calls.length).toBe(calls)
  })
})

describe('shaping and applying', () => {
  it('shapes the region again with the offset and the feather', async () => {
    await useMaskStore.getState().detect(target)
    await useMaskStore.getState().shape('s1', { offset: 3, feather: 9 })
    expect(vi.mocked(shapeSky).mock.calls.at(-1)?.[1]).toBe(3)
    expect(vi.mocked(shapeSky).mock.calls.at(-1)?.[2]).toBe(9)
    expect(state()).toMatchObject({ offset: 3, feather: 9 })
  })

  it('does not shape once for every step of a slider that moves quickly', async () => {
    await useMaskStore.getState().detect(target)
    vi.mocked(shapeSky).mockClear()
    await Promise.all([1, 2, 3, 4, 5].map((offset) => useMaskStore.getState().shape('s1', { offset })))
    expect(vi.mocked(shapeSky).mock.calls.length).toBeLessThan(5)
    expect(state().offset).toBe(5)
    expect(vi.mocked(shapeSky).mock.calls.at(-1)?.[1]).toBe(5)
  })

  it('makes the shaped mask the mask and ends the contour phase', async () => {
    await useMaskStore.getState().detect(target)
    const shaped = state().shaped
    useMaskStore.getState().apply('s1')
    expect(state()).toMatchObject({ phase: 'applied', mask: shaped, region: null })
  })

  it('changes the tool when the refine brush is no longer there', async () => {
    useMaskStore.setState({ tool: 'refine' })
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().apply('s1')
    expect(useMaskStore.getState().tool).toBe('hand')
  })
})

describe('painting and history', () => {
  it('takes a painted mask only once the mask is applied', async () => {
    useMaskStore.getState().commitPaint('s1', new Uint8Array(WIDTH * HEIGHT))
    expect(state()).toBe(EMPTY_MASK)
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().apply('s1')
    const painted = new Uint8Array(WIDTH * HEIGHT).fill(7)
    useMaskStore.getState().commitPaint('s1', painted)
    expect(state().mask).toBe(painted)
    await useMaskStore.getState().undo('s1')
    expect(state().mask).not.toBe(painted)
    await useMaskStore.getState().redo('s1')
    expect(state().mask).toBe(painted)
  })

  it('clears the mask here and in the backend', async () => {
    await useMaskStore.getState().detect(target)
    useMaskStore.getState().clear('s1')
    expect(state().phase).toBe('none')
    expect(clearMask).toHaveBeenCalledWith('s1')
  })
})

describe('handing the mask to the backend', () => {
  it('sends an applied mask and nothing else', async () => {
    expect(await useMaskStore.getState().publish('s1')).toBe(false)
    await useMaskStore.getState().detect(target)
    expect(await useMaskStore.getState().publish('s1')).toBe(false)
    useMaskStore.getState().apply('s1')
    expect(await useMaskStore.getState().publish('s1')).toBe(true)
    expect(setMask).toHaveBeenCalledWith('s1', expect.objectContaining({ width: WIDTH, height: HEIGHT }))
  })
})
