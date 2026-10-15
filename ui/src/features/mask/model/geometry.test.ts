import { describe, expect, it } from 'vitest'

import { toPicturePoint } from './geometry'

const box = { left: 100, top: 50, width: 800, height: 600 }
const picture = { width: 1600, height: 1000 }

describe('toPicturePoint', () => {
  it('puts the middle of the viewer on the middle of the picture', () => {
    expect(toPicturePoint(500, 350, box, { scale: 0.5, panX: 0, panY: 0 }, picture)).toEqual([800, 500])
  })

  it('follows the scale', () => {
    const [x] = toPicturePoint(600, 350, box, { scale: 0.5, panX: 0, panY: 0 }, picture)
    expect(x).toBe(1000)
  })

  it('follows the pan: the picture moved right shows a point further left', () => {
    const [x] = toPicturePoint(500, 350, box, { scale: 1, panX: 40, panY: 0 }, picture)
    expect(x).toBe(760)
  })
})
