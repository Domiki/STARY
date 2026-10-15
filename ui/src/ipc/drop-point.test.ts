import { describe, expect, it } from 'vitest'

import { detectPlatform, dropPointToCss } from './drop-point'

describe('dropPointToCss', () => {
  it('keeps Linux positions as they are', () => {
    expect(dropPointToCss({ x: 120, y: 340 }, 'linux', 2, 28)).toEqual({ x: 120, y: 340 })
  })

  it('removes the title bar height on macOS and keeps x', () => {
    expect(dropPointToCss({ x: 120, y: 140 }, 'macos', 2, 28)).toEqual({ x: 120, y: 112 })
  })

  it('does not shift macOS positions when there is no title bar (full screen)', () => {
    expect(dropPointToCss({ x: 5, y: 50 }, 'macos', 2, 0)).toEqual({ x: 5, y: 50 })
  })

  it('converts Windows physical pixels to CSS pixels', () => {
    expect(dropPointToCss({ x: 300, y: 450 }, 'windows', 1.5, 0)).toEqual({ x: 200, y: 300 })
  })
})

describe('detectPlatform', () => {
  it('recognises the three webview user agents', () => {
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit')).toBe('macos')
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64) Edg/120')).toBe('windows')
    expect(detectPlatform('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit')).toBe('linux')
  })
})
