import { type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import { availableLanguages } from '../../../i18n/locales'
import { SYSTEM_PREFERENCE } from '../../../i18n/resolve-language'
import { IconButton } from '../../../shared/IconButton'
import { Toggle } from '../../../shared/Toggle'
import { useUpdateStore } from '../../update/store'
import { CloseIcon } from '../../../shared/icons'
import { type ComputeDevice, useSettingsStore } from '../store'

const DEVICES: ComputeDevice[] = ['cpu', 'gpu']

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
  const { enabled: checkUpdates, setEnabled: setCheckUpdates } = useUpdateStore()
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

        <Section title={t('settings.device.title')}>
          <div role="radiogroup" className="grid grid-cols-2 rounded border border-twilight p-0.5">
            {DEVICES.map((choice) => (
              <button
                key={choice}
                type="button"
                role="radio"
                aria-checked={settings.device === choice}
                disabled={choice === 'gpu' && !settings.gpu}
                onClick={() => settings.setDevice(choice)}
                className={`rounded py-1 disabled:cursor-default disabled:opacity-40 ${settings.device === choice ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
              >
                {t(`settings.device.${choice}`)}
              </button>
            ))}
          </div>
          <ul className="space-y-0.5 text-xs text-mist">
            <li>{t('settings.device.cpuInfo', { count: navigator.hardwareConcurrency })}</li>
            <li>
              {settings.gpu
                ? t('settings.device.gpuInfo', { name: settings.gpu.name, backend: settings.gpu.backend })
                : t(settings.gpu === undefined ? 'settings.device.gpuChecking' : 'settings.device.gpuUnavailable')}
            </li>
          </ul>
        </Section>

        <Section title={t('settings.memory.title')}>
          <label className="block">
            <span className="mb-1 flex justify-end text-xs text-moon">{settings.memoryPercent}%</span>
            <input
              type="range"
              min={10}
              max={80}
              step={5}
              value={settings.memoryPercent}
              onChange={(event) => settings.setMemoryPercent(Number(event.target.value))}
              aria-label={t('settings.memory.title')}
              className="w-full accent-[#d8a48f]"
            />
          </label>
        </Section>

        <Section title={t('settings.updates.title')}>
          <Toggle
            label={t('settings.updates.check')}
            checked={checkUpdates}
            onChange={setCheckUpdates}
          />
        </Section>
      </div>
    </div>
  )
}
