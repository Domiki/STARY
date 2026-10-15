import { describe, expect, it } from 'vitest'

import { matchLanguage, resolveLanguage } from './resolve-language'

const available = ['en', 'ko', 'pt-BR']

describe('matchLanguage', () => {
  it('matches a regional system language to its base', () => {
    expect(matchLanguage(['ko-KR'], available)).toBe('ko')
  })

  it('prefers an exact regional match', () => {
    expect(matchLanguage(['pt-BR'], available)).toBe('pt-BR')
  })

  it('walks the candidate list in order', () => {
    expect(matchLanguage(['fr-FR', 'ko-KR'], available)).toBe('ko')
  })

  it('returns null when nothing matches', () => {
    expect(matchLanguage(['fr-FR'], available)).toBeNull()
  })
})

describe('resolveLanguage', () => {
  it('follows the system language by default', () => {
    expect(resolveLanguage('system', ['ko-KR'], available)).toBe('ko')
  })

  it('falls back to English for unsupported system languages', () => {
    expect(resolveLanguage('system', ['fr-FR'], available)).toBe('en')
  })

  it('uses an explicit preference over the system language', () => {
    expect(resolveLanguage('en', ['ko-KR'], available)).toBe('en')
  })

  it('ignores a stored preference that no longer exists', () => {
    expect(resolveLanguage('xx', ['ko-KR'], available)).toBe('ko')
  })
})
