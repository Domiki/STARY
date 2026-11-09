import { describe, expect, it } from 'vitest'

import type { ProbeInfo, Role } from '../../../ipc/types'
import { createSession, placeInfo } from './placement'
import { setRepresentative } from './representative'
import { info } from './test-fixtures'
import { warningsOf } from './warnings'
import { type Session, type Workspace } from './workspace'

function buildSession(entries: [Role, ProbeInfo][]): Session {
  const session = createSession('s1')
  const start: Workspace = { sessions: [session], results: [], activeSessionId: 's1' }
  const workspace = entries.reduce(
    (current, [role, item]) => placeInfo(current, item, { sessionId: 's1', role }).workspace,
    start,
  )
  return workspace.sessions[0]!
}

const keysFor = (session: Session, path: string) =>
  (warningsOf(session).get(path) ?? []).map((warning) => warning.key)

describe('warningsOf', () => {
  it('is empty for a consistent session', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1' })],
      ['light', info({ path: '/l/2' })],
      ['dark', info({ path: '/d/1' })],
    ])
    expect(warningsOf(session).size).toBe(0)
  })

  it('flags a dark whose exposure differs from the lights', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', exposure_s: 30 })],
      ['dark', info({ path: '/d/1', exposure_s: 10 })],
    ])
    expect(keysFor(session, '/d/1')).toContain('exposure_differs')
  })

  it('flags frames inside a slot that differ in ISO', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', iso: 800 })],
      ['light', info({ path: '/l/2', iso: 1600 })],
    ])
    expect(keysFor(session, '/l/2')).toContain('iso_differs')
    expect(keysFor(session, '/l/1')).toEqual([])
  })

  it('ignores values that are missing on either side', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', exposure_s: null, iso: null })],
      ['dark', info({ path: '/d/1', exposure_s: 10, iso: 400 })],
    ])
    expect(warningsOf(session).size).toBe(0)
  })

  it('does not compare flat exposure with the lights', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', exposure_s: 30 })],
      ['flat', info({ path: '/f/1', exposure_s: 0.01 })],
    ])
    expect(warningsOf(session).size).toBe(0)
  })

  it('warns that jpeg calibration frames are less accurate', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', format: 'jpeg', layout: { kind: 'rgb' } })],
      ['dark', info({ path: '/d/1', format: 'jpeg', layout: { kind: 'rgb' } })],
    ])
    expect(keysFor(session, '/d/1')).toEqual(['low_accuracy'])
  })

  it('flags a file format that differs from the session', () => {
    const session = buildSession([
      ['light', info({ path: '/l/1', format: 'fits' })],
      ['light', info({ path: '/l/2', format: 'tiff' })],
    ])
    expect(keysFor(session, '/l/2')).toContain('format_mixed')
  })

  describe('with another representative light', () => {
    const sessionWith = (darkExposure: number): Session =>
      buildSession([
        ['light', info({ path: '/l/1', exposure_s: 30, format: 'fits' })],
        ['light', info({ path: '/l/2', exposure_s: 60, format: 'tiff' })],
        ['dark', info({ path: '/d/1', exposure_s: darkExposure })],
      ])
    const represent = (session: Session, path: string): Session =>
      setRepresentative(
        { sessions: [session], results: [], activeSessionId: session.id },
        session.id,
        path,
      ).sessions[0]!

    it('judges darks and lights against the first light by default', () => {
      expect(keysFor(sessionWith(30), '/d/1')).not.toContain('exposure_differs')
      expect(keysFor(sessionWith(60), '/d/1')).toContain('exposure_differs')
      expect(keysFor(sessionWith(30), '/l/2')).toContain('exposure_differs')
      expect(keysFor(sessionWith(30), '/l/1')).not.toContain('exposure_differs')
    })

    it('moves the warnings to the new representative', () => {
      const thirty = represent(sessionWith(30), '/l/2')
      expect(keysFor(thirty, '/d/1')).toContain('exposure_differs')
      expect(keysFor(thirty, '/l/1')).toContain('exposure_differs')
      expect(keysFor(thirty, '/l/2')).not.toContain('exposure_differs')
      const sixty = represent(sessionWith(60), '/l/2')
      expect(keysFor(sixty, '/d/1')).not.toContain('exposure_differs')
    })

    it('takes the format of the representative as the one of the session', () => {
      const session = represent(sessionWith(30), '/l/2')
      expect(keysFor(session, '/l/1')).toContain('format_mixed')
      expect(keysFor(session, '/l/2')).not.toContain('format_mixed')
    })
  })
})
