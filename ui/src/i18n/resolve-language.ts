export const SYSTEM_PREFERENCE = 'system'
export const FALLBACK_LANGUAGE = 'en'

const baseOf = (code: string) => code.toLowerCase().split('-')[0] ?? ''

function matchOne(candidate: string, available: readonly string[]): string | null {
  const exact = available.find((code) => code.toLowerCase() === candidate.toLowerCase())
  const sameBase = available.find((code) => baseOf(code) === baseOf(candidate))
  return exact ?? sameBase ?? null
}

export function matchLanguage(
  candidates: readonly string[],
  available: readonly string[],
): string | null {
  return candidates.reduce<string | null>(
    (found, candidate) => found ?? matchOne(candidate, available),
    null,
  )
}

export function resolveLanguage(
  preference: string,
  systemLanguages: readonly string[],
  available: readonly string[],
): string {
  const explicit = preference === SYSTEM_PREFERENCE ? null : matchOne(preference, available)
  return explicit ?? matchLanguage(systemLanguages, available) ?? FALLBACK_LANGUAGE
}
