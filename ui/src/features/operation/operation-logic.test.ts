import { describe, expect, it } from 'vitest'

import { placeInfo } from '../file-list/model/placement'
import { info } from '../file-list/model/test-fixtures'
import { emptyWorkspace } from '../file-list/model/workspace'
import {
  calibratedLightsBytes,
  hasCalibrationData,
  hasFramesToIntegrate,
  hasLights,
  roleStatus,
} from './calibration-status'
import { defaultOutputFolder, parentFolder } from './output-folder'

const workspace = (entries: [string, string][]) => {
  let current = placeInfo(emptyWorkspace(), info({ path: '/n/lights/1.raf' })).workspace
  const sessionId = current.sessions[0]!.id
  for (const [role, path] of entries) {
    current = placeInfo(current, info({ path }), { sessionId, role: role as 'dark' }).workspace
  }
  return current
}

describe('calibration status', () => {
  it('reports none / frames / master per role', () => {
    const base = workspace([['dark', '/n/darks/1.raf'], ['dark', '/n/darks/2.raf']])
    const session = base.sessions[0]
    expect(roleStatus(session, 'dark')).toEqual({ kind: 'frames', count: 2 })
    expect(roleStatus(session, 'flat')).toEqual({ kind: 'none' })
    expect(roleStatus(undefined, 'bias')).toEqual({ kind: 'none' })
  })

  it('knows whether there is anything to calibrate with or integrate', () => {
    const empty = workspace([]).sessions[0]
    expect(hasLights(empty)).toBe(true)
    expect(hasCalibrationData(empty)).toBe(false)
    const withDarks = workspace([['dark', '/n/darks/1.raf']]).sessions[0]
    expect(hasCalibrationData(withDarks)).toBe(true)
    expect(hasFramesToIntegrate(withDarks)).toBe(true)
  })
})

describe('calibrated lights size', () => {
  it('estimates 2 bytes per sensor sample for every light', () => {
    const base = workspace([])
    expect(calibratedLightsBytes(base.sessions[0])).toBe(100 * 80 * 2)
    expect(calibratedLightsBytes(undefined)).toBeNull()
  })
})

describe('default output folders', () => {
  it('puts outputs next to the first input under stary-output', () => {
    const base = workspace([])
    expect(defaultOutputFolder(base, 'result')).toBe('/n/lights/stary-output')
    expect(defaultOutputFolder(base, 'master')).toBe('/n/lights/stary-output/masters')
    expect(defaultOutputFolder(base, 'calibrated')).toBe('/n/lights/stary-output/calibrated')
  })

  it('has no default without input files', () => {
    expect(defaultOutputFolder(emptyWorkspace(), 'result')).toBeNull()
  })

  it('keeps Windows separators', () => {
    expect(parentFolder('C:\\sky\\lights\\1.raf')).toBe('C:\\sky\\lights')
  })
})
