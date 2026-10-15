import { describe, expect, it } from 'vitest'

import { locate } from './locate'
import { placeInfo } from './placement'
import { info } from './test-fixtures'
import { emptyWorkspace } from './workspace'

describe('locate', () => {
  it('finds a file in a session role and in results', () => {
    const withLight = placeInfo(emptyWorkspace(), info({ path: '/l/1' })).workspace
    const both = placeInfo(withLight, info({ path: '/r/1', stary_kind: 'stack_result' })).workspace
    expect(locate(both, '/l/1')).toMatchObject({ kind: 'session', role: 'light' })
    expect(locate(both, '/r/1')).toEqual({ kind: 'result' })
    expect(locate(both, '/missing')).toBeNull()
  })
})
