import { describe, expect, it } from 'vitest'

import type { Role } from '../../ipc/types'
import { placeInfo } from '../file-list/model/placement'
import { setRepresentative } from '../file-list/model/representative'
import { info } from '../file-list/model/test-fixtures'
import { type Workspace, emptyWorkspace } from '../file-list/model/workspace'
import { type OperationChoices, buildJobRequest, canRun } from './job-request'

const choices: OperationChoices = {
  applyCalibration: true,
  saveMasters: false,
  saveCalibrated: false,
  folders: { master: null, calibrated: null, result: null },
  quality: 'fast',
  trails: { method: 'lighten', tail_frames: 10, reverse_order: false },
  stacking: { model: 'similarity', resampler: 'bicubic', combine: 'winsorized', low: 3, high: 3, skip_poor_frames: true },
  memoryPercent: 50,
  device: 'cpu',
  read: 'exact',
  ground: { masking: false, separate: false, maskReady: false },
}

function workspace(entries: [Role, string][] = [], master?: [Role, string]): Workspace {
  let current = placeInfo(emptyWorkspace(), info({ path: '/n/lights/1.raf' })).workspace
  current = placeInfo(current, info({ path: '/n/lights/2.raf' })).workspace
  const sessionId = current.sessions[0]!.id
  for (const [role, path] of entries) {
    current = placeInfo(current, info({ path }), { sessionId, role }).workspace
  }
  if (master) {
    current = placeInfo(
      current,
      info({ path: master[1], stary_kind: `master_${master[0]}` as 'master_dark' }),
      { sessionId, role: master[0] },
    ).workspace
  }
  return current
}

describe('buildJobRequest', () => {
  it('builds a trails request from the lights and the calibration frames', () => {
    const request = buildJobRequest('trails', 'trails', workspace([['dark', '/n/darks/1.raf']]), choices)!
    expect(request.kind).toBe('trails')
    expect(request.lights).toEqual(['/n/lights/1.raf', '/n/lights/2.raf'])
    expect(request.dark).toEqual({ kind: 'frames', paths: ['/n/darks/1.raf'] })
    expect(request.flat).toEqual({ kind: 'none' })
    expect(request.apply_calibration).toBe(true)
    expect(request.result_folder).toBe('/n/lights/stary-output')
  })

  it('sends a master as a master', () => {
    const request = buildJobRequest('trails', 'trails', workspace([], ['dark', '/m/dark.fits']), choices)!
    expect(request.dark).toEqual({ kind: 'master', path: '/m/dark.fits' })
  })

  it('does not apply calibration when there is nothing to apply or the toggle is off', () => {
    expect(buildJobRequest('trails', 'trails', workspace(), choices)!.apply_calibration).toBe(false)
    const off = { ...choices, applyCalibration: false }
    expect(buildJobRequest('trails', 'trails', workspace([['dark', '/d/1']]), off)!.apply_calibration).toBe(false)
  })

  it('uses the chosen folders and the default ones only for options that are switched on', () => {
    const on = { ...choices, saveMasters: true, saveCalibrated: true, folders: { ...choices.folders, calibrated: '/mine' } }
    const request = buildJobRequest('trails', 'trails', workspace([['dark', '/d/1']]), on)!
    expect(request.masters_folder).toBe('/n/lights/stary-output/masters')
    expect(request.calibrated_folder).toBe('/mine')
    expect(buildJobRequest('trails', 'trails', workspace([['dark', '/d/1']]), choices)!.masters_folder).toBeNull()
  })

  it('does not offer to save masters that already exist as masters', () => {
    const on = { ...choices, saveMasters: true }
    const request = buildJobRequest('trails', 'trails', workspace([], ['dark', '/m/dark.fits']), on)!
    expect(request.masters_folder).toBeNull()
  })
})

describe('trails requests', () => {
  it('carries the method and the tail length', () => {
    const comet = { ...choices, trails: { method: 'comet', tail_frames: 25, reverse_order: true } } as const
    const request = buildJobRequest('trails', 'trails', workspace(), comet)!
    expect(request.trails).toEqual({ method: 'comet', tail_frames: 25, reverse_order: true })
    expect(buildJobRequest('trails', 'trails', workspace(), choices)!.trails.method).toBe('lighten')
  })

  it('needs two lights for gap filling but one for the other methods', () => {
    const one = placeInfo(emptyWorkspace(), info({ path: '/n/lights/1.raf' })).workspace
    const gap = { ...choices, trails: { method: 'gap_filling', tail_frames: 10, reverse_order: false } } as const
    expect(canRun('trails', 'trails', one, gap)).toBe(false)
    expect(canRun('trails', 'trails', workspace(), gap)).toBe(true)
    expect(canRun('trails', 'trails', one, choices)).toBe(true)
    expect(buildJobRequest('trails', 'trails', one, gap)).toBeNull()
  })
})

