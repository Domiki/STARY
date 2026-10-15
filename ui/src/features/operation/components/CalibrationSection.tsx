import { useTranslation } from 'react-i18next'

import { CollapsibleSection } from '../../../shared/CollapsibleSection'
import { formatBytes } from '../../../shared/format'
import { Toggle } from '../../../shared/Toggle'
import { sessionDisplayName } from '../../file-list/session-name'
import { useFileListStore } from '../../file-list/store'
import {
  CALIBRATION_ROLES,
  type RoleStatus,
  calibratedLightsBytes,
  hasCalibrationData,
  hasFramesToIntegrate,
  hasLights,
  roleStatus,
} from '../calibration-status'
import { canRun } from '../job-request'
import { useJobStore } from '../job-store'
import { defaultOutputFolder } from '../output-folder'
import type { ReadMode } from '../../../ipc/types'
import { useSettingsStore } from '../../settings/store'
import { useOperationChoices } from '../useOperationChoices'
import { useOperationStore } from '../store'
import { FolderField } from './FolderField'

const BUTTON =
  'w-full rounded border border-twilight px-3 py-1.5 hover:bg-twilight/50 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent'

const READ_MODES: ReadMode[] = ['exact', 'fast']

export function CalibrationSection() {
  const { t } = useTranslation()
  const workspace = useFileListStore((state) => state.workspace)
  const applyCalibration = useOperationStore((state) => state.applyCalibration)
  const setApplyCalibration = useOperationStore((state) => state.setApplyCalibration)
  const saveMasters = useOperationStore((state) => state.saveMasters)
  const setSaveMasters = useOperationStore((state) => state.setSaveMasters)
  const saveCalibrated = useOperationStore((state) => state.saveCalibrated)
  const setSaveCalibrated = useOperationStore((state) => state.setSaveCalibrated)

  const readMode = useSettingsStore((state) => state.readMode)
  const setReadMode = useSettingsStore((state) => state.setReadMode)

  const index = workspace.sessions.findIndex(({ id }) => id === workspace.activeSessionId)
  const session = workspace.sessions[index]
  const sessionName = session ? sessionDisplayName(session, index, t) : ''
  const hasData = hasCalibrationData(session)
  const canSaveMasters = hasFramesToIntegrate(session)
  const canSaveCalibrated = hasData && hasLights(session)
  const lightsBytes = calibratedLightsBytes(session)
  const choices = useOperationChoices()
  const mode = useOperationStore((state) => state.mode)
  const running = useJobStore((state) => state.phase === 'running')
  const start = useJobStore((state) => state.start)
  const runnable = !running && canRun('calibration', mode, workspace, choices)

  const statusText = (status: RoleStatus) =>
    status.kind === 'master'
      ? t('operation.calibration.master')
      : status.kind === 'frames'
        ? t('operation.calibration.frames', { count: status.count })
        : t('operation.calibration.none')

  return (
    <CollapsibleSection
      title={t('operation.calibration.title')}
      actions={<span className="max-w-32 truncate text-xs text-mist">{sessionName}</span>}
    >
      <div className="space-y-3 px-3 pb-3">
        <ul className="space-y-0.5 text-xs">
          {CALIBRATION_ROLES.map((role) => {
            const status = roleStatus(session, role)
            return (
              <li key={role} className="flex justify-between">
                <span className="text-mist">{t(`roles.${role}`)}</span>
                <span className={status.kind === 'none' ? 'text-mist/70' : ''}>
                  {statusText(status)}
                </span>
              </li>
            )
          })}
        </ul>
        <Toggle
          label={t('operation.calibration.apply')}
          checked={applyCalibration && hasData}
          disabled={!hasData}
          onChange={setApplyCalibration}
        />
        <div className="space-y-1">
          <span className="text-xs text-mist">{t('operation.calibration.read.title')}</span>
          <div role="radiogroup" className="grid grid-cols-2 rounded border border-twilight p-0.5">
            {READ_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                role="radio"
                aria-checked={readMode === mode}
                onClick={() => setReadMode(mode)}
                className={`rounded py-1 ${readMode === mode ? 'bg-twilight text-moon' : 'text-mist hover:text-moon'}`}
              >
                {t(`operation.calibration.read.${mode}`)}
              </button>
            ))}
          </div>
        </div>
        <div className="space-y-2">
          <Toggle
            label={t('operation.calibration.saveMasters')}
            checked={saveMasters && canSaveMasters}
            disabled={!canSaveMasters}
            onChange={setSaveMasters}
          />
          {saveMasters && canSaveMasters && (
            <FolderField
              kind="master"
              label={t('operation.calibration.masterFolder')}
              defaultFolder={defaultOutputFolder(workspace, 'master')}
            />
          )}
        </div>
        <div className="space-y-2">
          <Toggle
            label={t('operation.calibration.saveCalibrated')}
            checked={saveCalibrated && canSaveCalibrated}
            disabled={!canSaveCalibrated}
            onChange={setSaveCalibrated}
          />
          {saveCalibrated && canSaveCalibrated && (
            <>
              <FolderField
                kind="calibrated"
                label={t('operation.calibration.calibratedFolder')}
                defaultFolder={defaultOutputFolder(workspace, 'calibrated')}
              />
              {lightsBytes !== null && (
                <p className="text-xs text-caution">
                  {t('operation.calibration.sizeEstimate', { size: formatBytes(lightsBytes) })}
                </p>
              )}
            </>
          )}
        </div>
        <button
          type="button"
          disabled={!runnable}
          onClick={() => void start('calibration')}
          className={BUTTON}
          title={runnable ? undefined : t('operation.calibration.runHint')}
        >
          {t('operation.calibration.runCalibration')}
        </button>
      </div>
    </CollapsibleSection>
  )
}
