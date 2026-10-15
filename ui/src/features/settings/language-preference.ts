import { SYSTEM_PREFERENCE } from '../../i18n/resolve-language'

const STORAGE_KEY = 'stary.language'

export function loadLanguagePreference(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? SYSTEM_PREFERENCE
  } catch {
    return SYSTEM_PREFERENCE
  }
}

export function saveLanguagePreference(preference: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, preference)
  } catch {
    // storage can be unavailable; the preference then lasts for this run only
  }
}
