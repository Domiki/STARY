import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import { availableLanguages } from '../../../i18n/locales'
import { SYSTEM_PREFERENCE } from '../../../i18n/resolve-language'
import { IconButton } from '../../../shared/IconButton'
import { CloseIcon } from '../../../shared/icons'
import { useSettingsStore } from '../store'

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-2 border-t border-twilight/60 pt-3">
      <h3 className="text-xs tracking-wide text-mist">{title}</h3>
      {children}
    </section>
  )
}

export function SettingsDialog() {
  const { t } = useTranslation()
  const settings = useSettingsStore()
  if (!settings.dialogOpen) return null

  return (
    <div
      className="fixed inset-0 z-20 flex items-center justify-center bg-black/50"
      onClick={() => settings.setDialogOpen(false)}
    >
      <div
        role="dialog"
        aria-modal
        aria-label={t('settings.title')}
        className="max-h-[85vh] w-[28rem] space-y-4 overflow-auto rounded-lg border border-twilight bg-midnight p-5 shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2>{t('settings.title')}</h2>
          <IconButton label={t('settings.close')} onClick={() => settings.setDialogOpen(false)}>
            <CloseIcon />
          </IconButton>
        </div>

        <Section title={t('settings.language')}>
          <select
            value={settings.languagePreference}
            onChange={(event) => settings.setLanguagePreference(event.target.value)}
            className="w-full rounded border border-twilight bg-midnight px-2 py-1 text-moon"
          >
            <option value={SYSTEM_PREFERENCE}>{t('settings.systemDefault')}</option>
            {availableLanguages.map(({ code, name }) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </Section>
      </div>
    </div>
  )
}
