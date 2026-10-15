import { describe, expect, it } from 'vitest'

import { flattenKeys } from './flatten-keys'
import { localeFiles } from './locales'

const reference = new Set(flattenKeys(localeFiles.en ?? {}))
const others = Object.entries(localeFiles).filter(([code]) => code !== 'en')

describe('locale files', () => {
  it('include English as the fallback language', () => {
    expect(localeFiles.en).toBeDefined()
  })

  it.each(Object.keys(localeFiles))('%s has a language name', (code) => {
    expect(localeFiles[code]?.meta.name.length).toBeGreaterThan(0)
  })

  it.each(others)('%s has no keys that English does not define', (_code, file) => {
    const unknown = flattenKeys(file).filter((key) => !reference.has(key))
    expect(unknown).toEqual([])
  })

  it.each(others)('%s reports missing keys', (code, file) => {
    const present = new Set(flattenKeys(file))
    const missing = [...reference].filter((key) => !present.has(key))
    if (missing.length > 0) console.warn(`${code} is missing ${missing.length} key(s)`, missing)
    expect(true).toBe(true)
  })
})
