import { describe, expect, it } from 'vitest'

import { placeInfo } from './placement'
import {
  clickSelect,
  emptySelection,
  orderedPaths,
  pruneSelection,
  selectAll,
} from './selection'
import { info } from './test-fixtures'
import { type Workspace, emptyWorkspace } from './workspace'

const order = ['/a', '/b', '/c', '/d', '/e']
const plain = { toggle: false, range: false }

describe('clickSelect', () => {
  it('selects a single item on a plain click', () => {
    const next = clickSelect(emptySelection, order, '/b', plain)
    expect(next).toEqual({ paths: ['/b'], primary: '/b', anchor: '/b' })
  })

  it('adds and removes items with toggle', () => {
    const first = clickSelect(emptySelection, order, '/a', plain)
    const added = clickSelect(first, order, '/c', { toggle: true, range: false })
    expect(added.paths).toEqual(['/a', '/c'])
    expect(added.primary).toBe('/c')
    const removed = clickSelect(added, order, '/c', { toggle: true, range: false })
    expect(removed.paths).toEqual(['/a'])
    expect(removed.primary).toBe('/a')
  })

  it('selects a range from the anchor in either direction', () => {
    const anchor = clickSelect(emptySelection, order, '/b', plain)
    expect(clickSelect(anchor, order, '/d', { toggle: false, range: true }).paths).toEqual([
      '/b',
      '/c',
      '/d',
    ])
    expect(clickSelect(anchor, order, '/a', { toggle: false, range: true }).paths).toEqual([
      '/a',
      '/b',
    ])
  })

  it('keeps the anchor while extending a range', () => {
    const anchor = clickSelect(emptySelection, order, '/b', plain)
    const extended = clickSelect(anchor, order, '/d', { toggle: false, range: true })
    expect(extended.anchor).toBe('/b')
    expect(extended.primary).toBe('/d')
  })

  it('treats a range click without an anchor as a plain click', () => {
    const next = clickSelect(emptySelection, order, '/c', { toggle: false, range: true })
    expect(next.paths).toEqual(['/c'])
  })
})

describe('workspace selection helpers', () => {
  const workspace: Workspace = ['/l/1', '/l/2'].reduce(
    (current, path) => placeInfo(current, info({ path })).workspace,
    emptyWorkspace(),
  )

  it('lists paths in tree order', () => {
    expect(orderedPaths(workspace)).toEqual(['/l/1', '/l/2'])
  })

  it('selects everything', () => {
    expect(selectAll(workspace).paths).toEqual(['/l/1', '/l/2'])
  })

  it('prunes paths that no longer exist', () => {
    const selection = { paths: ['/l/1', '/gone'], primary: '/gone', anchor: '/gone' }
    expect(pruneSelection(selection, workspace)).toEqual({
      paths: ['/l/1'],
      primary: '/l/1',
      anchor: '/l/1',
    })
  })
})
