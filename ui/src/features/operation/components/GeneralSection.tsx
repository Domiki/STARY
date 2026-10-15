import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { CollapsibleSection } from '../../../shared/CollapsibleSection'
import { type ComputeDevice, useSettingsStore } from '../../settings/store'

const DEVICES: ComputeDevice[] = ['cpu', 'gpu']

export function GeneralSection() {
  const { t } = useTranslation()
  const { device, gpu, memoryPercent, detectGpu, setDevice, setMemoryPercent } = useSettingsStore()
  useEffect(() => void detectGpu(), [detectGpu])

  return (
    <CollapsibleSection title={t('operation.general.title')}>
      <div className="space-y-3 px-3 pb-3">
        <div className="space-y-1">
          <span className="text-xs text-mist">{t('operation.general.device.title')}</span>
          <div role="radiogroup" className="grid grid-cols-2 rounded border border-twilight p-0.5">
            {DEVICES.map((choice) => (
              <button
                key={choice}
                type="button"
                role="radio"
                aria-checked={device === choice}
                disabled={choice === 'gpu' && !gpu}
                onClick={() => setDevice(choice)}
                className={`rounded py-1 disabled:cursor-default disabled:opacity-40 ${device === choice ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
              >
                {t(`operation.general.device.${choice}`)}
              </button>
            ))}
          </div>
          <ul className="space-y-0.5 text-xs text-mist">
            <li>{t('operation.general.device.cpuInfo', { count: navigator.hardwareConcurrency })}</li>
            <li>
              {gpu
                ? t('operation.general.device.gpuInfo', { name: gpu.name, backend: gpu.backend })
                : t(gpu === undefined ? 'operation.general.device.gpuChecking' : 'operation.general.device.gpuUnavailable')}
            </li>
          </ul>
        </div>
        <label className="block">
          <span className="mb-1 flex justify-between text-xs text-mist">
            <span>{t('operation.general.memory')}</span>
            <span className="text-moon">{memoryPercent}%</span>
          </span>
          <input
            type="range"
            min={10}
            max={80}
            step={5}
            value={memoryPercent}
            onChange={(event) => setMemoryPercent(Number(event.target.value))}
            className="w-full accent-[#d8a48f]"
          />
        </label>
      </div>
    </CollapsibleSection>
  )
}
