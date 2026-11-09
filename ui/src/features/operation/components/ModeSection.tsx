import { useTranslation } from 'react-i18next'

import type {
  AlignModel,
  CombineMethod,
  DemosaicQuality,
  Resampler,
  TrailsMethodChoice,
} from '../../../ipc/types'
import { CollapsibleSection } from '../../../shared/CollapsibleSection'
import { Toggle } from '../../../shared/Toggle'
import { useFileListStore } from '../../file-list/store'
import { sessionDisplayName } from '../../file-list/session-name'
import { hasLights } from '../calibration-status'
import { defaultOutputFolder } from '../output-folder'
import { referenceSessionOf } from '../reference-session'
import { useHdr } from '../useHdr'
import { type Mode, useOperationStore } from '../store'
import { FolderField } from './FolderField'

const MODES: Mode[] = ['trails', 'stacking']
const QUALITIES: DemosaicQuality[] = ['fast', 'high']
const ALIGN_MODELS: AlignModel[] = ['similarity', 'affine', 'homography']
const TRAILS_METHODS: TrailsMethodChoice[] = ['lighten', 'comet', 'gap_filling']
const RESAMPLERS: Resampler[] = ['bilinear', 'bicubic', 'lanczos3']
const COMBINES: CombineMethod[] = ['average', 'sigma']
const FIELD =
  'w-full rounded border border-twilight bg-midnight px-2 py-1 text-moon disabled:opacity-45'

interface Option {
  value: string
  label: string
  disabled?: boolean
}

function SelectField(props: {
  label: string
  options: Option[]
  value?: string
  disabled?: boolean
  onChange?: (value: string) => void
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-mist">{props.label}</span>
      <select
        className={FIELD}
        disabled={props.disabled}
        value={props.value}
        onChange={(event) => props.onChange?.(event.target.value)}
      >
        {props.options.map(({ value, label, disabled }) => (
          <option key={value} value={value} disabled={disabled}>
            {label}
          </option>
        ))}
      </select>
    </label>
  )
}

function NumberField(props: {
  label: string
  value: number
  min?: number
  max?: number
  step?: number
  onChange: (value: number) => void
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-mist">{props.label}</span>
      <input
        type="number"
        className={FIELD}
        min={props.min ?? 0.5}
        max={props.max ?? 10}
        step={props.step ?? 0.1}
        value={props.value}
        onChange={(event) => {
          const next = event.target.valueAsNumber
          if (Number.isFinite(next)) props.onChange(next)
        }}
      />
    </label>
  )
}

function GroundOptions() {
  const { t } = useTranslation()
  const hdr = useHdr()
  const { groundMasking, groundSeparate, setGroundMasking, setGroundSeparate } = useOperationStore()
  return (
    <div>
      <Toggle
        label={t('operation.mode.groundMasking')}
        checked={groundMasking}
        onChange={setGroundMasking}
      />
      <div className="pl-4">
        <Toggle
          label={t('operation.mode.groundStack')}
          checked={groundMasking && groundSeparate && !hdr.active}
          disabled={!groundMasking || hdr.active}
          onChange={setGroundSeparate}
        />
      </div>
    </div>
  )
}

function TrailsOptions() {
  const { t } = useTranslation()
  const { trails, setTrails } = useOperationStore()
  return (
    <>
      <SelectField
        label={t('operation.mode.combine')}
        value={trails.method}
        onChange={(method) => setTrails({ method: method as TrailsMethodChoice })}
        options={TRAILS_METHODS.map((value) => ({
          value,
          label: t(`operation.mode.trailsMethods.${value}`),
        }))}
      />
      {trails.method === 'comet' && (
        <>
          <NumberField
            label={t('operation.mode.tailFrames')}
            value={trails.tail_frames}
            min={1}
            max={500}
            step={1}
            onChange={(tail) => setTrails({ tail_frames: Math.max(1, Math.round(tail)) })}
          />
          <Toggle
            label={t('operation.mode.reverseOrder')}
            checked={trails.reverse_order}
            onChange={(reverse_order) => setTrails({ reverse_order })}
          />
        </>
      )}
      {trails.method === 'gap_filling' && (
        <p className="text-xs text-mist">{t('operation.mode.gapFillingNote')}</p>
      )}
      <GroundOptions />
    </>
  )
}

