import { describe, expect, it } from 'vitest'

import { initialWorkspace } from './placement'
import { MAX_SESSION_NAME_LENGTH, normalizeSessionName, renameSession } from './rename'

describe('session names', () => {
  it('trims whitespace', () => {
    expect(normalizeSessionName('  M16 night 1  ')).toBe('M16 night 1')
  })

  it('treats an empty name as the default name', () => {
    expect(normalizeSessionName('   ')).toBeNull()
    expect(normalizeSessionName('')).toBeNull()
  })

  it('limits the length', () => {
    expect(normalizeSessionName('x'.repeat(100))).toHaveLength(MAX_SESSION_NAME_LENGTH)
  })

  it('renames one session and can reset it', () => {
    const workspace = initialWorkspace()
    const id = workspace.sessions[0]!.id
    const renamed = renameSession(workspace, id, 'Eagle Nebula')
    expect(renamed.sessions[0]!.name).toBe('Eagle Nebula')
    expect(renameSession(renamed, id, '').sessions[0]!.name).toBeNull()
  })
})

describe('initialWorkspace', () => {
  it('starts with one active session', () => {
    const workspace = initialWorkspace()
    expect(workspace.sessions).toHaveLength(1)
    expect(workspace.activeSessionId).toBe(workspace.sessions[0]!.id)
    expect(workspace.results).toEqual([])
  })
})
