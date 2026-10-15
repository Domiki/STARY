import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import type { ExportContainer, ExportDepth, ExportSpace, ExportTone } from '../../../ipc/types'
import { fileNameOf } from '../../../shared/format'
import {
  CONTAINERS,
  SPACES,
  TONES,
  choosesTone,
  depthsOf,
  keepsColors,
  losesDetail,
} from '../model/options'
import { useExportStore } from '../store'

const RADIO = 'rounded px-2 py-1 disabled:opacity-40'

function Choice<T extends string>(props: {
  label: string
  value: T
  choices: { value: T; label: string }[]
  disabled?: boolean
  onChange: (value: T) => void
}) {
  return (
    <div className="space-y-1">
      <span className="block text-xs text-mist">{props.label}</span>
      <div
        role="radiogroup"
        aria-label={props.label}
        className="flex flex-wrap gap-0.5 rounded border border-twilight p-0.5"
      >
        {props.choices.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={props.value === value}
            disabled={props.disabled}
            onClick={() => props.onChange(value)}
            className={`${RADIO} ${props.value === value ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  )
}

export function SaveAsDialog() {
  const { t } = useTranslation()
  const { source, options, busy, failure, change, close, save } = useExportStore()

  useEffect(() => {
    if (!source) return
    const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && close()
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [source, close])

  if (!source) return null
  const tone = choosesTone(options)
  const colors = !keepsColors(options.container)

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/55" onClick={close}>
      <div
        role="dialog"
        aria-modal
        aria-label={t('export.title')}
        className="w-[30rem] space-y-4 rounded-lg border border-twilight bg-midnight p-5 shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div>
          <h2>{t('export.title')}</h2>
          <p className="truncate text-xs text-mist" title={source}>
            {fileNameOf(source)}
          </p>
        </div>

        <Choice<ExportContainer>
          label={t('export.format')}
          value={options.container}
          choices={CONTAINERS.map((value) => ({ value, label: t(`export.containers.${value}`) }))}
          onChange={(container) => change({ container })}
        />
        <Choice<ExportDepth>
          label={t('export.depth')}
          value={options.depth}
          choices={depthsOf(options.container).map((value) => ({
            value,
            label: t(`export.depths.${value}`),
          }))}
          onChange={(depth) => change({ depth })}
        />
        <Choice<ExportSpace | 'keep'>
          label={t('export.space')}
          value={options.space ?? 'keep'}
          disabled={!colors}
          choices={[
            { value: 'keep', label: t('export.spaces.keep') },
            ...SPACES.map((value) => ({ value, label: t(`export.spaces.${value}`) })),
          ]}
          onChange={(space) => change({ space: space === 'keep' ? null : space })}
        />
        <Choice<ExportTone>
          label={t('export.tone')}
          value={options.tone}
          disabled={!tone}
          choices={TONES.map((value) => ({ value, label: t(`export.tones.${value}`) }))}
          onChange={(next) => change({ tone: next })}
        />
        {options.container === 'jpeg' && (
          <label className="flex items-center gap-2">
            <span className="text-xs text-mist">{t('export.quality')}</span>
            <input
              type="range"
              aria-label={t('export.quality')}
              min={50}
              max={100}
              value={options.quality}
              onChange={(event) => change({ quality: Number(event.target.value) })}
              className="w-40 accent-[#d8a48f]"
            />
            <span className="w-8 tabular-nums">{options.quality}</span>
          </label>
        )}

        {losesDetail(options) && <p className="text-xs text-caution">{t('export.loses')}</p>}
        {failure && (
          <p className="text-xs text-danger" role="alert">
            {t('export.failed', { message: failure })}
          </p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <button
            type="button"
            disabled={busy}
            onClick={close}
            className="rounded border border-twilight px-3 py-1 hover:bg-twilight/50 disabled:opacity-40"
          >
            {t('export.cancel')}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void save(t('export.title'), t('export.sameFile'))}
            className="rounded bg-ember/90 px-3 py-1 text-midnight hover:bg-ember disabled:opacity-50"
          >
            {busy ? t('export.saving') : t('export.save')}
          </button>
        </div>
      </div>
    </div>
  )
}
