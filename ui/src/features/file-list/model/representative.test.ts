import { describe, expect, it } from 'vitest'

import { placeInfo } from './placement'
import { removePaths } from './removal'
import { isRepresentative, representativeOf, setRepresentative } from './representative'
import { info } from './test-fixtures'
import { type Workspace, emptyWorkspace } from './workspace'

function withLights(paths: string[]): Workspace {
  return paths.reduce(
    (workspace, path) => placeInfo(workspace, info({ path })).workspace,
    emptyWorkspace(),
  )
}

const first = (workspace: Workspace) => workspace.sessions[0]!

describe('representative image', () => {
  it('is the first light unless another one was picked', () => {
    const workspace = withLights(['/l/1.fits', '/l/2.fits', '/l/3.fits'])
    expect(representativeOf(first(workspace))?.path).toBe('/l/1.fits')
    expect(isRepresentative(first(workspace), '/l/1.fits')).toBe(true)
    expect(isRepresentative(first(workspace), '/l/2.fits')).toBe(false)
  })

  it('is nothing for a session without lights', () => {
    const workspace = placeInfo(emptyWorkspace(), info({ path: '/d/1.fits' }), {
      sessionId: placeInfo(emptyWorkspace(), info({ path: '/x.fits' })).workspace.sessions[0]!.id,
      role: 'dark',
    }).workspace
    expect(workspace.sessions.every((session) => representativeOf(session) === null)).toBe(true)
  })

  it('can be changed to any light of the session', () => {
    const workspace = withLights(['/l/1.fits', '/l/2.fits', '/l/3.fits'])
    const chosen = setRepresentative(workspace, first(workspace).id, '/l/3.fits')
    expect(representativeOf(first(chosen))?.path).toBe('/l/3.fits')
    expect(isRepresentative(first(chosen), '/l/1.fits')).toBe(false)
  })

  it('ignores a file that is not a light of the session', () => {
    const workspace = withLights(['/l/1.fits'])
    expect(setRepresentative(workspace, first(workspace).id, '/nope.fits')).toBe(workspace)
    expect(setRepresentative(workspace, 'no-such-session', '/l/1.fits')).toBe(workspace)
  })

  it('passes to the first remaining light when the representative is removed', () => {
    const workspace = withLights(['/l/1.fits', '/l/2.fits', '/l/3.fits'])
    const chosen = setRepresentative(workspace, first(workspace).id, '/l/2.fits')
    const removed = removePaths(chosen, ['/l/2.fits'])
    expect(first(removed).representativePath).toBeNull()
    expect(representativeOf(first(removed))?.path).toBe('/l/1.fits')
  })

  it('does not come back for a light that was removed and added again', () => {
    const workspace = withLights(['/l/1.fits', '/l/2.fits'])
    const chosen = setRepresentative(workspace, first(workspace).id, '/l/2.fits')
    const removed = removePaths(chosen, ['/l/2.fits'])
    const again = placeInfo(removed, info({ path: '/l/2.fits' })).workspace
    expect(representativeOf(first(again))?.path).toBe('/l/1.fits')
  })

  it('stays when other lights are removed', () => {
    const workspace = withLights(['/l/1.fits', '/l/2.fits', '/l/3.fits'])
    const chosen = setRepresentative(workspace, first(workspace).id, '/l/3.fits')
    const removed = removePaths(chosen, ['/l/1.fits'])
    expect(representativeOf(first(removed))?.path).toBe('/l/3.fits')
  })
})