function StackingOptions() {
  const { t } = useTranslation()
  const { stacking, setStacking, referenceSession, setReferenceSession, setHdr } =
    useOperationStore()
  const hdr = useHdr()
  const workspace = useFileListStore((state) => state.workspace)
  const reference = referenceSessionOf(workspace, referenceSession)
  const sessions = workspace.sessions.flatMap((session, index) =>
    hasLights(session) ? [{ value: session.id, label: sessionDisplayName(session, index, t) }] : [],
  )
  const labelled = <T extends string>(group: string, values: T[]) =>
    values.map((value) => ({ value, label: t(`operation.mode.${group}.${value}`) }))
  return (
    <>
      <SelectField
        label={t('operation.mode.reference')}
        value={reference?.id}
        onChange={setReferenceSession}
        options={sessions}
      />
      <SelectField
        label={t('operation.mode.alignModel')}
        value={stacking.model}
        onChange={(model) => setStacking({ model: model as AlignModel })}
        options={labelled('alignModels', ALIGN_MODELS)}
      />
      <SelectField
        label={t('operation.mode.resampler')}
        value={stacking.resampler}
        onChange={(resampler) => setStacking({ resampler: resampler as Resampler })}
        options={labelled('resamplers', RESAMPLERS)}
      />
      <SelectField
        label={t('operation.mode.combine')}
        value={stacking.combine}
        onChange={(combine) => setStacking({ combine: combine as CombineMethod })}
        options={COMBINES.map((value) => ({ value, label: t(`operation.mode.${value}`) }))}
      />
      {stacking.combine !== 'average' && (
        <div className="grid grid-cols-2 gap-2">
          <NumberField
            label={t('operation.mode.rejectLow')}
            value={stacking.low}
            onChange={(low) => setStacking({ low })}
          />
          <NumberField
            label={t('operation.mode.rejectHigh')}
            value={stacking.high}
            onChange={(high) => setStacking({ high })}
          />
        </div>
      )}
      <Toggle
        label={t('operation.mode.hdr')}
        checked={hdr.active}
        disabled={!hdr.available}
        onChange={setHdr}
      />
      <Toggle
        label={t('operation.mode.skipPoorFrames')}
        checked={stacking.skip_poor_frames}
        onChange={(skip_poor_frames) => setStacking({ skip_poor_frames })}
      />
      <GroundOptions />
    </>
  )
}

export function ModeSection() {
  const { t } = useTranslation()
  const { mode, setMode, quality, setQuality } = useOperationStore()
  const workspace = useFileListStore((state) => state.workspace)

  return (
    <CollapsibleSection title={t('operation.mode.title')}>
      <div className="space-y-3 px-3 pb-3">
        <div role="radiogroup" className="grid grid-cols-2 rounded border border-twilight p-0.5">
          {MODES.map((candidate) => (
            <button
              key={candidate}
              type="button"
              role="radio"
              aria-checked={mode === candidate}
              onClick={() => setMode(candidate)}
              className={`rounded py-1 ${mode === candidate ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
            >
              {t(`operation.mode.${candidate}`)}
            </button>
          ))}
        </div>

        {mode === 'trails' ? <TrailsOptions /> : <StackingOptions />}

        <SelectField
          label={t('operation.mode.quality')}
          value={quality}
          onChange={(value) => setQuality(value as DemosaicQuality)}
          options={QUALITIES.map((value) => ({
            value,
            label: t(`operation.mode.qualities.${value}`),
          }))}
        />
        <FolderField
          kind="result"
          label={t('operation.mode.resultFolder')}
          defaultFolder={defaultOutputFolder(workspace, 'result')}
        />
      </div>
    </CollapsibleSection>
  )
}
