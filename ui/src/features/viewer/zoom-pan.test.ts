import { describe, expect, it } from 'vitest'

import { FIT, clampZoom, fitScale, zoomAround } from './zoom-pan'

describe('zoom-pan', () => {
  it('fits the image inside the container', () => {
    expect(fitScale({ width: 800, height: 400 }, { width: 400, height: 400 })).toBe(1)
    expect(fitScale({ width: 200, height: 400 }, { width: 400, height: 400 })).toBe(0.5)
  })

  it('clamps zoom', () => {
    expect(clampZoom(1000)).toBe(32)
    expect(clampZoom(0.01)).toBe(0.5)
  })

  it('keeps the point under the pointer fixed', () => {
    const base = 1
    const pointer = { x: 100, y: 50 }
    const next = zoomAround(FIT, 2, base, pointer.x, pointer.y)
    const imagePointBefore = (pointer.x - FIT.panX) / (base * FIT.zoom)
    const imagePointAfter = (pointer.x - next.panX) / (base * next.zoom)
    expect(imagePointAfter).toBeCloseTo(imagePointBefore)
  })

  it('does not move when zooming at the centre from the fit state', () => {
    const next = zoomAround(FIT, 2, 1, 0, 0)
    expect(next.panX).toBeCloseTo(0)
    expect(next.panY).toBeCloseTo(0)
  })
})
