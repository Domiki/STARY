import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

import { availableCodes, resources } from './locales'
import { FALLBACK_LANGUAGE, resolveLanguage } from './resolve-language'

const systemLanguages = () => (navigator.languages.length ? navigator.languages : [navigator.language])

export const resolveFor = (preference: string) =>
  resolveLanguage(preference, systemLanguages(), availableCodes)

export function initI18n(preference: string) {
  return i18n.use(initReactI18next).init({
    resources,
    lng: resolveFor(preference),
    fallbackLng: FALLBACK_LANGUAGE,
    interpolation: { escapeValue: false },
    saveMissing: import.meta.env.DEV,
    missingKeyHandler: (_languages, _namespace, key) => console.warn(`missing i18n key: ${key}`),
  })
}

export { i18n }
