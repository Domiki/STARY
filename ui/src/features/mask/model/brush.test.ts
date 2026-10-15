import { describe, expect, it } from 'vitest'

import { addDab, coverageAt, dabBounds, pointsBetween, unionOf } from './brush'

describe('brush coverage', () => {
  it('is full inside the hard core and nothing outside the radius', () => {
    const brush = { radius: 10, hardness: 0.5 }
    expect(coverageAt(0, brush)).toBe(1)
    expect(coverageAt(5, brush)).toBe(1)
    expect(coverageAt(10, brush)).toBe(0)
    expect(coverageAt(14, brush)).toBe(0)
  })

  it('fades smoothly between the core and the edge', () => {
    const brush = { radius: 10, hardness: 0.2 }
    const values = [3, 5, 7, 9].map((distance) => coverageAt(distance, brush))
    expect(values.every((value, at) => at === 0 || value < values[at - 1]!)).toBe(true)
    expect(values[0]).toBeLessThan(1)
    expect(values[3]).toBeGreaterThan(0)
  })

  it('has a hard edge at full hardness and a fully soft one at zero hardness', () => {
    expect(coverageAt(9.9, { radius: 10, hardness: 1 })).toBe(1)
    const soft = { radius: 10, hardness: 0 }
    expect(coverageAt(0, soft)).toBe(1)
    expect(coverageAt(5, soft)).toBeCloseTo(0.5, 5)
  })
})

describe('dabs', () => {
  it('only touch the part of the picture they are on', () => {
    expect(dabBounds(2, 2, 5, 100, 100)).toEqual({ left: 0, top: 0, right: 7, bottom: 7 })
    expect(dabBounds(-20, 50, 5, 100, 100)).toBeNull()
    expect(dabBounds(99, 99, 5, 100, 100)).toEqual({ left: 94, top: 94, right: 100, bottom: 100 })
  })

  it('keep the larger coverage where they overlap', () => {
    const coverage = new Float32Array(20 * 20)
    const brush = { radius: 4, hardness: 0.3 }
    addDab(coverage, 20, 20, 8, 10, brush)
    const once = coverage[10 * 20 + 10]!
    addDab(coverage, 20, 20, 9, 10, brush)
    addDab(coverage, 20, 20, 8, 10, brush)
    expect(coverage[10 * 20 + 10]!).toBeGreaterThanOrEqual(once)
    expect(Math.max(...coverage)).toBeLessThanOrEqual(1)
    expect(coverage[0]).toBe(0)
  })

  it('are joined by points close enough to run together', () => {
    const points = pointsBetween([0, 0], [100, 0], 20)
    expect(points.at(-1)).toEqual([100, 0])
    const gaps = points.map((point, at) => point[0] - (points[at - 1]?.[0] ?? 0))
    expect(Math.max(...gaps)).toBeLessThanOrEqual(3.01)
    expect(pointsBetween([5, 5], [5, 5], 10)).toEqual([[5, 5]])
  })

  it('have bounds that can be joined', () => {
    const a = { left: 0, top: 0, right: 5, bottom: 5 }
    const b = { left: 3, top: 4, right: 9, bottom: 7 }
    expect(unionOf(a, b)).toEqual({ left: 0, top: 0, right: 9, bottom: 7 })
    expect(unionOf(null, b)).toEqual(b)
    expect(unionOf(null, null)).toBeNull()
  })
})