describe('stacking requests', () => {
  it('needs the Stacking mode and at least two lights', () => {
    const one = placeInfo(emptyWorkspace(), info({ path: '/n/lights/1.raf' })).workspace
    expect(canRun('stacking', 'stacking', one, choices)).toBe(false)
    expect(canRun('stacking', 'trails', workspace(), choices)).toBe(false)
    expect(canRun('stacking', 'stacking', workspace(), choices)).toBe(true)
  })

  it('carries the alignment and rejection choices', () => {
    const custom = {
      ...choices,
      stacking: { model: 'affine', resampler: 'lanczos3', combine: 'sigma', low: 2.5, high: 4, skip_poor_frames: false },
    } as const
    const request = buildJobRequest('stacking', 'stacking', workspace(), custom)!
    expect(request.kind).toBe('stacking')
    expect(request.stacking).toEqual({ ...custom.stacking, reference_index: 0 })
    expect(request.lights).toHaveLength(2)
  })

  it('lines the stack up on the representative image', () => {
    const base = workspace()
    const sessionId = base.sessions[0]!.id
    const chosen = setRepresentative(base, sessionId, '/n/lights/2.raf')
    expect(buildJobRequest('stacking', 'stacking', chosen, choices)!.stacking.reference_index).toBe(1)
    expect(buildJobRequest('stacking', 'stacking', base, choices)!.stacking.reference_index).toBe(0)
  })

  it('is not built for a single light', () => {
    const one = placeInfo(emptyWorkspace(), info({ path: '/n/lights/1.raf' })).workspace
    expect(buildJobRequest('stacking', 'stacking', one, choices)).toBeNull()
  })
})

describe('canRun', () => {
  it('needs lights for trails and the Trails mode to be selected', () => {
    expect(canRun('trails', 'trails', emptyWorkspace(), choices)).toBe(false)
    expect(canRun('trails', 'stacking', workspace(), choices)).toBe(false)
    expect(canRun('trails', 'trails', workspace(), choices)).toBe(true)
  })

  it('runs calibration only when something would be saved', () => {
    const base = workspace([['dark', '/d/1']])
    expect(canRun('calibration', 'trails', base, choices)).toBe(false)
    expect(canRun('calibration', 'trails', base, { ...choices, saveMasters: true })).toBe(true)
    expect(canRun('calibration', 'trails', base, { ...choices, saveCalibrated: true })).toBe(true)
    expect(canRun('calibration', 'trails', workspace(), { ...choices, saveCalibrated: true })).toBe(false)
  })
})

describe('how the masters are read', () => {
  it('goes into the request', () => {
    const fast = { ...choices, read: 'fast' as const }
    expect(buildJobRequest('trails', 'trails', workspace(), fast)!.read).toBe('fast')
    expect(buildJobRequest('trails', 'trails', workspace(), choices)!.read).toBe('exact')
  })
})

describe('ground masking in a request', () => {
  const masked = (separate: boolean, maskReady: boolean) => ({
    ...choices,
    ground: { masking: true, separate, maskReady },
  })

  it('cannot run until the mask is applied, except to only calibrate', () => {
    const lights = workspace([['dark', '/d/1']])
    expect(canRun('stacking', 'stacking', lights, masked(false, false))).toBe(false)
    expect(canRun('trails', 'trails', lights, masked(false, false))).toBe(false)
    expect(canRun('stacking', 'stacking', lights, masked(false, true))).toBe(true)
    expect(canRun('calibration', 'stacking', lights, { ...masked(false, false), saveCalibrated: true })).toBe(true)
  })

  it('names the session whose mask the job takes and what to do with the ground', () => {
    const lights = workspace()
    const sessionId = lights.sessions[0]!.id
    const apart = buildJobRequest('stacking', 'stacking', lights, masked(true, true))!
    expect(apart.masking).toEqual({ session_id: sessionId, ground: 'separate' })
    const together = buildJobRequest('trails', 'trails', lights, masked(false, true))!
    expect(together.masking).toEqual({ session_id: sessionId, ground: 'with_sky' })
  })

  it('sends no masking when it is off', () => {
    expect(buildJobRequest('trails', 'trails', workspace(), choices)!.masking).toBeNull()
  })
})
