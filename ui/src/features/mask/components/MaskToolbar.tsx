import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { IconButton } from '../../../shared/IconButton'
import { RedoIcon, UndoIcon } from '../../../shared/icons'
import { useJobStore } from '../../operation/job-store'
import { skyShare } from '../model/paint'
import { type Tool, canRedo, canUndo, maskOf, useMaskStore } from '../store'

interface Props {
  sessionId: string
  path: string
}

const TOOLS: Tool[] = ['hand', 'sky', 'ground', 'refine']
const RANGE = 'w-24 accent-[#d8a48f]'

function Slider(props: {
  label: string
  value: number
  min: number
  max: number
  step: number
  format: (value: number) => string
  onChange: (value: number) => void
}) {
  return (
    <label className="flex items-center gap-1.5">
      <span className="text-mist">{props.label}</span>
      <input
        type="range"
        aria-label={props.label}
        className={RANGE}
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onChange={(event) => props.onChange(Number(event.target.value))}
      />
      <span className="w-9 text-right tabular-nums">{props.format(props.value)}</span>
    </label>
  )
}

export function MaskToolbar({ sessionId, path }: Props) {
  const { t } = useTranslation()
  const store = useMaskStore()
  const state = maskOf(store.entries, sessionId)
  const editable = state.phase !== 'none'
  const undoable = canUndo(store.entries, sessionId)
  const redoable = canRedo(store.entries, sessionId)
  const tools = TOOLS.filter((tool) => tool !== 'refine' || state.phase === 'contour')
  const mask = state.phase === 'applied' ? state.mask : state.shaped

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'z') return
      if (useJobStore.getState().phase === 'running') return
      event.preventDefault()
      const { undo, redo } = useMaskStore.getState()
      void (event.shiftKey ? redo(sessionId) : undo(sessionId))
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [sessionId])

  return (
    <div className="space-y-1.5 border-b border-twilight px-2 py-1.5 text-xs">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
        <button
          type="button"
          disabled={store.busy}
          onClick={() => void store.requestDetect({ sessionId, path })}
          className="rounded border border-ember/70 px-2 py-0.5 text-ember hover:bg-ember/15 disabled:opacity-40"
        >
          {t('mask.detect')}
        </button>
        <div role="radiogroup" aria-label={t('mask.tools.title')} className="flex rounded border border-twilight p-0.5">
          {tools.map((tool) => (
            <button
              key={tool}
              type="button"
              role="radio"
              aria-checked={store.tool === tool}
              disabled={tool !== 'hand' && !editable}
              onClick={() => store.setTool(tool)}
              className={`rounded px-2 py-0.5 disabled:opacity-40 ${store.tool === tool ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
            >
              {t(`mask.tools.${tool}`)}
            </button>
          ))}
        </div>
        <Slider
          label={t('mask.size')}
          value={store.brushRadius}
          min={2}
          max={150}
          step={1}
          format={(value) => `${value}`}
          onChange={store.setBrushRadius}
        />
        {state.phase === 'applied' && (
          <Slider
            label={t('mask.hardness')}
            value={Math.round(store.hardness * 100)}
            min={0}
            max={100}
            step={5}
            format={(value) => `${value}%`}
            onChange={(percent) => store.setHardness(percent / 100)}
          />
        )}
        <span className="ml-auto flex items-center gap-1">
          {store.busy && <span className="text-mist">{t('mask.working')}</span>}
          {store.failure && (
            <span className="text-danger" title={store.failure}>
              {t('mask.failed')}
            </span>
          )}
          {mask && <span className="text-mist">{t('mask.skyShare', { percent: Math.round(skyShare(mask) * 100) })}</span>}
          <IconButton label={t('mask.undo')} disabled={!undoable} onClick={() => void store.undo(sessionId)}>
            <UndoIcon />
          </IconButton>
          <IconButton label={t('mask.redo')} disabled={!redoable} onClick={() => void store.redo(sessionId)}>
            <RedoIcon />
          </IconButton>
          <button
            type="button"
            disabled={!editable}
            onClick={() => store.clear(sessionId)}
            className="rounded border border-twilight px-2 py-0.5 hover:bg-twilight/50 disabled:opacity-40"
          >
            {t('mask.clear')}
          </button>
        </span>
      </div>
      {state.phase === 'contour' && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
          <Slider
            label={t('mask.offset')}
            value={state.offset}
            min={-30}
            max={30}
            step={0.5}
            format={(value) => `${value}`}
            onChange={(offset) => void store.shape(sessionId, { offset })}
          />
          <Slider
            label={t('mask.feather')}
            value={state.feather}
            min={0}
            max={60}
            step={0.5}
            format={(value) => `${value}`}
            onChange={(feather) => void store.shape(sessionId, { feather })}
          />
          <button
            type="button"
            onClick={() => store.apply(sessionId)}
            className="ml-auto rounded bg-ember/90 px-3 py-0.5 text-midnight hover:bg-ember"
          >
            {t('mask.apply')}
          </button>
        </div>
      )}
    </div>
  )
}
