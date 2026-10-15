import { describe, expect, it } from 'vitest'

import { applyOutcomes } from './apply-outcomes'
import { placeInfo } from './placement'
import { removeFromSession, removeSession } from './removal'
import { info } from './test-fixtures'
import { type Workspace, emptyWorkspace, slotItems } from './workspace'

const first = (workspace: Workspace) => workspace.sessions[0]!

function placeAll(paths: string[], overrides = {}): Workspace {
  return paths.reduce(
    (workspace, path) => placeInfo(workspace, info({ path, ...overrides })).workspace,
    emptyWorkspace(),
  )
}

describe('placeInfo', () => {
  it('creates a session and puts plain files into Light', () => {
    const workspace = placeAll(['/a/1.fits', '/a/2.fits'])
    expect(workspace.sessions).toHaveLength(1)
    expect(slotItems(first(workspace).slots.light).map((item) => item.path)).toEqual([
      '/a/1.fits',
      '/a/2.fits',
    ])
  })

  it('routes a master to the role named by its keyword', () => {
    const { workspace } = placeInfo(
      emptyWorkspace(),
      info({ path: '/m/dark.fits', stary_kind: 'master_dark' }),
    )
    expect(first(workspace).slots.dark.kind).toBe('master')
    expect(first(workspace).slots.light.kind).toBe('empty')
  })

  it('rejects a master dropped on the wrong role', () => {
    const base = placeAll(['/a/1.fits'])
    const { rejection } = placeInfo(
      base,
      info({ path: '/m/dark.fits', stary_kind: 'master_dark' }),
      { sessionId: first(base).id, role: 'flat' },
    )
    expect(rejection).toMatchObject({ reason: 'wrong_role', expected: 'dark' })
  })

  it('rejects frames into a slot that holds a master', () => {
    const base = placeInfo(
      emptyWorkspace(),
      info({ path: '/m/dark.fits', stary_kind: 'master_dark' }),
    ).workspace
    const { rejection } = placeInfo(base, info({ path: '/d/1.fits' }), {
      sessionId: first(base).id,
      role: 'dark',
    })
    expect(rejection?.reason).toBe('slot_holds_master')
  })

  it('rejects a master into a slot that holds frames until they are removed', () => {
    const seeded = placeAll(['/l/1.fits'])
    const session = first(seeded)
    const base = placeInfo(seeded, info({ path: '/d/1.fits' }), {
      sessionId: session.id,
      role: 'dark',
    }).workspace
    const master = info({ path: '/m/dark.fits', stary_kind: 'master_dark' })
    expect(placeInfo(base, master).rejection?.reason).toBe('slot_holds_frames')
    const cleared = removeFromSession(base, session.id, 'dark', '/d/1.fits')
    expect(placeInfo(cleared, master).rejection).toBeNull()
  })

  it('rejects a second master in the same slot', () => {
    const base = placeInfo(
      emptyWorkspace(),
      info({ path: '/m/one.fits', stary_kind: 'master_dark' }),
    ).workspace
    const { rejection } = placeInfo(
      base,
      info({ path: '/m/two.fits', stary_kind: 'master_dark' }),
    )
    expect(rejection?.reason).toBe('slot_holds_master')
  })

  it('rejects duplicates', () => {
    const base = placeAll(['/a/1.fits'])
    expect(placeInfo(base, info({ path: '/a/1.fits' })).rejection?.reason).toBe('duplicate')
  })

  it('blocks files whose size differs from the session', () => {
    const base = placeAll(['/a/1.fits'])
    const { rejection } = placeInfo(base, info({ path: '/a/2.fits', width: 200 }))
    expect(rejection?.reason).toBe('incompatible_size')
  })

  it('blocks files whose pixel layout differs from the session', () => {
    const base = placeAll(['/a/1.fits'])
    const { rejection } = placeInfo(base, info({ path: '/a/2.jpg', layout: { kind: 'rgb' } }))
    expect(rejection?.reason).toBe('incompatible_layout')
  })

  it('allows exposure differences (only warned about elsewhere)', () => {
    const base = placeAll(['/a/1.fits'])
    expect(placeInfo(base, info({ path: '/a/2.fits', exposure_s: 60 })).rejection).toBeNull()
  })

  it('sends stack results to the Result section only', () => {
    const { workspace } = placeInfo(
      emptyWorkspace(),
      info({ path: '/r/out.fits', stary_kind: 'stack_result' }),
    )
    expect(workspace.results).toHaveLength(1)
    expect(workspace.sessions).toHaveLength(0)
  })
})

describe('applyOutcomes', () => {
  it('sorts files naturally and reports probe errors', () => {
    const outcomes = [
      { status: 'ok' as const, info: info({ path: '/a/10.fits' }) },
      { status: 'ok' as const, info: info({ path: '/a/2.fits' }) },
      { status: 'error' as const, path: '/a/bad.txt', code: 'unsupported_file', message: '' },
    ]
    const applied = applyOutcomes(emptyWorkspace(), outcomes)
    expect(applied.placed).toEqual(['/a/2.fits', '/a/10.fits'])
    expect(applied.rejections).toEqual([{ path: '/a/bad.txt', reason: 'unsupported_file' }])
  })

  it('maps unknown error codes to internal', () => {
    const outcomes = [{ status: 'error' as const, path: '/x', code: 'weird', message: '' }]
    expect(applyOutcomes(emptyWorkspace(), outcomes).rejections[0]?.reason).toBe('internal')
  })
})

describe('removal', () => {
  it('empties a slot when its last frame is removed', () => {
    const base = placeAll(['/a/1.fits'])
    const next = removeFromSession(base, first(base).id, 'light', '/a/1.fits')
    expect(first(next).slots.light.kind).toBe('empty')
  })

  it('keeps an active session after removing the active one', () => {
    const base = placeAll(['/a/1.fits'])
    const next = removeSession(base, first(base).id)
    expect(next.sessions).toHaveLength(0)
    expect(next.activeSessionId).toBeNull()
  })
})
