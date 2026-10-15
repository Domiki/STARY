import { describe, expect, it } from 'vitest'

import { fileNameOf, formatBytes, formatExposure } from './format'

describe('format helpers', () => {
  it('formats byte sizes', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(40 * 1024 * 1024)).toBe('40.0 MB')
  })

  it('formats exposures as seconds or fractions', () => {
    expect(formatExposure(30)).toBe('30s')
    expect(formatExposure(0.004)).toBe('1/250s')
  })

  it('extracts file names from both path styles', () => {
    expect(fileNameOf('/a/b/c.fits')).toBe('c.fits')
    expect(fileNameOf('C:\\a\\b\\c.fits')).toBe('c.fits')
  })
})
