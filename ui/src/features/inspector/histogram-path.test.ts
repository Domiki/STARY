import { describe, expect, it } from 'vitest'

import { CHART_HEIGHT, areaPath, histogramPeak } from './histogram-path'

describe('histogram path', () => {
  it('ignores the clipped end bins when finding the peak', () => {
    expect(histogramPeak([[1000, 5, 7, 1000]])).toBe(7)
  })

  it('draws a closed path with one point per bin', () => {
    const path = areaPath([0, 5, 10, 0], 10, false)
    expect(path.startsWith(`M0,${CHART_HEIGHT}`)).toBe(true)
    expect(path.endsWith('Z')).toBe(true)
    expect(path.match(/L/g)).toHaveLength(5)
  })

  it('clamps values above the peak to the chart top', () => {
    expect(areaPath([0, 100, 0], 10, false)).toContain('1,0.0')
  })

  it('handles an empty histogram', () => {
    expect(areaPath([0, 0], 0, true)).toContain(`${CHART_HEIGHT}`)
  })
})
