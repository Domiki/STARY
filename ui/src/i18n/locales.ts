type LocaleFile = { meta: { name: string } } & Record<string, unknown>

const files = import.meta.glob<LocaleFile>('../locales/*.json', {
  eager: true,
  import: 'default',
})

const codeOf = (path: string) => path.split('/').pop()?.replace(/\.json$/, '') ?? path

export const localeFiles: Record<string, LocaleFile> = Object.fromEntries(
  Object.entries(files).map(([path, file]) => [codeOf(path), file]),
)

export const availableLanguages = Object.entries(localeFiles)
  .map(([code, file]) => ({ code, name: file.meta.name }))
  .sort((a, b) => a.code.localeCompare(b.code))

export const availableCodes = availableLanguages.map(({ code }) => code)

export const resources = Object.fromEntries(
  Object.entries(localeFiles).map(([code, file]) => [code, { translation: file }]),
)
