import { describe, expect, it } from 'vitest'

import { addDab } from './brush'
import { hasGround, paintInto, skyShare } from './paint'

const WIDTH = 20
const HEIGHT = 20
const whole = { left: 0, top: 0, right: WIDTH, bottom: HEIGHT }

function stroke(hardness: number) {
  const coverage = new Float32Array(WIDTH * HEIGHT)
  addDab(coverage, WIDTH, HEIGHT, 10, 10, { radius: 6, hardness })
  return coverage
}

describe('painting a mask', () => {
  it('sets a hard stroke of sky to full sky and leaves the rest', () => {
    const base = new Uint8Array(WIDTH * HEIGHT)
    const out = base.slice()
    paintInto(base, stroke(1), 'sky', whole, WIDTH, out)
    expect(out[10 * WIDTH + 10]).toBe(255)
    expect(out[0]).toBe(0)
  })

  it('sets a stroke of ground to ground', () => {
    const base = new Uint8Array(WIDTH * HEIGHT).fill(255)
    const out = base.slice()
    paintInto(base, stroke(1), 'ground', whole, WIDTH, out)
    expect(out[10 * WIDTH + 10]).toBe(0)
    expect(out[0]).toBe(255)
  })

  it('leaves in-between values at the soft edge of a brush', () => {
    const base = new Uint8Array(WIDTH * HEIGHT)
    const out = base.slice()
    paintInto(base, stroke(0), 'sky', whole, WIDTH, out)
    const edge = out[10 * WIDTH + 14]!
    expect(edge).toBeGreaterThan(0)
    expect(edge).toBeLessThan(255)
    expect(out[10 * WIDTH + 10]!).toBeGreaterThan(235)
  })

  it('blends with what is there: half coverage over half sky', () => {
    const base = new Uint8Array(WIDTH * HEIGHT).fill(128)
    const coverage = new Float32Array(WIDTH * HEIGHT).fill(0.5)
    const out = base.slice()
    paintInto(base, coverage, 'sky', whole, WIDTH, out)
    expect(out[0]).toBe(Math.round(128 + 127 * 0.5))
    paintInto(base, coverage, 'ground', whole, WIDTH, out)
    expect(out[0]).toBe(64)
  })

  it('writes only inside the rectangle', () => {
    const base = new Uint8Array(WIDTH * HEIGHT)
    const out = base.slice()
    paintInto(base, new Float32Array(WIDTH * HEIGHT).fill(1), 'sky', { left: 2, top: 2, right: 4, bottom: 4 }, WIDTH, out)
    expect(out[2 * WIDTH + 2]).toBe(255)
    expect(out[5 * WIDTH + 5]).toBe(0)
  })
})

describe('what a mask holds', () => {
  it('counts the share of sky and sees ground', () => {
    const mask = new Uint8Array([255, 255, 0, 100])
    expect(skyShare(mask)).toBe(0.5)
    expect(hasGround(mask)).toBe(true)
    expect(hasGround(new Uint8Array([255, 255]))).toBe(false)
  })
})
