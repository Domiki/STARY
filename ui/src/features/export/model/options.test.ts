import { describe, expect, it } from 'vitest'

import {
  type ExportOptions,
  choosesTone,
  defaultOptions,
  depthsOf,
  losesDetail,
  requestFor,
  settled,
  suggestedName,
} from './options'

const base: ExportOptions = { container: 'tiff', depth: 'sixteen', space: 'srgb', tone: 'display', quality: 90 }

describe('what a file can be', () => {
  it('offers the depths a format has', () => {
    expect(depthsOf('tiff')).toEqual(['eight', 'sixteen', 'float'])
    expect(depthsOf('png')).toEqual(['eight', 'sixteen'])
    expect(depthsOf('jpeg')).toEqual(['eight'])
    expect(depthsOf('fits')).toEqual(['sixteen', 'float'])
  })

  it('moves to a depth the new format has, keeping 16 bits where it can', () => {
    expect(settled({ ...base, container: 'jpeg' }).depth).toBe('eight')
    expect(settled({ ...base, container: 'png', depth: 'float' }).depth).toBe('sixteen')
    expect(settled({ ...base, container: 'fits', depth: 'eight' }).depth).toBe('sixteen')
    expect(settled(base).depth).toBe('sixteen')
  })

  it('saves floating point values linear', () => {
    expect(settled({ ...base, depth: 'float' }).tone).toBe('linear')
    expect(choosesTone({ container: 'tiff', depth: 'float' })).toBe(false)
    expect(choosesTone({ container: 'tiff', depth: 'sixteen' })).toBe(true)
  })

  it('keeps the colors and the linear values in FITS', () => {
    const fits = settled({ ...base, container: 'fits' })
    expect(fits).toMatchObject({ space: null, tone: 'linear' })
    expect(choosesTone(fits)).toBe(false)
  })

  it('says when the file holds less than the result', () => {
    expect(losesDetail({ container: 'tiff', depth: 'float' })).toBe(false)
    expect(losesDetail({ container: 'tiff', depth: 'sixteen' })).toBe(true)
    expect(losesDetail({ container: 'fits', depth: 'float' })).toBe(false)
    expect(losesDetail({ container: 'jpeg', depth: 'eight' })).toBe(true)
  })
})

describe('the starting choices', () => {
  it('stretches a stack and encodes anything else', () => {
    expect(defaultOptions('M16_stack_37f.tif').tone).toBe('stretch')
    expect(defaultOptions('DSCF2061_lighten_12f.tif').tone).toBe('display')
    expect(defaultOptions('x.tif')).toMatchObject({ container: 'tiff', depth: 'sixteen', space: 'srgb' })
  })

  it('suggests a name that says the colors and the bits', () => {
    expect(suggestedName('M16_stack_37f.tif', base)).toBe('M16_stack_37f_srgb16.tif')
    expect(suggestedName('a.b.tif', { ...base, container: 'png', depth: 'eight', space: 'display_p3' })).toBe('a.b_p38.png')
    expect(suggestedName('r.tif', { ...base, container: 'fits', depth: 'float', space: null })).toBe('r_src32.fits')
    expect(suggestedName('r.tif', { ...base, container: 'jpeg', depth: 'eight', space: 'adobe_rgb' })).toBe('r_adobe8.jpg')
  })

  it('makes the request of the backend', () => {
    expect(requestFor('/a/r.tif', '/b/r.png', { ...base, container: 'png' })).toEqual({
      source: '/a/r.tif',
      target: '/b/r.png',
      container: 'png',
      depth: 'sixteen',
      space: 'srgb',
      tone: 'display',
      quality: 90,
    })
  })
})
