import { describe, expect, it } from 'vitest'

import { clearMaster, masterAvailability, setMaster } from './master'
import { placeInfo } from './placement'
import { info } from './test-fixtures'
import { removePaths } from './removal'
import { type Workspace, emptyWorkspace } from './workspace'

function build(entries: [string, string][]): Workspace {
  let workspace = placeInfo(emptyWorkspace(), info({ path: '/l/1' })).workspace
  const sessionId = workspace.sessions[0]!.id
  for (const [role, path] of entries) {
    workspace = placeInfo(workspace, info({ path }), {
      sessionId,
      role: role as 'dark',
    }).workspace
  }
  return workspace
}

describe('master designation', () => {
  it('allows a master only when the file is alone in its role', () => {
    const workspace = build([['dark', '/d/1']])
    const session = workspace.sessions[0]!
    expect(masterAvailability(session, 'dark', '/d/1')).toBe('ok')
    const result = setMaster(workspace, session.id, 'dark', '/d/1')
    expect(result.workspace.sessions[0]!.slots.dark.kind).toBe('master')
  })

  it('asks for the other files to be removed first', () => {
    const workspace = build([
      ['dark', '/d/1'],
      ['dark', '/d/2'],
    ])
    const session = workspace.sessions[0]!
    expect(masterAvailability(session, 'dark', '/d/1')).toBe('other_files')
    expect(setMaster(workspace, session.id, 'dark', '/d/1').workspace).toBe(workspace)
    const cleared = removePaths(workspace, ['/d/2'])
    expect(masterAvailability(cleared.sessions[0]!, 'dark', '/d/1')).toBe('ok')
  })

  it('never allows a master for Light', () => {
    const workspace = build([])
    expect(masterAvailability(workspace.sessions[0]!, 'light', '/l/1')).toBe('light_role')
  })

  it('turns a master back into a plain file', () => {
    const workspace = build([['bias', '/b/1']])
    const session = workspace.sessions[0]!
    const withMaster = setMaster(workspace, session.id, 'bias', '/b/1').workspace
    const restored = clearMaster(withMaster, session.id, 'bias')
    expect(restored.sessions[0]!.slots.bias.kind).toBe('frames')
  })

  it('then rejects further frames while the master is set', () => {
    const workspace = build([['flat', '/f/1']])
    const session = workspace.sessions[0]!
    const withMaster = setMaster(workspace, session.id, 'flat', '/f/1').workspace
    const { rejection } = placeInfo(withMaster, info({ path: '/f/2' }), {
      sessionId: session.id,
      role: 'flat',
    })
    expect(rejection?.reason).toBe('slot_holds_master')
  })
})

describe('removePaths', () => {
  it('removes files across roles and results in one pass', () => {
    const workspace = build([
      ['dark', '/d/1'],
      ['flat', '/f/1'],
    ])
    const next = removePaths(workspace, ['/l/1', '/d/1'])
    const session = next.sessions[0]!
    expect(session.slots.light.kind).toBe('empty')
    expect(session.slots.dark.kind).toBe('empty')
    expect(session.slots.flat.kind).toBe('frames')
  })
})
